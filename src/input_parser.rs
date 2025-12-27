use crate::ast::*;
use super::{Token, TokenType, default_token};
use ariadne::{Report, ReportKind, Label, Source};

pub struct InputParser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    len: usize,
    index: usize
}

impl<'a> InputParser<'a> {
    pub fn new(source: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            source,
            len: tokens.len(),
            index: 0,
            tokens
        }
    }

    fn next_token(&mut self) -> Token {
        if self.index >= self.len {
            return default_token!(EOF);
        }

        let tok = self.tokens[self.index];
        self.index += 1;
        tok
    }

    fn expect_token(&mut self, typ: &[TokenType], note: Option<&'static str>) -> Option<Token> {
        let token = self.next_token();

        if !typ.contains(&token.typ) {
            let mut report = Report::build(ReportKind::Error, ("stdin", 0..0))
                .with_message("Invalid command")
                .with_label(
                    Label::new(("stdin", token.start .. token.end))
                        .with_message(format!("Expected {:?} token here", typ))
                );

            if let Some(note) = note {
                report = report.with_note(note);
            }

            report
                .finish()
                .print(("stdin", Source::from(self.source)))
                .unwrap();

            return None;
        }

        Some(token)
    }

    fn process_command(&mut self) -> Option<Spanned<Program>> {
        let tmp = self.next_token();
        if tmp.typ == TokenType::EOF {
            return None
        }
        self.index -= 1;

        let cmd = self.expect_token(
            &[TokenType::Path, TokenType::Identifier],
            Some("This was not recognized as an internal or external command")
        )?;

        let mut argv = Vec::new();
        let mut token = self.next_token();
        while ![TokenType::EOF, TokenType::And, TokenType::Pipe, TokenType::RedirIn, TokenType::RedirOut].contains(&token.typ) {
            argv.push(token.start .. token.end);
            token = self.next_token();
        }

        let mut stdin = StreamStrategy::Inherit;
        let mut stdout = StreamStrategy::Inherit;

        match token.typ {
            TokenType::Pipe => {
                stdin = StreamStrategy::Inherit;
                stdout = StreamStrategy::PipeToStdin
            }

            TokenType::RedirIn => {
                let file_handle = self.expect_token(
                    &[TokenType::Path],
                    Some("You must provide the path to a file to redirect to stdin")
                )?;
                stdin = StreamStrategy::PipeFromFile(file_handle.start .. file_handle.end);
                stdout = StreamStrategy::Inherit;
            }

            TokenType::RedirOut => {
                let file_handle = self.expect_token(
                    &[TokenType::Path],
                    Some("You must provide the path to a file to redirect stdout to")
                )?;
                stdin = StreamStrategy::Inherit;
                stdout = StreamStrategy::PipeToFile(file_handle.start .. file_handle.end)
            }

            TokenType::EOF | TokenType::And => {
                // Correct the span if there are no pipes or redirects (which would cause EOF with span of 0 .. 0)
                token.end = cmd.end;
            }

            _ => unreachable!()
        }

        Some(Spanned::new(Program::new(
            cmd.start .. cmd.end,
            argv,
            stdin,
            stdout
        ), cmd.start .. token.end))
    }

    pub fn build_ast(&mut self) -> Module {
        let mut stmts = Vec::new();

        while let Some(cmd) = self.process_command() {
            stmts.push(cmd);
        }

        Module { stmts }
    }
}