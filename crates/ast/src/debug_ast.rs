use std::fmt::Write;

use crate::{
    ctx::AstCtx,
    expr::{ExprKind, LitKind},
    item::{ConstDef, ImportDef, ItemId},
    ty::TypeKind,
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

    fn visit_const(&mut self, item_id: ItemId, def: &ConstDef, ctx: &AstCtx) {
        let item = ctx.items.get(item_id);
        let _ = writeln!(
            &mut self.out,
            "const {} [{:?}]",
            ctx.strings.lookup(def.name.name),
            item.span
        );

        let ty = ctx.types.get(def.ty);
        match &ty.kind {
            TypeKind::Path(path) => {
                let joined = path
                    .0
                    .iter()
                    .map(|i| ctx.strings.lookup(i.name))
                    .collect::<Vec<_>>()
                    .join("::");
                let _ = writeln!(
                    &mut self.out,
                    "  type: Path {} [{:?}]",
                    joined, ty.span
                );
            }
        }

        let expr = ctx.exprs.get(def.value);
        let ExprKind::Literal(lit) = &expr.kind;
        match &lit.kind {
            LitKind::Bool(b) => {
                let _ = writeln!(
                    &mut self.out,
                    "  value: Bool({}) [{:?}]",
                    b, lit.span
                );
            }
            LitKind::Int => {
                let _ = writeln!(&mut self.out, "  value: Int [{:?}]", lit.span);
            }
            LitKind::Float => {
                let _ = writeln!(&mut self.out, "  value: Float [{:?}]", lit.span);
            }
            LitKind::Char => {
                let _ = writeln!(&mut self.out, "  value: Char [{:?}]", lit.span);
            }
            LitKind::Str => {
                let _ = writeln!(&mut self.out, "  value: Str [{:?}]", lit.span);
            }
        }
    }
}

pub fn format_program_ast(items: &[ItemId], ctx: &AstCtx) -> String {
    let mut visitor = DebugAstPrinterVisitor::new();
    walk_program(&mut visitor, items, ctx);
    visitor.into_output()
}
