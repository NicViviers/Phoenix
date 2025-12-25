use std::ops::Range;
use ariadne::{Label, Report, ReportKind, Source};

// Cannot display fancy errors here if we don't restrict which type is available here
#[cfg(target_os = "windows")]
const SLASH: char = '\\';
#[cfg(target_os = "linux")]
const SLASH: char = '/';

// Macro assumes that 'self' is in scope of 'InputLexer'
macro_rules! expect_char {
    ( $this:expr, $expected:expr, $span:expr $(, $hint:expr)? ) => {{
        if $this.cur_char != $expected {
            Report::build(ReportKind::Error, ("stdin", 0..0))
                .with_message("Invalid expression")
                .with_label(
                    Label::new(("stdin", $span))
                        .with_message(format!("Expected '{}' here", $expected))
                )
                // TODO: Figure out why this isn't working even though macro expansion is fine
                $(.with_note($hint))?
                .finish()
                .print(("stdin", Source::from(String::from_utf8($this.source.clone().into()).unwrap())))
                .unwrap();

            return None
        }

        $this.next_char();
    }}
}

pub struct InputLexer {
    source: Vec<u8>,
    cur_char: char,
    peek_char: char,
    index: usize,
}

impl InputLexer {
    pub fn new(source: String) -> Self {
        let mut source: Vec<u8> = source.as_bytes().into();
        #[cfg(target_os = "windows")]
        for _ in 0..2 { source.pop().unwrap(); }

        let cur_char = *source.get(0).unwrap_or(&0) as char;
        let peek_char = *source.get(1).unwrap_or(&0) as char;

        Self {
            source,
            cur_char,
            peek_char,
            index: 0
        }
    }

    fn next_char(&mut self) {
        self.index += 1;
        self.cur_char = *self.source.get(self.index).unwrap_or(&0) as char;
        self.peek_char = *self.source.get(self.index + 1).unwrap_or(&0) as char;
    }

    pub fn next_token(&mut self) -> Option<Token> {
        match self.cur_char {
            // Identifier
            c if c.is_alphabetic() && self.peek_char != ':' => {
                let start = self.index;

                while self.cur_char.is_alphanumeric() {
                    self.next_char();
                }

                let end = self.index;

                return Some(Token::new(
                    TokenType::Identifier,
                    String::from_utf8(self.source[start .. end].into()).unwrap(),
                    start .. end
                ));
            }

            // Number
            c if c.is_numeric() => {
                let start = self.index;

                while self.cur_char.is_numeric() {
                    self.next_char();
                }

                let end = self.index;

                return Some(Token::new(
                    TokenType::Number,
                    String::from_utf8(self.source[start .. end].into()).unwrap(),
                    start .. end
                ));
            }

            // Path
            c if InputLexer::path_cond(c, self.peek_char) => {
                match c {
                    // Relative path
                    '.' => {
                        if self.peek_char == SLASH {
                            // Relative forward path
                            let start = self.index;
                            self.next_char();
                            expect_char!(self, SLASH, self.index .. self.index + 1);

                            while self.cur_char.is_alphanumeric() || self.peek_char == SLASH {
                                self.next_char();
                            }

                            let end = self.index;

                            return Some(Token::new(
                                TokenType::Path,
                                String::from_utf8(self.source[start .. end].into()).unwrap(),
                                start .. end
                            ));
                        } else if self.peek_char == '.' {
                            // Relative backward path
                            let start = self.index;
                            self.next_char();
                            expect_char!(self, '.', self.index .. self.index + 1);
                            expect_char!(self, SLASH, self.index .. self.index + 1);

                            while self.cur_char.is_alphanumeric() || self.cur_char == SLASH {
                                self.next_char();
                            }

                            let end = self.index;

                            return Some(Token::new(
                                TokenType::Path,
                                String::from_utf8(self.source[start .. end].into()).unwrap(),
                                start .. end
                            ));
                        } else {
                            let error_offset = if self.source.len() == 1 { 1 } else { 2 };

                            Report::build(ReportKind::Error, ("stdin", 0..0))
                                .with_message("Unexpected end of path")
                                .with_label(
                                    Label::new(("stdin", self.index .. self.index + error_offset))
                                        .with_message(format!("Expected relative path such as '.{}' or '..{}'", SLASH, SLASH))
                                )
                                .with_note("Slashes are platform dependant")
                                .finish()
                                .print(("stdin", Source::from(String::from_utf8(self.source.clone().into()).unwrap())))
                                .unwrap();

                            return None;
                        }
                    }

                    c if (c.is_alphabetic() && self.peek_char == ':') || c == SLASH => {
                        // Full path
                        let start = self.index;
                        
                        #[cfg(target_os = "windows")]
                        {
                            expect_char!(self, ':', self.index .. self.index + 1);
                            expect_char!(self, SLASH, self.index .. self.index + 1, "Slashes are platform depdendant");
                        }

                        while self.cur_char.is_alphanumeric() || self.cur_char == SLASH {
                            self.next_char();
                        }

                        let end = self.index;

                        return Some(Token::new(
                            TokenType::Path,
                            String::from_utf8(self.source[start .. end].into()).unwrap(),
                            start .. end
                        ));
                    }

                    _ => unimplemented!()
                }

                return Some(Token::new(TokenType::And, String::new(), 0..1))
            }

            '\0' => Some(Token::new(TokenType::EOF, "\0".to_string(), 0..0)),
            _ => unreachable!("No matching token implementation found for this input")
        }
    }

    #[cfg(target_os = "windows")]
    #[inline(always)]
    fn path_cond(c: char, peek: char) -> bool {
        // Not checking c == '.' && ['/', '.'].contains(&peek) here
        // otherwise we can't generate a useful error
        c == '.' || (c.is_alphabetic() && peek == ':')
    }

    #[cfg(target_os = "linux")]
    #[inline(always)]
    fn path_cond(c: char, _: char) -> bool {
        // Not checking c == '.' && ['/', '.'].contains(&peek) here
        // otherwise we can't generate a useful error
        c == '.' || c == '/'
    }
}

#[derive(Debug)]
pub struct Token {
    typ: TokenType,
    text: String,
    range: Range<usize>
}

impl Token {
    pub fn new(typ: TokenType, text: String, range: Range<usize>) -> Self {
        Self {
            typ,
            text,
            range
        }
    }
}

#[derive(Debug)]
pub enum TokenType {
    // Text-values
    Identifier,
    Number,
    Path,

    // Operators
    Pipe, // '|'
    RedirIn, // '<'
    RedirOut, // '>'
    And, // '&'

    // Special types
    // Generally used for internal reference and not an actual value
    EOF
}