use parser::{parse_file, AstNode, Primitive, Program};
use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
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
    compile(opts.out, program)?;
    Ok(())
}

fn compile(out: PathBuf, program: Program) -> io::Result<()> {
    let mut out = File::create(out)?;
    out.write(
        b"#include <stdio.h>

int main(void) {
    char mem[30000] = {0};
    char* ptr = mem;

",
    )?;

    for node in program.nodes() {
        compile_node(node, &mut out, 1)?;
    }

    out.write(b"}")?;

    Ok(())
}

fn compile_node(node: &AstNode, out: &mut File, depth: u32) -> io::Result<()> {
    let mut indentation = String::new();
    for _ in 0..depth {
        indentation.push_str("    ");
    }
    match node {
        AstNode::Primitive(p) => {
            use Primitive::*;
            writeln!(
                out,
                "{}{}",
                indentation,
                match p {
                    Dec => "--*ptr;",
                    Inc => "++*ptr;",
                    PtrRight => "++ptr;",
                    PtrLeft => "--ptr;",
                    Read => "if ((*ptr = getchar()) == -1) *ptr = 0;",
                    Write => "putchar(*ptr);",
                }
            )?
        }

        AstNode::Loop(l) => {
            writeln!(out, "{}while (*ptr) {{", indentation)?;
            for node in l.nodes() {
                compile_node(node, out, depth + 1)?;
            }
            writeln!(out, "{}}}", indentation)?;
        }
    }
    Ok(())
}
