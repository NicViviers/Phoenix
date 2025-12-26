use std::ops::Range;
use ariadne::{Label, Report, ReportKind, Source};

// Cannot display fancy errors here if we don't restrict which type is available here
#[cfg(target_os = "windows")]
const SLASH: char = '\\';
#[cfg(target_os = "linux")]
const SLASH: char = '/';

// This as used as char exceptions for classifying identifiers
// Unfortunately OS-dependant since windows uses '/' and '?' inside program arguments
#[cfg(target_os = "windows")]
const IDENT_EXCEPT: [char; 4] = ['/', '?', '-', '.'];
#[cfg(target_os = "linux")]
const IDENT_EXCEPT: [char; 2] = ['-', '.'];

// Macro assumes that 'this' is in scope of 'InputLexer'
macro_rules! expect_char {
    ( $this:expr, $expected:expr, $span:expr $(, $hint:expr)? ) => {{
        if $this.cur_char != $expected {
            Report::build(ReportKind::Error, ("stdin", 0..0))
                .with_message("Invalid expression")
                .with_label(
                    Label::new(("stdin", $span))
                        .with_message(format!("Expected '{}' here", $expected))
                )
                $(.with_note($hint))?
                .finish()
                .print(("stdin", Source::from(String::from_utf8($this.source.clone().into()).unwrap())))
                .unwrap();

            return None
        }

        $this.next_char();
    }}
}

// Creates a default token of $var type with no text or span
#[macro_export]
macro_rules! default_token {
    ( $var:ident ) => {
        Token::new(TokenType::$var, 0 .. 0)
    }
}

pub struct InputLexer {
    source: Vec<u8>,
    cur_char: char,
    peek_char: char,
    index: usize,
}

impl InputLexer {
    pub fn new(mut source: Vec<u8>) -> Self {
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
        self.cur_char = *self.source.get(self.index).unwrap_or(&0x03u8) as char;
        self.peek_char = *self.source.get(self.index + 1).unwrap_or(&0x03u8) as char;
    }

    pub fn next_token(&mut self) -> Option<Token> {
        match self.cur_char {
            // Identifier
            // Accepts IDENT_EXCEPT characters for purposes of file extensions and argv
            c if (c.is_alphabetic() && self.peek_char != ':') || IDENT_EXCEPT.contains(&c) => {
                let start = self.index;

                while self.cur_char.is_alphanumeric() || IDENT_EXCEPT.contains(&self.cur_char) {
                    self.next_char();
                }

                let end = self.index;

                // Check if there is a file extension
                if str::from_utf8(&self.source[start .. end]).unwrap().contains('.') {
                    return Some(Token::new(
                        TokenType::Path,
                        start .. end
                    ))
                }

                return Some(Token::new(
                    TokenType::Identifier,
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
                                start .. end
                            ));
                        } else if self.peek_char == '.' {
                            // Relative backward path
                            let start = self.index;
                            self.next_char();
                            expect_char!(self, '.', self.index .. self.index + 1);
                            expect_char!(self, SLASH, self.index .. self.index + 1, "Slashes are platform depdendant");

                            while self.cur_char.is_alphanumeric() || self.cur_char == SLASH {
                                self.next_char();
                            }

                            let end = self.index;

                            return Some(Token::new(
                                TokenType::Path,
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
                        self.next_char();
                        
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
                            start .. end
                        ));
                    }

                    _ => unimplemented!()
                }
            }

            // String
            '"' | '\'' => {
                let quote_char = self.cur_char;
                let start = self.index;
                self.next_char();

                let mut closed = false;

                while self.index < self.source.len() {
                    match self.cur_char {
                        '\\' => {
                            self.next_char();
                            self.next_char();
                        }

                        c if c == quote_char => {
                            self.next_char();
                            closed = true;
                            break;
                        }

                        _ => self.next_char()
                    }
                }

                if !closed {
                    Report::build(ReportKind::Error, ("stdin", 0..0))
                        .with_message("Unexpected termination of string")
                        .with_label(
                            Label::new(("stdin", start .. self.index - 1))
                                .with_message(format!("This string should be terminated with {}", quote_char))
                        )
                        .with_note("Keep string delimiters should be consistent")
                        .finish()
                        .print(("stdin", Source::from(String::from_utf8(self.source.clone().into()).unwrap())))
                        .unwrap();

                    return None;
                }

                Some(Token::new(TokenType::String, start .. self.index))
            }

            // Pipe
            '|' => {
                self.next_char();
                Some(Token::new(TokenType::Pipe, self.index - 1 .. self.index))
            }

            // RedirIn
            '<' => {
                self.next_char();
                Some(Token::new(TokenType::RedirIn, self.index - 1 .. self.index))
            }

            // RedirOut
            '>' => {
                self.next_char();
                Some(Token::new(TokenType::RedirOut, self.index - 1 .. self.index))
            }

            // And
            '&' => {
                self.next_char();
                Some(Token::new(TokenType::And, self.index - 1 .. self.index))
            }

            c if c.is_whitespace() => {
                self.next_char();
                Some(default_token!(Whitespace))
            }

            '\0' => Some(default_token!(EOF)),
            '\x03' => None, // This represents 0x03 END OF TEXT byte to stop any iterators
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

impl Iterator for InputLexer {
    type Item = Token;
    
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Token {
    pub typ: TokenType,
    pub start: usize,
    pub end: usize
}

impl Token {
    pub fn new(typ: TokenType, range: Range<usize>) -> Self {
        Self {
            typ,
            start: range.start,
            end: range.end
        }
    }
}

// TODO: Can I implement sub-commands here like 'echo ${cat /home/nicholas/test.txt}'?
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    // Text-values
    Identifier,
    Number,
    Path,
    String,

    // Operators
    Pipe, // '|' - pipes stdout to stdin of following program
    RedirIn, // '<' - pipes file to stdin of program
    RedirOut, // '>' - pipes stdout to file
    And, // '&'

    // Special types
    // Generally used for internal reference and not an actual value
    Whitespace,
    EOF
}