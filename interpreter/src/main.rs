use interpreter::Interpreter;
use parser::{parse_file, AstWalker};
use std::{
    io::{self, Read, Write},
    path::PathBuf,
};

struct Opts {
    path: PathBuf,
    verbose: bool,
}

impl Opts {
    fn from_args() -> Self {
        let mut verbose = false;
        let mut path = None;
        let mut args = std::env::args();
        let name = args.next().expect("First argument should always exist");
        for arg in args {
            match arg.as_ref() {
                "-v" | "--verbose" if !verbose => verbose = true,
                p if path.is_none() => path = Some(p.into()),
                _ => {
                    println!(
                        r"Usage:
    {} [-v|--verbose] [filename]

-v --verbose    If given, prints out the state of the interpreter after execution.
filename        The program to execute (default: main.bf).
",
                        name
                    );
                    std::process::exit(1);
                }
            }
        }
        Opts {
            verbose,
            path: path.unwrap_or_else(|| "main.bf".into()),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    let path = opts.path;

    let program = parse_file(path)?;
    let mut interpreter = Interpreter::new(
        std::io::stdin().bytes(),
        WriteExitBrokenPipe {
            inner: std::io::stdout(),
        },
    );
    interpreter.walk(&program)?;
    if opts.verbose {
        println!("ptr: {}\nmem:\n{:?}", interpreter.ptr, interpreter.mem);
    }

    Ok(())
}

struct WriteExitBrokenPipe<W> {
    inner: W,
}

impl<W: Write> Write for WriteExitBrokenPipe<W> {
    fn flush(&mut self) -> io::Result<()> {
        match self.inner.flush() {
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => std::process::exit(0),
            x => x,
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Err(err) if err.kind() == io::ErrorKind::BrokenPipe => std::process::exit(0),
            x => x,
        }
    }
}
