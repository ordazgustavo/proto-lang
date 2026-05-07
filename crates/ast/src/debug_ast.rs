use std::fmt::Write;

use crate::{
    ctx::AstCtx,
    item::{ImportDef, ItemId},
    visit::{AstVisitor, walk_program},
};

pub struct DebugAstPrinterVisitor {
    out: String,
}

impl DebugAstPrinterVisitor {
    pub fn new() -> Self {
        Self { out: String::new() }
    }

    pub fn into_output(self) -> String {
        self.out
    }
}

impl AstVisitor for DebugAstPrinterVisitor {
    fn visit_import(&mut self, item_id: ItemId, import: &ImportDef, ctx: &AstCtx) {
        let item = ctx.items.get(item_id);
        let _ = writeln!(&mut self.out, "import [{:?}]", item.span);
        for path in &import.path.0 {
            let _ = writeln!(
                &mut self.out,
                "  - ident: {}",
                ctx.strings.lookup(path.name)
            );
        }
    }
}

pub fn format_program_ast(items: &[ItemId], ctx: &AstCtx) -> String {
    let mut visitor = DebugAstPrinterVisitor::new();
    walk_program(&mut visitor, items, ctx);
    visitor.into_output()
}
