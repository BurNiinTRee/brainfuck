use anyhow::{Error, Result};
use cranelift::prelude::*;
use parser::{parse_file, AstWalker, Primitive, Program};
use std::{fs::File, io::Write, path::PathBuf};

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

fn compile(out: PathBuf, program: Program) -> Result<()> {
    let mut flags = settings::builder();
    flags.set("enable_probestack", "false")?;
    let obj_builder = cranelift_object::ObjectBuilder::new(
        codegen::isa::lookup(target_lexicon::Triple::host())?.finish(settings::Flags::new(flags)),
        String::from("main_object"),
        cranelift_module::default_libcall_names(),
    )?;

    let mut module = cranelift_module::Module::<cranelift_object::ObjectBackend>::new(obj_builder);

    let ptr_type = module.target_config().pointer_type();
    let mut context = FunctionBuilderContext::new();

    let mut sig = module.make_signature();
    sig.returns.push(AbiParam::new(types::I32));

    let mut main =
        codegen::ir::function::Function::with_name_signature(ExternalName::user(0, 0), sig);

    let mut putchar_sig = module.make_signature();
    putchar_sig.params.push(AbiParam::new(types::I32));

    let mut getchar_sig = module.make_signature();
    getchar_sig.returns.push(AbiParam::new(types::I32));

    let putchar_id = module
        .declare_function("putchar", cranelift_module::Linkage::Import, &putchar_sig)
        .expect("Declare putchar");
    let getchar_id = module
        .declare_function("getchar", cranelift_module::Linkage::Import, &getchar_sig)
        .expect("Declare getchar");
    let putchar = module.declare_func_in_func(putchar_id, &mut main);
    let getchar = module.declare_func_in_func(getchar_id, &mut main);

    let main_id =
        module.declare_function("main", cranelift_module::Linkage::Export, &main.signature)?;

    let mut main_builder = FunctionBuilder::new(&mut main, &mut context);
    let block = main_builder.create_block();
    main_builder.switch_to_block(block);

    let buffer_size = 30000;
    let mem = main_builder
        .create_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, buffer_size));

    let null = main_builder.ins().iconst(types::I8, 0);
    let size = main_builder.ins().iconst(ptr_type, buffer_size as i64);
    let mem_addr = main_builder.ins().stack_addr(ptr_type, mem, 0);

    let ptr = main_builder.create_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        ptr_type.bytes(),
    ));

    main_builder.call_memset(module.target_config(), mem_addr, null, size);

    main_builder.ins().stack_store(mem_addr, ptr, 0);

    let mut comp = CompileFunction {
        builder: &mut main_builder,
        ptr_type: module.target_config().pointer_type(),
        ptr,
        putchar,
        getchar,
    };

    comp.walk(&program)?;

    let ret = comp.builder.ins().iconst(types::I32, 0);

    comp.builder.ins().return_(&[ret]);
    let current_block = comp.builder.current_block().expect("At this point there should at least be the block created above");
    comp.builder.seal_block(current_block);
    comp.builder.finalize();

    println!("{}", main.display(None));

    let mut context = codegen::Context::for_function(main);
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

struct CompileFunction<'a> {
    builder: &'a mut FunctionBuilder<'a>,
    ptr: codegen::ir::entities::StackSlot,
    ptr_type: Type,
    putchar: codegen::ir::entities::FuncRef,
    getchar: codegen::ir::entities::FuncRef,
}

impl<'a> AstWalker for CompileFunction<'a> {
    type Err = Error;
    fn visit_prim(&mut self, prim: &Primitive) -> Result<()> {
        use Primitive::*;
        match prim {
            PtrRight => {
                let old_ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
                let new_ptr = self.builder.ins().iadd_imm(old_ptr, 1);
                self.builder.ins().stack_store(new_ptr, self.ptr, 0);
            }
            PtrLeft => {
                let old_ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
                let new_ptr = self.builder.ins().iadd_imm(old_ptr, -1);
                self.builder.ins().stack_store(new_ptr, self.ptr, 0);
            }
            Inc => {
                let ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
                let old_c = self.builder.ins().load(types::I8, MemFlags::new(), ptr, 0);
                let new_c = self.builder.ins().iadd_imm(old_c, 1);
                self.builder.ins().store(MemFlags::new(), new_c, ptr, 0);
            }
            Dec => {
                let ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
                let old_c = self.builder.ins().load(types::I8, MemFlags::new(), ptr, 0);
                let new_c = self.builder.ins().iadd_imm(old_c, -1);
                self.builder.ins().store(MemFlags::new(), new_c, ptr, 0);
            }
            Read => {
                let getc_ins = self.builder.ins().call(self.getchar, &[]);
                let new_c = self.builder.inst_results(getc_ins)[0];
                let ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
                self.builder.ins().store(MemFlags::new(), new_c, ptr, 0);
            }
            Write => {
                let ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
                let c = self
                    .builder
                    .ins()
                    .uload8(types::I32, MemFlags::new(), ptr, 0);
                let putc_ins = self.builder.ins().call(self.putchar, &[c]);
                self.builder.inst_results(putc_ins);
            }
        }
        Ok(())
    }

    fn visit_loop(&mut self, lop: &Program) -> Result<()> {
        let loop_header = self.builder.create_block();
        let prev_block = self
            .builder
            .current_block()
            .expect("We should always have at least one block setup by the `compile` function!");

        let loop_continuation = self.builder.create_block();

        let loop_body = self.builder.create_block();
        
        let jump = self.builder.ins().jump(loop_header, &[]);
        self.builder.inst_results(jump);
        self.builder.switch_to_block(loop_header);
        self.builder.seal_block(prev_block);

        let ptr = self.builder.ins().stack_load(self.ptr_type, self.ptr, 0);
        let c = self.builder.ins().load(types::I8, MemFlags::new(), ptr, 0);

        let true_branch = self.builder.ins().brnz(c, loop_body, &[]);
        let false_branch = self.builder.ins().jump(loop_continuation, &[]);


        self.builder.switch_to_block(loop_body);
        self.walk(lop)?;

        let end_body = self.builder.current_block().unwrap();
        let ret = self.builder.ins().jump(loop_header, &[]);
        self.builder.seal_block(end_body);
        self.builder.seal_block(loop_header);

        self.builder.switch_to_block(loop_continuation);


        Ok(())
    }
}
