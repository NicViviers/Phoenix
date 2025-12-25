use std::path::PathBuf;
use std::io::{stdin, stdout, Write};

mod input_lexer;
use input_lexer::*;

pub struct Terminal {
    path: PathBuf
}

fn main() {
    let mut lexer;
    let mut stdin_buffer;
    let mut stdout = stdout();
    let stdin = stdin();
    
    loop {
        stdin_buffer = String::new();
        print!(">");
        stdout.flush().expect("Unable to flush stdout!");
        stdin.read_line(&mut stdin_buffer).expect("Unable to read line from stdin!");

        lexer = InputLexer::new(stdin_buffer);
        println!("{:?}", lexer.next_token());
    }
}
