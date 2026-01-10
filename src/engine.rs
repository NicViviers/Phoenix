use ariadne::{Label, Report, ReportKind, Source};
use crate::ast::{Module, Program, Spanned, StreamStrategy};
use std::{env, fs::File, io::{Error, ErrorKind}, path::PathBuf, process::{Command, Stdio}};
use std::collections::HashMap;


#[cfg(target_os = "windows")]
mod PLATFORM_VARS {
    pub const PATH_SEPARATOR: char = ';';
    pub const BASE_DIR: &'static str = "USERPROFILE";
}

#[cfg(target_os = "linux")]
mod PLATFORM_VARS {
    const PATH_SEPARATOR: char = ':';
    const BASE_DIR: &'static str = "HOME";
}

pub struct Engine {
    pub cur_dir: String, // TODO: Implement paths
    path: Vec<String>,
    vars: Vec<String>, // TODO: Implement environment variables. Load from Windows / bashrc ?
    builtins: HashMap<&'static str, builtins::BuiltinFn>,
    source: String
}

impl Engine {
    pub fn new() -> Self {
        let path = env::var_os("PATH")
            .unwrap()
            .to_str()
            .unwrap()
            .split(PLATFORM_VARS::PATH_SEPARATOR)
            .map(|p| p.to_string())
            .collect();

        Self {
            cur_dir: Engine::get_base_dir(),
            path,
            vars: Vec::new(),
            builtins: builtins::builtin_registry(),
            source: String::new()
        }
    }

    pub fn execute(&mut self, source: &str, module: Module) {
        self.source = source.to_string(); // Save the source to the instance for builtins to reference
        let mut iter = module.stmts.into_iter().peekable();

        while let Some(stmt) = iter.next() {
            let mut pipe_chain = vec![stmt];

            while let Some(_) = iter.peek() {
                if pipe_chain.last().unwrap().value.stdout == StreamStrategy::PipeToStdin {
                    pipe_chain.push(iter.next().unwrap());
                } else {
                    break;
                }
            }

            if pipe_chain.len() == 1 {
                // Single command, no piping
                self.execute_single(source, pipe_chain.pop().unwrap()).unwrap();
            } else {
                // We have a pipe chain so execute each statement individually and pipe stdio accordingly
                self.execute_pipeline(source, pipe_chain).unwrap();
            }
        }
    }

    fn execute_pipeline(&mut self, source: &str, chain: Vec<Spanned<Program>>) -> std::io::Result<()> {
        let mut children = Vec::new();
        let mut prev_stdout = None;

        for stmt in chain {
            if self.builtins.contains_key(&source[stmt.value.program.clone()]) {
                Report::build(ReportKind::Error, ("stdin", 0..0))
                    .with_message("Unsupported pipe operation")
                    .with_label(
                        Label::new(("stdin", stmt.span))
                            .with_message("Unable to pipe stdio between internal commands")
                    )
                    .finish()
                    .print(("stdin", Source::from(source)))
                    .unwrap();

                return Ok(())
            }

            let executable = self.find_executable(&source[stmt.value.program])?;
            let mut cmd = Command::new(executable);
            cmd.args(stmt.value.argv.iter().map(|arg| &source[arg.clone()]));

            let stdin = match prev_stdout.take() {
                Some(stdout) => Stdio::from(stdout),
                None => match stmt.value.stdin {
                    StreamStrategy::PipeFromFile(path) => {
                        let file = File::open(&source[path])?;
                        Stdio::from(file)
                    }

                    // First statement meaning we can guarantee it's inhering stdin if not from above file
                    _ => Stdio::inherit()
                }
            };

            cmd.stdin(stdin);

            let stdout = match stmt.value.stdout {
                StreamStrategy::PipeToStdin => Stdio::piped(),
                StreamStrategy::PipeToFile(ref path) => {
                    let file = File::create(&source[path.clone()])?;
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

    fn execute_single(&mut self, source: &str, stmt: Spanned<Program>) -> std::io::Result<()> {
        // Check if it is a built in command and execute before assuming it is an external command
        if let Some(builtin) = self.builtins.get(&source[stmt.value.program.clone()]) {
            return builtin(self, &stmt);
        }

        let executable = self.find_executable(&source[stmt.value.program])?;
        let mut cmd = Command::new(executable);
        cmd.args(stmt.value.argv.iter().map(|arg| &source[arg.clone()]));

        match stmt.value.stdin {
            StreamStrategy::PipeFromFile(path) => {
                let file = File::open(&source[path])?;
                cmd.stdin(Stdio::from(file));
            }

            _ => { cmd.stdin(Stdio::inherit()); }
        }

        match stmt.value.stdout {
            StreamStrategy::PipeToFile(path) => {
                let file = File::create(&source[path])?;
                cmd.stdout(Stdio::from(file));
            }

            _ => { cmd.stdout(Stdio::inherit()); }
        }

        // TODO: Implement program not found error
        let mut child = cmd.spawn()?;
        child.wait()?;

        Ok(())
    }

    fn find_executable(&self, cmd: &str) -> std::io::Result<PathBuf> {
        let extensions = if cfg!(windows) {
            vec!["exe", "cmd", "bat", "com"]
        } else {
            vec![""]
        };

        for dir in &self.path {
            for ext in &extensions {
                let mut full_path = PathBuf::from(dir).join(cmd);

                if !ext.is_empty() {
                    full_path.set_extension(ext);
                }

                if full_path.exists() && full_path.is_file() {
                    return Ok(full_path)
                }
            }
        }

        // TODO: Can we generate a ariadne error somehow?
        Err(Error::new(ErrorKind::NotFound, format!("Unrecognized command '{}'", cmd)))
    }

    fn get_base_dir() -> String {
        env::var_os(PLATFORM_VARS::BASE_DIR).unwrap().into_string().unwrap()
    }
}

// TODO: Finish implementing builtins module
mod builtins {
    use std::{collections::HashMap, env, io::{Read, Write}};
    use crate::{ast::{Program, Spanned}, engine::Engine};

    pub type BuiltinFn = fn(&mut crate::Engine, &Spanned<Program>) -> std::io::Result<()>;

    pub fn builtin_registry() -> HashMap<&'static str, BuiltinFn> {
        HashMap::from([
            ("cd", cd as BuiltinFn),
            ("ls", ls as BuiltinFn),
            ("echo", echo as BuiltinFn),
            ("clear", clear as BuiltinFn),
            ("exit", exit as BuiltinFn)
        ])
    }

    fn cd(engine: &mut crate::Engine, stmt: &Spanned<Program>) -> std::io::Result<()> {
        // TODO: Implement 'cd' command with no argv that should go back to home directory
        // TODO: Implement implicit relative paths such as 'C:\>cd Users' currently moves to 'Users\>' which doesn't exist
        // TODO: Lexer crashes with no token implementation of 'cd ..\'
        if let Some(path) = stmt.value.argv.get(0) {
            let path = str::from_utf8(&engine.source.as_bytes()[path.clone()]).unwrap();

            env::set_current_dir(path)?;
            engine.cur_dir = path.to_string();
        } else {
            let path = Engine::get_base_dir();
            env::set_current_dir(&path)?;
            engine.cur_dir = path;
        }

        Ok(())
    }

    fn ls(engine: &mut crate::Engine, stmt: &Spanned<Program>) -> std::io::Result<()> {
        std::fs::read_dir(engine.cur_dir.as_str()).unwrap().for_each(|entry| {
            println!("{}", entry.unwrap().file_name().display());
        });

        println!();

        Ok(())
    }

    fn echo(engine: &mut crate::Engine, stmt: &Spanned<Program>) -> std::io::Result<()> {
        if stmt.value.argv.len() > 0 {
            let content = &engine.source[stmt.value.argv[0].clone()];
            println!("{}", content);
        } else {
            // TODO: We don't support piping for internals
            // could we possibly change that to support piping *to* internals at least
            let mut buffer = Vec::new();
            std::io::stdin().read_to_end(&mut buffer)?;
            println!("{}", String::from_utf8(buffer).unwrap());
        }

        println!();

        Ok(())
    }

    fn clear(_: &mut crate::Engine, _: &Spanned<Program>) -> std::io::Result<()> {
        std::io::stdout().flush().unwrap();
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);

        Ok(())
    }

    fn exit(_: &mut crate::Engine, _: &Spanned<Program>) -> std::io::Result<()> {
        std::process::exit(0);
    }
}