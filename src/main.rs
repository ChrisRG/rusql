use rustyline::error::ReadlineError;
use rustyline::Editor;

fn main() {
    repl();
}

pub fn repl() {
    let mut rl = Editor::<()>::new();

    loop {
        let readline = rl.readline("db > ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                match line.as_str() {
                    ".exit" => {
                        break;
                    }
                    _ => parse_input(line),
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }

        rl.save_history("history.txt").unwrap();
    }
}

fn parse_input(line: String) {
    if line.as_bytes()[0] == b'.' {
        println!("Meta command");
    } else {
        match prepare_statement(&line) {
            Some(stmt) => {
                execute_statement(stmt);
                println!("Executed.");
            }
            None => println!("Unrecognized command {}", line),
        }
    }
}

pub struct Stmt {
    stmt_type: StmtType,
}

pub enum StmtType {
    StmtInsert,
    StmtSelect,
}

use StmtType::*;

fn prepare_statement(line: &String) -> Option<Stmt> {
    match line.as_str() {
        "insert" => Some(Stmt {
            stmt_type: StmtInsert,
        }),
        "select" => Some(Stmt {
            stmt_type: StmtSelect,
        }),
        _ => None,
    }
}

fn execute_statement(stmt: Stmt) {
    match stmt.stmt_type {
        StmtInsert => {
            println!("Here we do an insert.");
        }
        StmtSelect => {
            println!("Here we do a select.");
        }
    }
}
