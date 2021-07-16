use bincode;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Range;
const TABLE_MAX_PAGES: usize = 100;
const PAGE_SIZE: usize = 4096;
const ROW_SIZE: usize = 291;
const ROWS_PER_PAGE: usize = PAGE_SIZE / ROW_SIZE; // 14
const TABLE_MAX_ROWS: usize = ROWS_PER_PAGE * TABLE_MAX_PAGES; // 1400

pub struct Stmt {
    pub stmt_type: StmtType,
}

pub enum StmtType {
    StmtInsert(Row),
    StmtSelect,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Row {
    pub id: u32,
    username: String,
    email: String,
}

impl Row {
    pub fn new(id: u32, username: String, email: String) -> Self {
        Self {
            id,
            username,
            email,
        }
    }

    pub fn into_bytes(&self) -> Option<Vec<u8>> {
        Some(bincode::serialize(&self).unwrap())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ExecError> {
        match bincode::deserialize::<Row>(&bytes) {
            Ok(row) => Ok(row),
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
        println!("Closing db");
        for i in 0..TABLE_MAX_PAGES {
            if self.pager.pages[i].is_none() {
                continue;
            }
            self.pager.flush(i).unwrap();
            self.pager.pages[i] = None;
        }
    }

    pub fn row_slot(&mut self, row_num: usize) -> Result<(usize, usize), ExecError> {
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
            let (page_num, offset) = self.row_slot(self.num_rows)?;
            let range = offset..offset + ROW_SIZE;
            if let Some(page) = &mut self.pager.pages[page_num] {
                page.write(range, row).unwrap();
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
        for row_num in 0..self.num_rows {
            let (page_num, offset) = self.row_slot(row_num).unwrap();
            if let Some(page) = &self.pager.pages[page_num] {
                let row_bytes = &page.bytes[offset..offset + ROW_SIZE];
                match Row::from_bytes(row_bytes) {
                    Ok(row) => rows.push(row),
                    Err(e) => return Err(e),
                }
            }
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
    // Change output to offset for page position
    fn load_page(&mut self, page_num: usize) -> Result<(), ExecError> {
        if page_num > TABLE_MAX_PAGES {
            return Err(ExecError {
                msg: format!(
                    "Tried to fetch page number out of bounds. {} > {}",
                    &page_num, TABLE_MAX_PAGES
                ),
            });
        }

        println!("Pages: {:?}", self.pages);
        if self.pages[page_num].is_none() {
            let mut page = vec![0; PAGE_SIZE];
            let mut num_pages = self.file_length / PAGE_SIZE;
            if self.file_length % PAGE_SIZE > 0 {
                num_pages += 1;
            }
            if page_num <= num_pages {
                let page_offset = (page_num * PAGE_SIZE) as u64;
                self.file_descriptor
                    .seek(SeekFrom::Start(page_offset))
                    .unwrap();
                self.file_descriptor.read(&mut page).unwrap();
            } else {
                return Err(ExecError {
                    msg: format!("Page number out of bounds."),
                });
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

        println!("Setting file position at {}", offset);
        self.file_descriptor.seek(SeekFrom::Start(offset)).unwrap();

        if let Some(page) = &self.pages[page_num] {
            println!("Writing: {:?}", page);
            let bytes = page.into_bytes();
            self.file_descriptor.write(&bytes).unwrap();
            let curr_pos = self.file_descriptor.seek(SeekFrom::Current(0));
            println!("Current position {:?}", curr_pos);
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

    pub fn write(&mut self, range: Range<usize>, row: Vec<u8>) -> Result<(), ExecError> {
        self.bytes.splice(range, row);
        Ok(())
    }

    pub fn into_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
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
