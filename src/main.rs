use std::ffi::CString;
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
    let mut lexer;
    let mut parser: InputParser;
    let mut module;

    let mut stdin_buffer;
    let mut stdin_bytes;
    let mut stdout = stdout();
    let stdin = stdin();
    
    // TODO: Empty command freezes / causes infinite loop
    loop {
        stdin_buffer = String::new();
        print!(">");
        stdout.flush().expect("Unable to flush stdout!");
        stdin.read_line(&mut stdin_buffer).expect("Unable to read line from stdin!");
        stdin_bytes = stdin_buffer.as_bytes().into();

        lexer = InputLexer::new(stdin_bytes);
        parser = InputParser::new(&*stdin_buffer, lexer.filter(|token| token.typ != TokenType::Whitespace).collect());
        module = parser.build_ast();
        engine.execute(module);
    }
}
