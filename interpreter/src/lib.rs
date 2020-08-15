use parser::{AstWalker, Primitive, Program, Sink};
use std::io;

#[derive(Debug)]
pub struct Interpreter<R, W> {
    pub mem: Vec<u8>,
    pub ptr: usize,
    pub source: R,
    pub sink: W,
}

impl<R, W> AstWalker for Interpreter<R, W>
where
    R: Iterator<Item = io::Result<u8>>,
    W: Sink<Err = io::Error>,
{
    type Err = io::Error;
    fn visit_prim(&mut self, prim: &Primitive) -> Result<(), Self::Err> {
        let ptr = &mut self.ptr;
        let mem = &mut self.mem;
        use Primitive::*;
        match prim {
            PtrLeft => *ptr -= 1,
            PtrRight => *ptr += 1,
            Inc => *mem_mut(mem, *ptr) = mem_mut(mem, *ptr).wrapping_add(1),
            Dec => *mem_mut(mem, *ptr) = mem_mut(mem, *ptr).wrapping_sub(1),
            Write => {
                self.sink.write(*mem_mut(mem, *ptr))?;
            }
            Read => {
                *mem_mut(mem, *ptr) = self.source.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Couldn't read from input stream".to_string(),
                    )
                })??;
            }
        }
        Ok(())
    }

    fn visit_loop(&mut self, lop: &Program) -> io::Result<()> {
        while *mem_mut(&mut self.mem, self.ptr) != 0 {
            self.walk(lop)?
        }
        Ok(())
    }
}
impl<R, W> Interpreter<R, W> {
    pub fn new(source: R, sink: W) -> Self {
        Interpreter {
            mem: Vec::new(),
            ptr: 0,
            source,
            sink,
        }
    }

    pub fn set(&mut self, ptr: usize, v: u8) {
        *mem_mut(&mut self.mem, ptr) = v;
    }
}

fn mem_mut(mem: &mut Vec<u8>, ptr: usize) -> &mut u8 {
    if ptr >= mem.len() {
        mem.resize(ptr + 1, 0);
    }
    &mut mem[ptr]
}
