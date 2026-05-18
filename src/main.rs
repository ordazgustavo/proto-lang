use ast::{ctx::AstCtx, span::FileId};
use hir::{format_hir_module, lower_program};
use parser::{Parser, format_program_ast};

fn main() {
    let source = include_str!("../assets/kitchen_sink.pr");
    // let source = r#"import std::Option
    //     import std::Result
    //     const PI: f32 = 3.14
    //     const NONE: std::Option = None
    //     fn add(a: int, b: int) -> int {a + b}
    //     struct Todo(title: str, description: str)
    //     enum Option<T> { some(T), none }"#;
    let mut ctx = AstCtx::new();
    let file = FileId(0);
    let mut parser = Parser::new(&mut ctx, source, file);
    let program = parser.parse_program().expect("bad program");
    let output = lower_program(&program, &ctx, file);
    println!("== AST ==");
    print!("{}", format_program_ast(&program, &ctx));
    println!("== HIR ==");
    print!("{}", format_hir_module(output.module, &output.hir, &ctx));
    if !output.diagnostics.is_empty() {
        println!("== HIR diagnostics ==");
        for diagnostic in output.diagnostics {
            println!("{:?}", diagnostic.kind);
        }
    }
}
