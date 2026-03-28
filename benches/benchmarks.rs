use criterion::{black_box, criterion_group, criterion_main, Criterion};
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;

const FIB_PROGRAM: &str = include_str!("programs/fib.txt");
const LOOP_PROGRAM: &str = include_str!("programs/loop.txt");
const FIB_AST_PROGRAM: &str = include_str!("programs/fib_ast.txt");
const ARRAY_OPS_PROGRAM: &str = include_str!("programs/array_ops.txt");
const STRING_CONCAT_PROGRAM: &str = include_str!("programs/string_concat.txt");
const JSON_OPS_PROGRAM: &str = include_str!("programs/json_ops.txt");
const GC_ALLOC_PROGRAM: &str = include_str!("programs/gc_alloc.txt");

/// A moderately complex program exercising functions, loops and conditionals
const COMPLEX_PROGRAM: &str = r#"
define → add → (a: int, b: int) → int
  return → a + b
end

define → mul → (a: int, b: int) → int
  return → a * b
end

store → sum → 0
store → i → 0
while → i < 100
  store → sum → add(sum, i)
  store → i → i + 1
end

if → sum > 0
  store → result → mul(sum, 2)
end

print → result
"#;

fn bench_lexer(c: &mut Criterion) {
    let source = FIB_PROGRAM.to_string();
    c.bench_function("lexer/fib", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(source.clone()));
            lexer.tokenize().expect("lex failed")
        })
    });

    let loop_src = LOOP_PROGRAM.to_string();
    c.bench_function("lexer/loop", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(loop_src.clone()));
            lexer.tokenize().expect("lex failed")
        })
    });
}

fn bench_parser(c: &mut Criterion) {
    let source = COMPLEX_PROGRAM.to_string();
    // Pre-lex once; bench only the parser
    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize().expect("lex failed");

    c.bench_function("parser/complex", |b| {
        b.iter(|| {
            let mut parser = Parser::new(black_box(tokens.clone()));
            parser.parse().expect("parse failed")
        })
    });
}

fn bench_ast_vm(c: &mut Criterion) {
    use txtcode::runtime::vm::VirtualMachine;

    // Pre-parse once; bench only VM execution
    let source = LOOP_PROGRAM.to_string();
    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse failed");

    c.bench_function("vm/ast_loop", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            black_box(vm.interpret(black_box(&program)).ok())
        })
    });
}

fn bench_compile(c: &mut Criterion) {
    use txtcode::compiler::bytecode::BytecodeCompiler;

    let source = COMPLEX_PROGRAM.to_string();
    c.bench_function("compile/lex+parse+bytecode", |b| {
        b.iter(|| {
            let mut lexer = Lexer::new(black_box(source.clone()));
            let tokens = lexer.tokenize().expect("lex failed");
            let mut parser = Parser::new(tokens);
            let program = parser.parse().expect("parse failed");
            let mut compiler = BytecodeCompiler::new();
            black_box(compiler.compile(&program))
        })
    });
}

fn bench_vm_bytecode(c: &mut Criterion) {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::runtime::bytecode_vm::BytecodeVM;

    let source = LOOP_PROGRAM.to_string();
    let mut lexer = Lexer::new(source.clone());
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse failed");
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);

    c.bench_function("vm/bytecode_loop", |b| {
        b.iter(|| {
            let mut vm = BytecodeVM::new();
            black_box(vm.execute(&bytecode).ok())
        })
    });
}

fn parse_program(source: &str) -> txtcode::parser::ast::Program {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().expect("lex failed");
    let mut parser = Parser::new(tokens);
    parser.parse().expect("parse failed")
}

fn bench_ast_fib(c: &mut Criterion) {
    use txtcode::runtime::vm::VirtualMachine;
    let program = parse_program(FIB_AST_PROGRAM);
    c.bench_function("vm/ast_fib20", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            black_box(vm.interpret(black_box(&program)).ok())
        })
    });
}

fn bench_ast_array_ops(c: &mut Criterion) {
    use txtcode::runtime::vm::VirtualMachine;
    let program = parse_program(ARRAY_OPS_PROGRAM);
    c.bench_function("vm/ast_array_ops", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            black_box(vm.interpret(black_box(&program)).ok())
        })
    });
}

fn bench_ast_string_concat(c: &mut Criterion) {
    use txtcode::runtime::vm::VirtualMachine;
    let program = parse_program(STRING_CONCAT_PROGRAM);
    c.bench_function("vm/ast_string_concat", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            black_box(vm.interpret(black_box(&program)).ok())
        })
    });
}

fn bench_ast_json_ops(c: &mut Criterion) {
    use txtcode::runtime::vm::VirtualMachine;
    let program = parse_program(JSON_OPS_PROGRAM);
    c.bench_function("vm/ast_json_ops", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            black_box(vm.interpret(black_box(&program)).ok())
        })
    });
}

fn bench_ast_gc_alloc(c: &mut Criterion) {
    use txtcode::runtime::vm::VirtualMachine;
    let program = parse_program(GC_ALLOC_PROGRAM);
    c.bench_function("vm/ast_gc_alloc_10k", |b| {
        b.iter(|| {
            let mut vm = VirtualMachine::new();
            black_box(vm.interpret(black_box(&program)).ok())
        })
    });
}

fn bench_bytecode_fib(c: &mut Criterion) {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    let program = parse_program(FIB_AST_PROGRAM);
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);
    c.bench_function("vm/bytecode_fib20", |b| {
        b.iter(|| {
            let mut vm = BytecodeVM::new();
            black_box(vm.execute(black_box(&bytecode)).ok())
        })
    });
}

fn bench_bytecode_array_ops(c: &mut Criterion) {
    use txtcode::compiler::bytecode::BytecodeCompiler;
    use txtcode::runtime::bytecode_vm::BytecodeVM;
    let program = parse_program(ARRAY_OPS_PROGRAM);
    let mut compiler = BytecodeCompiler::new();
    let bytecode = compiler.compile(&program);
    c.bench_function("vm/bytecode_array_ops", |b| {
        b.iter(|| {
            let mut vm = BytecodeVM::new();
            black_box(vm.execute(black_box(&bytecode)).ok())
        })
    });
}

criterion_group!(
    benches,
    bench_lexer,
    bench_parser,
    bench_ast_vm,
    bench_ast_fib,
    bench_ast_array_ops,
    bench_ast_string_concat,
    bench_ast_json_ops,
    bench_ast_gc_alloc,
    bench_compile,
    bench_vm_bytecode,
    bench_bytecode_fib,
    bench_bytecode_array_ops,
);

#[cfg(not(feature = "bytecode"))]
criterion_group!(
    benches,
    bench_lexer,
    bench_parser,
    bench_ast_vm,
    bench_ast_fib,
    bench_ast_array_ops,
    bench_ast_string_concat,
    bench_ast_json_ops,
    bench_ast_gc_alloc,
);

criterion_main!(benches);
