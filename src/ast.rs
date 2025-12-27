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

#[derive(Debug, Clone)]
pub struct Module {
    pub stmts: Vec<Spanned<Program>>
}

#[derive(Debug, Clone)]
pub struct Program {
    pub program: Range<usize>,
    pub argv: Vec<Range<usize>>,
    pub stdin: StreamStrategy,
    pub stdout: StreamStrategy
    // We don't handle stderr in any special way
}

impl Program {
    pub fn new(
        program: Range<usize>,
        argv: Vec<Range<usize>>,
        stdin: StreamStrategy,
        stdout: StreamStrategy
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
pub enum StreamStrategy {
    Inherit, // Inherit from Phoenix
    PipeFromFile(Range<usize>), // Pipe file content to stdin
    PipeToFile(Range<usize>), // Pipe stdout to file
    PipeToStdin // Pipe stdout to stdin of next program
}