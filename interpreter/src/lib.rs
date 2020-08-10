use parser::{AstNode, Primitive, Program};
use std::io::{self, Read, Write};

#[derive(Debug)]
pub struct Interpreter {
    pub program: Program,
    pub mem: Vec<u8>,
    pub ptr: usize,
}

impl Interpreter {
    pub fn new(program: Program) -> Self {
        Interpreter {
            program,
            mem: Vec::new(),
            ptr: 0,
        }
    }

    pub fn set(&mut self, ptr: usize, v: u8) {
        *mem_mut(&mut self.mem, ptr) = v;
    }

    pub fn run<R: Read, W: Write>(&mut self, stdin: R, mut stdout: W) -> io::Result<()> {
        fn step<I: Iterator<Item = io::Result<u8>>, W: Write>(
            node: &AstNode,
            mem: &mut Vec<u8>,
            ptr: &mut usize,
            stdin: &mut I,
            stdout: &mut W,
        ) -> io::Result<()> {
            match node {
                AstNode::Primitive(p) => {
                    use Primitive::*;
                    match p {
                        PtrLeft => *ptr -= 1,
                        PtrRight => *ptr += 1,
                        Inc => *mem_mut(mem, *ptr) = mem_mut(mem, *ptr).wrapping_add(1),
                        Dec => *mem_mut(mem, *ptr) = mem_mut(mem, *ptr).wrapping_sub(1),
                        Write => {
                            stdout.write(&[*mem_mut(mem, *ptr)])?;
                        }
                        Read => {
                            *mem_mut(mem, *ptr) = stdin.next().ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "Couldn't read from input stream".to_string(),
                                )
                            })??;
                        }
                    }
                }
                AstNode::Loop(l) => {
                    while *mem_mut(mem, *ptr) != 0 {
                        for node in l.nodes() {
                            step(node, mem, ptr, stdin, stdout)?;
                        }
                    }
                }
            }
            Ok(())
        }
        let mut stdin = stdin.bytes();

        for node in self.program.nodes() {
            step(node, &mut self.mem, &mut self.ptr, &mut stdin, &mut stdout)?;
        }
        Ok(())
    }
}
fn mem_mut(mem: &mut Vec<u8>, ptr: usize) -> &mut u8 {
    if ptr >= mem.len() {
        mem.resize(ptr + 1, 0);
    }
    &mut mem[ptr]
}
