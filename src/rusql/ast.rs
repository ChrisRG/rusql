use std::fmt;

pub struct Stmt {
    pub stmt_type: StmtType,
}

pub enum StmtType {
    StmtInsert(Row),
    StmtSelect,
}

#[derive(Debug)]
pub struct Row {
    id: u32,
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
