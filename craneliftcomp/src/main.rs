use cranelift::prelude::*;
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
            path: path.unwrap_or_else(|| "../main.bf".into()),
            out: out.unwrap_or_else(|| "main.o".into()),
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

fn compile(out: PathBuf, _program: Program) -> io::Result<()> {
    let obj_builder = cranelift_object::ObjectBuilder::new(
        codegen::isa::lookup(target_lexicon::Triple::host())
            .expect("A valid triple")
            .finish(settings::Flags::new(settings::builder())),
        String::from("main_object"),
        cranelift_module::default_libcall_names(),
    )
    .expect("obj_builder");

    let mut module = cranelift_module::Module::<cranelift_object::ObjectBackend>::new(obj_builder);
    let ptr_type = module.target_config().pointer_type();

    let mut sig = module.make_signature();
    sig.returns.push(AbiParam::new(types::I32));

    let mut puts_sig = module.make_signature();
    puts_sig.params.push(AbiParam::new(ptr_type));
    puts_sig.returns.push(AbiParam::new(types::I32));

    let string_id = module
        .declare_data("hello", cranelift_module::Linkage::Local, true, false, None)
        .expect("Declare string");

    let puts_id = module
        .declare_function("puts", cranelift_module::Linkage::Import, &puts_sig)
        .expect("Declare puts");
    let main_id = module
        .declare_function("main", cranelift_module::Linkage::Export, &sig)
        .expect("Declare main");

    let mut fn_builder_ctx = FunctionBuilderContext::new();
    let mut func =
        codegen::ir::function::Function::with_name_signature(ExternalName::user(0, 0), sig);

    let puts_ref = module.declare_func_in_func(puts_id, &mut func);
    let string_ref = module.declare_data_in_func(string_id, &mut func);
    let mut data_context = cranelift_module::DataContext::new();
    data_context.define(String::from("Hello World!\0").into_bytes().into_boxed_slice());
    module.define_data(string_id, &data_context).expect("Define string");

    {
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);
        let block = builder.create_block();
        builder.switch_to_block(block);

        let string_addr = builder.ins().global_value(ptr_type, string_ref);
        builder.ins().call(puts_ref, &[string_addr]);
        let ret = builder.ins().iconst(types::I32, Imm64::new(0));
        builder.ins().return_(&[ret]);

        builder.seal_block(block);
        builder.finalize();
    }
    println!("{}", func.display(None));
    let mut context = codegen::Context::for_function(func);
    let mut trap_sink = codegen::binemit::NullTrapSink {};
    module
        .define_function(main_id, &mut context, &mut trap_sink)
        .expect("Define main");

    module.finalize_definitions();
    let obj_code = module.finish().emit().expect("Emitting object code");

    let mut out_file = File::create(out)?;
    out_file.write_all(&obj_code)?;

    Ok(())
}

#[allow(dead_code)]
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
