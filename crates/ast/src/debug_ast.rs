use std::fmt::Write;

use crate::{
    ctx::AstCtx,
    expr::{ExprId, ExprKind, LitKind},
    item::{ConstDef, FunctionDef, ImportDef, ItemId, Param, ParamLabel, StmtKind, StructDef},
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
            TypeKind::Path { path, generic_args } => {
                let joined = path
                    .0
                    .iter()
                    .map(|i| ctx.get_str(i.name))
                    .collect::<Vec<_>>()
                    .join("::");
                let _ = writeln!(&mut self.out, "  type: Path {} [{:?}]", joined, ty.span);
                for generic_arg in generic_args {
                    self.write_type("    generic", *generic_arg, ctx);
                }
            }
        }

        self.write_expr("  value", def.value, ctx);
    }

    fn visit_function(&mut self, item_id: ItemId, def: &FunctionDef, ctx: &AstCtx) {
        let item = ctx.get_item(item_id);
        let _ = writeln!(
            &mut self.out,
            "fn {} [{:?}]",
            ctx.get_str(def.name.name),
            item.span
        );
        for generic in &def.generic_params {
            let _ = writeln!(&mut self.out, "  generic: {}", ctx.get_str(generic.name));
        }
        for param in &def.params {
            match &param.label {
                ParamLabel::Implicit => {
                    let _ = write!(&mut self.out, "  param {}", ctx.get_str(param.name.name));
                }
                ParamLabel::Explicit(label) => {
                    let _ = write!(
                        &mut self.out,
                        "  param {} {}",
                        ctx.get_str(label.name),
                        ctx.get_str(param.name.name)
                    );
                }
                ParamLabel::Suppressed => {
                    let _ = write!(&mut self.out, "  param _ {}", ctx.get_str(param.name.name));
                }
            }
            self.out.push_str(": ");
            self.write_type_inline(param.ty, ctx);
            self.out.push('\n');
        }
        if let Some(return_type) = def.return_type {
            self.out.push_str("  return: ");
            self.write_type_inline(return_type, ctx);
            self.out.push('\n');
        }
        let _ = writeln!(&mut self.out, "  block [{:?}]", def.body.span);
        for stmt in &def.body.stmts {
            match stmt.kind {
                StmtKind::Expr(expr) => self.write_expr("    expr", expr, ctx),
            }
        }
    }

    fn visit_struct(&mut self, item_id: ItemId, def: &StructDef, ctx: &AstCtx) {
        let item = ctx.get_item(item_id);
        let _ = writeln!(
            &mut self.out,
            "struct {} [{:?}]",
            ctx.get_str(def.name.name),
            item.span
        );
        for generic in &def.generic_params {
            let _ = writeln!(&mut self.out, "  generic: {}", ctx.get_str(generic.name));
        }
        for field in &def.fields {
            self.write_param("  field", field, ctx);
        }
    }
}

impl DebugAstPrinterVisitor {
    fn write_param(&mut self, label: &str, param: &Param, ctx: &AstCtx) {
        match &param.label {
            ParamLabel::Implicit => {
                let _ = write!(&mut self.out, "{label} {}", ctx.get_str(param.name.name));
            }
            ParamLabel::Explicit(param_label) => {
                let _ = write!(
                    &mut self.out,
                    "{label} {} {}",
                    ctx.get_str(param_label.name),
                    ctx.get_str(param.name.name)
                );
            }
            ParamLabel::Suppressed => {
                let _ = write!(&mut self.out, "{label} _ {}", ctx.get_str(param.name.name));
            }
        }
        self.out.push_str(": ");
        self.write_type_inline(param.ty, ctx);
        self.out.push('\n');
    }

    fn write_type(&mut self, label: &str, ty_id: crate::ty::TypeId, ctx: &AstCtx) {
        self.out.push_str(label);
        self.out.push_str(": ");
        self.write_type_inline(ty_id, ctx);
        self.out.push('\n');
    }

    fn write_type_inline(&mut self, ty_id: crate::ty::TypeId, ctx: &AstCtx) {
        let ty = ctx.get_type(ty_id);
        match &ty.kind {
            TypeKind::Path { path, generic_args } => {
                for (idx, ident) in path.0.iter().enumerate() {
                    if idx > 0 {
                        self.out.push_str("::");
                    }
                    self.out.push_str(ctx.get_str(ident.name));
                }
                if !generic_args.is_empty() {
                    self.out.push('<');
                    for (idx, generic_arg) in generic_args.iter().enumerate() {
                        if idx > 0 {
                            self.out.push_str(", ");
                        }
                        self.write_type_inline(*generic_arg, ctx);
                    }
                    self.out.push('>');
                }
            }
        }
    }

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
