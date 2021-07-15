use bincode;
use serde::{Deserialize, Serialize};
use std::fmt;

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
    pages: Vec<u8>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            num_rows: 0,
            pages: vec![0u8; TABLE_MAX_ROWS],
        }
    }

    pub fn row_slot_indices(&self, row_num: usize) -> (usize, usize) {
        let page_num = row_num / ROWS_PER_PAGE;
        let row_offset = row_num % ROWS_PER_PAGE;
        let byte_offset = row_offset * ROW_SIZE;
        let start = page_num + byte_offset;
        let end = start + ROW_SIZE;
        return (start, end);
    }

    pub fn insert(&mut self, row_to_insert: Row) -> Result<(), ExecError> {
        if self.num_rows >= TABLE_MAX_ROWS {
            return Err(ExecError {
                msg: String::from("Table full."),
            });
        }
        if let Some(row) = row_to_insert.into_bytes() {
            let (start, end) = self.row_slot_indices(self.num_rows);
            self.pages.splice(start..end, row);
            self.num_rows += 1;
        } else {
            return Err(ExecError {
                msg: String::from("Failed to serialize row."),
            });
        }
        Ok(())
    }

    pub fn select(&self) -> Result<Vec<Row>, ExecError> {
        let mut rows = Vec::new();
        for row_num in 0..self.num_rows {
            let start_idx = row_num * ROW_SIZE;
            let end_idx = start_idx + ROW_SIZE;
            let row_bytes = &self.pages[start_idx..end_idx];
            match Row::from_bytes(&row_bytes) {
                Ok(row) => rows.push(row),
                Err(e) => return Err(e),
            }
        }
        Ok(rows)
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
