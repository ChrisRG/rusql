use bincode;
use serde::{Deserialize, Serialize};
use std::env::current_dir;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
const TABLE_MAX_PAGES: usize = 3;
const PAGE_SIZE: usize = 528; // i.e. 4 rows per page; original 4096
const ROW_SIZE: usize = 132; // i.e. 1 byte flag + 3 byte id + 64 byte username + 64 byte email; Original 291
const ROWS_PER_PAGE: usize = PAGE_SIZE / ROW_SIZE; // 4 / original 14
const TABLE_MAX_ROWS: usize = ROWS_PER_PAGE * TABLE_MAX_PAGES; // 12 / original 1400

pub struct Cursor {
    num_rows: usize,
    row_num: usize,
    end_of_table: bool,
}

impl Cursor {
    pub fn table_start(num_rows: usize) -> Self {
        let end_of_table = num_rows == 0;
        Self {
            num_rows,
            row_num: 0,
            end_of_table,
        }
    }

    pub fn table_end(num_rows: usize) -> Self {
        let row_num = num_rows;
        Self {
            num_rows,
            row_num,
            end_of_table: true,
        }
    }

    pub fn advance(&mut self) {
        self.row_num += 1;
        if self.row_num >= self.num_rows {
            self.end_of_table = true;
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Row {
    flag: u8,
    id: u32,
    username: String,
    email: String,
}

impl Row {
    pub fn new(id: u32, username: String, email: String) -> Self {
        Self {
            flag: 1u8,
            id,
            username,
            email,
        }
    }

    pub fn into_bytes(&self) -> Option<Vec<u8>> {
        let mut serialized = bincode::serialize(&self).unwrap().to_vec();
        // We need to force each row to be ROW_SIZE
        // Resize fills the rest of the vec with 0s
        serialized.resize(ROW_SIZE, 0u8);
        Some(serialized)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Option<Self>, ExecError> {
        if bytes[0] == 0x00 {
            return Ok(None);
        }
        match bincode::deserialize::<Row>(&bytes) {
            Ok(row) => Ok(Some(row)),
            Err(_) => {
                return Err(ExecError {
                    msg: String::from("Unable to deserialize row"),
                })
            }
        }
    }
}

pub struct Table {
    num_rows: usize,
    pager: Pager,
}

impl Table {
    pub fn db_open(path: &str) -> Self {
        let pager = Pager::open_file(&path).unwrap();
        let num_rows = pager.file_length / ROW_SIZE;
        Self { pager, num_rows }
    }

    pub fn db_close(&mut self) {
        for i in 0..TABLE_MAX_PAGES {
            if self.pager.pages[i].is_none() {
                continue;
            }
            self.pager.flush(i).unwrap();
            self.pager.pages[i] = None;
        }
    }

    pub fn cursor_value(&mut self, cursor: &Cursor) -> Result<(usize, usize), ExecError> {
        let row_num = cursor.row_num;
        let page_num = row_num / ROWS_PER_PAGE;
        self.pager.load_page(page_num)?;
        let row_offset = row_num % ROWS_PER_PAGE;
        let byte_offset = row_offset * ROW_SIZE;
        return Ok((page_num, byte_offset));
    }

    pub fn insert(&mut self, row_to_insert: Row) -> Result<(), ExecError> {
        if self.num_rows >= TABLE_MAX_ROWS {
            return Err(ExecError {
                msg: String::from("Table full."),
            });
        }
        if let Some(row) = row_to_insert.into_bytes() {
            let cursor = Cursor::table_end(self.num_rows);
            let (page_num, offset) = self.cursor_value(&cursor)?;
            if let Some(page) = &mut self.pager.pages[page_num] {
                page.write(offset, row).unwrap();
                self.num_rows += 1;
            }
        } else {
            return Err(ExecError {
                msg: String::from("Failed to serialize row."),
            });
        }
        Ok(())
    }

    pub fn select(&mut self) -> Result<Vec<Row>, ExecError> {
        let mut rows = Vec::new();
        let mut cursor = Cursor::table_start(self.num_rows);
        while !cursor.end_of_table {
            let (page_num, offset) = self.cursor_value(&cursor).unwrap();
            if let Some(page) = &self.pager.pages[page_num] {
                let row_bytes = &page.bytes[offset..offset + ROW_SIZE];
                match Row::from_bytes(row_bytes) {
                    Ok(None) => {}
                    Ok(Some(row)) => rows.push(row),
                    Err(e) => return Err(e),
                }
            }
            cursor.advance();
        }
        Ok(rows)
    }
}

pub struct Pager {
    file_descriptor: File,
    file_length: usize,
    pages: Vec<Option<Page>>,
}

impl Pager {
    pub fn open_file(path: &str) -> Result<Self, ExecError> {
        if let Ok(mut fd) = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
        {
            let file_length = fd.seek(SeekFrom::End(0)).unwrap() as usize;
            let pages: Vec<Option<Page>> = vec![None; TABLE_MAX_PAGES];
            Ok(Self {
                file_descriptor: fd,
                file_length,
                pages,
            })
        } else {
            Err(ExecError {
                msg: String::from("Failed to read database file."),
            })
        }
    }

    fn load_page(&mut self, page_num: usize) -> Result<(), ExecError> {
        if page_num > TABLE_MAX_PAGES {
            return Err(ExecError {
                msg: format!(
                    "Tried to fetch page number out of bounds. {} > {}",
                    &page_num, TABLE_MAX_PAGES
                ),
            });
        }

        // If page_num isn't in cache, we allocate a new page
        if self.pages[page_num].is_none() {
            let mut page = vec![0; PAGE_SIZE];
            let mut num_pages = self.file_length / PAGE_SIZE;
            if self.file_length % PAGE_SIZE == 0 {
                num_pages += 1;
            }
            if page_num <= num_pages {
                let page_offset = (page_num * PAGE_SIZE) as u64;
                self.file_descriptor
                    .seek(SeekFrom::Start(page_offset))
                    .unwrap();
                self.file_descriptor.read(&mut page).unwrap();
            }
            self.pages[page_num] = Some(Page::from_bytes(page));
        }
        Ok(())
    }

    pub fn flush(&mut self, page_num: usize) -> Result<(), ExecError> {
        if self.pages[page_num].is_none() {
            return Err(ExecError {
                msg: format!("Tried to flush empty page."),
            });
        }

        let offset = (page_num * PAGE_SIZE) as u64;

        self.file_descriptor.seek(SeekFrom::Start(offset)).unwrap();

        if let Some(page) = &self.pages[page_num] {
            let bytes = page.into_bytes();
            self.file_descriptor.write(&bytes).unwrap();
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    bytes: Vec<u8>,
}

impl Page {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn write(&mut self, start: usize, row: Vec<u8>) -> Result<(), ExecError> {
        self.bytes.splice(start..start + ROW_SIZE, row);
        Ok(())
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}
pub struct Stmt {
    pub stmt_type: StmtType,
}

pub enum StmtType {
    StmtInsert(Row),
    StmtSelect,
}

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[derive(Debug)]
pub struct ExecError {
    pub msg: String,
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
