use std::{
    io,
    iter::Peekable,
};

#[derive(Debug)]
pub enum Primitive {
    PtrRight,
    PtrLeft,
    Inc,
    Dec,
    Write,
    Read,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Token {
    PtrRight,
    PtrLeft,
    Inc,
    Dec,
    Write,
    Read,
    StartLoop,
    EndLoop,
}

fn token_to_prim(t: Token) -> Option<Primitive> {
    use Token::*;
    match t {
        PtrRight => Some(Primitive::PtrRight),
        PtrLeft => Some(Primitive::PtrLeft),
        Inc => Some(Primitive::Inc),
        Dec => Some(Primitive::Dec),
        Write => Some(Primitive::Write),
        Read => Some(Primitive::Read),
        _ => None,
    }
}

#[derive(Debug)]
pub enum AstNode {
    Primitive(Primitive),
    Loop(Program),
}

#[derive(Debug)]
pub struct Program(Vec<AstNode>);

impl Program {
    pub fn nodes(&self) -> &[AstNode] {
        &self.0
    }
}

impl Program {
    fn try_from_iter<T>(iter: T) -> io::Result<Self>
    where
        T: IntoIterator<Item = u8>,
    {
        let mut iter = iter.into_iter().filter_map(parse_token).peekable();
        let mut program = Vec::new();
        while iter.peek().is_some() {
            program.push(parse(&mut iter).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Other,
                    "Your program didn't parse!\nIt probably doesn't have matching '[' and ']'."
                        .to_string(),
                )
            })?);
        }
        Ok(Program(program))
    }
}

fn parse<I: Iterator<Item = Token>>(iter: &mut Peekable<I>) -> Option<AstNode> {
    parse_prim(iter)
        .map(AstNode::Primitive)
        .or_else(|| parse_loop(iter))
}

fn parse_prim<I: Iterator<Item = Token>>(iter: &mut Peekable<I>) -> Option<Primitive> {
    iter.peek().copied().and_then(token_to_prim).map(|p| {
        iter.next();
        p
    })
}

fn parse_loop<I: Iterator<Item = Token>>(iter: &mut Peekable<I>) -> Option<AstNode> {
    iter.peek()
        .copied()
        .filter(|&t| t == Token::StartLoop)
        .map(|_start_loop| {
            iter.next();
            let mut children = Vec::new();
            while let Some(node) = parse(iter) {
                children.push(node);
            }
            Program(children)
        })
        .and_then(|l| {
            iter.next()
                .filter(|&e| e == Token::EndLoop)
                .map(|_| AstNode::Loop(l))
        })
}

pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> io::Result<Program> {
    use io::Read;
    let file = std::fs::File::open(path.as_ref())?;
    Program::try_from_iter(file.bytes().filter_map(|b| b.ok()))
}

fn parse_token(c: u8) -> Option<Token> {
    use Token::*;
    match c {
        b'>' => Some(PtrRight),
        b'<' => Some(PtrLeft),
        b'+' => Some(Inc),
        b'-' => Some(Dec),
        b'.' => Some(Write),
        b',' => Some(Read),
        b'[' => Some(StartLoop),
        b']' => Some(EndLoop),
        _ => None,
    }
}


pub trait AstWalker {
    type Err;
    fn visit_prim(&mut self, prim: &Primitive) -> Result<(), Self::Err>;
    fn visit_loop(&mut self, lop: &Program) -> Result<(), Self::Err>;
    fn walk(&mut self, program: &Program) -> Result<(), Self::Err> {
        for node in &program.0 {
            match node {
                AstNode::Primitive(prim) => self.visit_prim(&prim)?,
                AstNode::Loop(lop) => self.visit_loop(&lop)?,
            }
        }
        Ok(())
    }
}
