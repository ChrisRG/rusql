extern crate rustyline;

mod ast;
mod parser;

pub mod repl {
    use super::parser;

    pub fn run() {
        let mut rl = rustyline::Editor::<()>::new();

        loop {
            let readline = rl.readline("db > ");

            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    match line.as_str() {
                        ".exit" => {
                            break;
                        }
                        _ => parser::parse_input(line),
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
