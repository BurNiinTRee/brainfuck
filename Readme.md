# Brainfucks
This repository contains a bunch of brainfuck implementations in rust.

They are:

* a simple AST based interpreter. 
* a naïve brainfuck-to-c transpiler.
* a compiler using cranelift.


None of these implementations are particularily well opimized.

## Prerequisites
To run these programs one only needs:
* a working rust installation and
* a C-compiler/linker to turn the C-file from the transpiler and the
  object-file from the compiler into executables, as they both require
  libc to be linked.


## Building
```bash
$ pwd
/path/to/brainfucks
$ cargo build --release
```

## Usage
### Interpreter
```bash
cargo run --release -p interpreter -- <source.bf> [--verbose]
```

### Transpiler
```bash
cargo run --release -p comp2c -- <source.bf> [-o <main.c>] [--verbose]
gcc main.c -o main
./main
```

### Compiler
```bash
cargo run --release -p craneliftcomp -- <source.bf> [-o <main.o>] [--verbose]
gcc main.o -o main
./main
```
