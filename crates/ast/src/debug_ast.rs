use std::fmt::Write;

use crate::{
    ctx::AstCtx,
    expr::{ExprId, ExprKind, LitKind},
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
        let item = ctx.get_item(item_id);
        let _ = writeln!(&mut self.out, "import [{:?}]", item.span);
        for path in &import.path.0 {
            let _ = writeln!(&mut self.out, "  - ident: {}", ctx.get_str(path.name));
        }
    }

    fn visit_const(&mut self, item_id: ItemId, def: &ConstDef, ctx: &AstCtx) {
        let item = ctx.get_item(item_id);
        let _ = writeln!(
            &mut self.out,
            "const {} [{:?}]",
            ctx.get_str(def.name.name),
            item.span
        );

        let ty = ctx.get_type(def.ty);
        match &ty.kind {
            TypeKind::Path(path) => {
                let joined = path
                    .0
                    .iter()
                    .map(|i| ctx.get_str(i.name))
                    .collect::<Vec<_>>()
                    .join("::");
                let _ = writeln!(&mut self.out, "  type: Path {} [{:?}]", joined, ty.span);
            }
        }

        self.write_expr("  value", def.value, ctx);
    }
}

impl DebugAstPrinterVisitor {
    fn write_expr(&mut self, label: &str, expr_id: ExprId, ctx: &AstCtx) {
        let expr = ctx.get_expr(expr_id);
        match &expr.kind {
            ExprKind::Literal(lit) => match &lit.kind {
                LitKind::Bool(b) => {
                    let _ = writeln!(&mut self.out, "{}: Bool({}) [{:?}]", label, b, lit.span);
                }
                LitKind::Int => {
                    let _ = writeln!(&mut self.out, "{}: Int [{:?}]", label, lit.span);
                }
                LitKind::Float => {
                    let _ = writeln!(&mut self.out, "{}: Float [{:?}]", label, lit.span);
                }
                LitKind::Char => {
                    let _ = writeln!(&mut self.out, "{}: Char [{:?}]", label, lit.span);
                }
                LitKind::Str => {
                    let _ = writeln!(&mut self.out, "{}: Str [{:?}]", label, lit.span);
                }
            },
            ExprKind::Path(ident) => {
                let _ = writeln!(
                    &mut self.out,
                    "{}: Path {} [{:?}]",
                    label,
                    ctx.get_str(ident.name),
                    expr.span
                );
            }
            ExprKind::Unary(unary) => {
                let _ = writeln!(
                    &mut self.out,
                    "{}: Unary {:?} [{:?}]",
                    label, unary.op, expr.span
                );
                self.write_expr("    expr", unary.expr, ctx);
            }
            ExprKind::Binary(binary) => {
                let _ = writeln!(
                    &mut self.out,
                    "{}: Binary {:?} [{:?}]",
                    label, binary.op, expr.span
                );
                self.write_expr("    lhs", binary.lhs, ctx);
                self.write_expr("    rhs", binary.rhs, ctx);
            }
            ExprKind::Call(call) => {
                let _ = writeln!(&mut self.out, "{}: Call [{:?}]", label, expr.span);
                self.write_expr("    callee", call.callee, ctx);
                for arg in &call.args {
                    self.write_expr("    arg", arg.value, ctx);
                }
            }
            ExprKind::Field(field) => {
                let _ = writeln!(
                    &mut self.out,
                    "{}: Field {} [{:?}]",
                    label,
                    ctx.get_str(field.field.name),
                    expr.span
                );
                self.write_expr("    base", field.base, ctx);
            }
        }
    }
}

pub fn format_program_ast(items: &[ItemId], ctx: &AstCtx) -> String {
    let mut visitor = DebugAstPrinterVisitor::new();
    walk_program(&mut visitor, items, ctx);
    visitor.into_output()
}
