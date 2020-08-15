use parser::{parse_file, AstWalker, Primitive, Program};
use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

struct Opts {
    path: PathBuf,
    out: PathBuf,
    verbose: bool,
}

impl Opts {
    fn from_args() -> Self {
        let mut verbose = false;
        let mut path = None;
        let mut out = None;
        let mut args = std::env::args().peekable();
        let name = args.next().expect("First argument should always exist");
        while let Some(arg) = args.next() {
            match arg.as_ref() {
                "-v" | "--verbose" if !verbose => verbose = true,
                "-o" if out.is_none() && args.peek().is_some() => out = args.next().map(Into::into),
                p if path.is_none() => path = Some(p.into()),
                _ => {
                    println!(
                        r"Usage:
    {} [-v|--verbose] [filename] [-o out]

-v --verbose    If given, prints out the state of the interpreter after execution.
filename        The program to compile (default: main.bf).
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
            out: out.unwrap_or_else(|| "main.c".into()),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    let path = opts.path;

    let program = parse_file(path)?;
    if opts.verbose {
        println!("{:?}", program);
    }
    let mut compiler = Compiler::new(&opts.out)?;
    compiler.walk(&program)?;
    Ok(())
}

#[derive(Debug)]
struct Compiler {
    out: File,
    depth: u32,
}

impl Compiler {
    fn new(path: &Path) -> io::Result<Self> {
        let mut out = File::create(path)?;
        out.write_all(
            b"#include <stdio.h>

int main(void) {
    char mem[30000] = {0};
    char* ptr = mem;

",
        )?;
        Ok(Self { out, depth: 1 })
    }
}

impl Drop for Compiler {
    fn drop(&mut self) {
        self.out.write_all(b"}").expect("Couldn't finalize output!");
    }
}

impl AstWalker for Compiler {
    type Err = io::Error;
    fn visit_prim(&mut self, prim: &Primitive) -> io::Result<()> {
        let mut indentation = String::new();
        for _ in 0..self.depth {
            indentation.push_str("    ");
        }
        use Primitive::*;
        writeln!(
            self.out,
            "{}{}",
            indentation,
            match prim {
                Dec => "--*ptr;",
                Inc => "++*ptr;",
                PtrRight => "++ptr;",
                PtrLeft => "--ptr;",
                Read => "if ((*ptr = getchar()) == -1) *ptr = 0;",
                Write => "putchar(*ptr);",
            }
        )
    }
    fn visit_loop(&mut self, lop: &Program) -> io::Result<()> {
        let mut indentation = String::new();
        for _ in 0..self.depth {
            indentation.push_str("    ");
        }
        writeln!(self.out, "{}while (*ptr) {{", indentation)?;
        self.depth += 1;
        self.walk(lop)?;
        self.depth -= 1;
        writeln!(self.out, "{}}}", indentation)?;
        Ok(())
    }
}
