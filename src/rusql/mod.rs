extern crate rustyline;

mod pager;
mod parser;

pub mod repl {
    use super::pager::Table;
    use super::parser;

    pub fn run() {
        let filepath = "test.db";
        let mut rl = rustyline::Editor::<()>::new();
        let mut table = Table::db_open(filepath);

        loop {
            let readline = rl.readline("db > ");

            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    match line.as_str() {
                        ".exit" => {
                            table.db_close();
                            break;
                        }
                        _ => parser::parse_input(&mut table, line),
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
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
}
