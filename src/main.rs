use ast::{ctx::AstCtx, item::ItemKind, span::FileId};
use parser::Parser;

fn main() {
    let source = "import std::Option\nimport std::Result";
    let mut ctx = AstCtx::new();
    let file = FileId(0);
    let mut parser = Parser::new(&mut ctx, source, file);
    let program = parser.parse_program().expect("bad program");
    for item in program {
        let item = ctx.items.get(item);
        match item.kind {
            ItemKind::Import(ref import_def) => {
                println!("import [{:?}]", item.span);
                for path in import_def.path.0.iter() {
                    println!("  - ident: {}", ctx.strings.lookup(path.name))
                }
            }
        }
    }
}
