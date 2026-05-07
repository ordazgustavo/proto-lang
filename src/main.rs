use ast::{ctx::AstCtx, span::FileId};
use parser::{Parser, format_program_ast};

fn main() {
    let source = r#"import std::Option
        import std::Result
        const PI: f32 = 3.14
        const NONE: std::Option = None"#;
    let mut ctx = AstCtx::new();
    let file = FileId(0);
    let mut parser = Parser::new(&mut ctx, source, file);
    let program = parser.parse_program().expect("bad program");
    print!("{}", format_program_ast(&program, &ctx));
}
