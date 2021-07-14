use super::ast::{ParseError, Row, Stmt, StmtType};

pub fn parse_input(line: String) {
    if line.as_bytes()[0] == b'.' {
        println!("Meta command");
    } else {
        // Check if there are args following command to pass into parser
        let split = match line.find(' ') {
            Some(idx) => idx,
            None => line.len(),
        };
        let (cmd, arg_str) = line.split_at(split);
        match prepare_statement(cmd.to_string(), arg_str.to_string()) {
            Ok(stmt) => {
                execute_statement(stmt);
                println!("Executed.")
            }
            Err(e) => println!("[Error] {}", e),
        }
    }
}

// Parses the input
pub fn prepare_statement(cmd: String, arg_str: String) -> Result<Stmt, ParseError> {
    match cmd.as_str() {
        "insert" => match prepare_insert(arg_str) {
            Ok(row) => {
                return Ok(Stmt {
                    stmt_type: StmtType::StmtInsert(row),
                })
            }
            Err(e) => return Err(e),
        },
        "select" => Ok(Stmt {
            stmt_type: StmtType::StmtSelect,
        }),
        _ => Err(ParseError {
            msg: String::from("Unrecognized command."),
        }),
    }
}

pub fn prepare_insert(arg_str: String) -> Result<Row, ParseError> {
    let arg_vec = arg_str.split_whitespace().take(3).collect::<Vec<&str>>();

    if let [id, username, email] = arg_vec[..] {
        println!("Insert {} {} {}", id, username, email);
        match id.parse::<u32>() {
            Ok(id_num) => return Ok(Row::new(id_num, username.to_string(), email.to_string())),
            Err(_) => {
                return Err(ParseError {
                    msg: String::from("First argument must be an id number."),
                })
            }
        }
    } else {
        return Err(ParseError {
            msg: String::from("Insert syntax: `insert <id> <name> <email>`"),
        });
    }
}

// Executes the command
pub fn execute_statement(stmt: Stmt) {
    match stmt.stmt_type {
        StmtType::StmtInsert(_) => {
            println!("Here we do an insert.");
        }
        StmtType::StmtSelect => {
            println!("Here we do a select.");
        }
    }
}
// }
