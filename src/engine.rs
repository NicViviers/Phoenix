use crate::ast::{Module, Program, Spanned, StreamStrategy};
use std::{fs::File, io::pipe, process::{Command, Stdio}};

pub struct Engine {
    cur_dir: String, // TODO: Implement paths
    vars: Vec<String> // TODO: Implement environment variables. Load from Windows / bashrc ?
}

impl<'a> Engine {
    pub fn new() -> Self {
        Self {
            cur_dir: String::new(),
            vars: Vec::new()
        }
    }

    pub fn execute(&mut self, mut module: Module<'a>) {
        let mut iter = module.stmts.into_iter().peekable();

        while let Some(stmt) = iter.next() {
            let mut pipe_chain = vec![stmt];

            while let Some(next_stmt) = iter.peek() {
                if pipe_chain.last().unwrap().value.stdout == StreamStrategy::PipeToStdin {
                    pipe_chain.push(iter.next().unwrap());
                } else {
                    break;
                }
            }

            if pipe_chain.len() == 1 {
                // Single command, no piping
                self.execute_single(pipe_chain.pop().unwrap()).unwrap();
            } else {
                // We have a pipe chain so execute each statement individually and pipe stdio accordingly
                self.execute_pipeline(pipe_chain).unwrap();
            }
        }
    }

    fn execute_pipeline(&mut self, chain: Vec<Spanned<Program<'a>>>) -> std::io::Result<()> {
        let mut children = Vec::new();
        let mut prev_stdout = None;

        for stmt in chain {
            let mut cmd = Command::new(stmt.value.program);
            cmd.args(&stmt.value.argv);

            let stdin = match prev_stdout.take() {
                Some(stdout) => Stdio::from(stdout),
                None => match stmt.value.stdin {
                    StreamStrategy::PipeFromFile(path) => {
                        let file = File::open(path)?;
                        Stdio::from(file)
                    }

                    // First statement meaning we can guarantee it's inhering stdin if not from above file
                    _ => Stdio::inherit()
                }
            };

            cmd.stdin(stdin);

            let stdout = match stmt.value.stdout {
                StreamStrategy::PipeToStdin => Stdio::piped(),
                StreamStrategy::PipeToFile(path) => {
                    let file = File::create(path)?;
                    Stdio::from(file)
                }

                // Default to inheriting if not piping to next statement or to a file
                _ => Stdio::inherit()
            };

            cmd.stdout(stdout);

            let mut child = cmd.spawn()?;

            if stmt.value.stdout == StreamStrategy::PipeToStdin {
                prev_stdout = Some(child.stdout.take().unwrap());
            }

            children.push(child);
        }

        for mut child in children {
            child.wait()?;
        }

        Ok(())
    }

    fn execute_single(&mut self, stmt: Spanned<Program<'a>>) -> std::io::Result<()> {
        let mut cmd = Command::new(stmt.value.program);
        cmd.args(&stmt.value.argv);

        match stmt.value.stdin {
            StreamStrategy::PipeFromFile(path) => {
                let file = File::open(path)?;
                cmd.stdin(Stdio::from(file));
            }

            _ => { cmd.stdin(Stdio::inherit()); }
        }

        match stmt.value.stdout {
            StreamStrategy::PipeToFile(path) => {
                let file = File::create(path)?;
                cmd.stdout(Stdio::from(file));
            }

            _ => { cmd.stdout(Stdio::inherit()); }
        }

        let mut child = cmd.spawn()?;
        child.wait()?;

        Ok(())
    }
}

// TODO: Finish implementing builtins module
mod builtins {
    use std::collections::HashMap;

    type BuiltinFn = fn();

    fn builtin_registry() -> HashMap<&'static str, BuiltinFn> {
        HashMap::from([
            ("cd", cd as BuiltinFn),
            ("exit", exit as BuiltinFn)
        ])
    }

    fn cd() {}
    fn exit() {}
}