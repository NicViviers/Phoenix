use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Spanned<T: Clone> {
    pub value: T,
    pub span: Range<usize>
}

impl<T: Clone> Spanned<T> {
    #[inline(always)]
    pub fn new(value: T, span: Range<usize>) -> Self {
        Self {
            value,
            span
        }
    }
}

#[derive(Debug)]
pub struct Module<'a> {
    pub stmts: Vec<Spanned<Program<'a>>>
}

#[derive(Debug, Clone)]
pub struct Program<'a> {
    pub program: &'a str,
    pub argv: Vec<&'a str>,
    pub stdin: StreamStrategy<'a>,
    pub stdout: StreamStrategy<'a>
    // We don't handle stderr in any special way
}

impl<'a> Program<'a> {
    pub fn new(
        program: &'a str,
        argv: Vec<&'a str>,
        stdin: StreamStrategy<'a>,
        stdout: StreamStrategy<'a>
    ) -> Self {
        Self {
            program,
            argv,
            stdin,
            stdout
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamStrategy<'a> {
    Inherit, // Inherit from Phoenix
    PipeFromFile(&'a str), // Pipe file content to stdin
    PipeToFile(&'a str), // Pipe stdout to file
    PipeToStdin // Pipe stdout to stdin of next program
}