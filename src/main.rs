use std::io::{stdin, stdout, Write};

mod input_lexer;
mod input_parser;
mod ast;
mod engine;

use input_lexer::*;
use input_parser::*;
use engine::*;

fn main() {
    let mut engine = Engine::new();
    let mut stdin_buffer;
    let mut stdout = stdout();
    let stdin = stdin();
    
    // TODO: Empty command freezes / causes infinite loop
    loop {
        stdin_buffer = String::new();
        print!("{}>", engine.cur_dir);
        stdout.flush().expect("Unable to flush stdout!");

        stdin.read_line(&mut stdin_buffer)
            .expect("Unable to read line from stdin!");

        let stdin_bytes = stdin_buffer.as_bytes().into();

        let lexer = InputLexer::new(stdin_bytes);

        let tokens = lexer
            .filter(|token| token.typ != TokenType::Whitespace)
            .collect();

        let mut parser = InputParser::new(&*stdin_buffer, tokens);

        let module = parser.build_ast();

        engine.execute(stdin_buffer.as_str(), module);
    }
}
