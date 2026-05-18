use std::fmt::Write;

use crate::{
    ctx::AstCtx,
    expr::{ExprId, ExprKind, IfElseBranch, LitKind},
    item::{
        AssignTarget, ConstDef, ElseBranch, EnumDef, ExtendDef, FunctionDef, ImportDef, ItemId,
        Param, ParamLabel, Stmt, StmtKind, StructDef,
    },
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
            self.write_stmt("    stmt", stmt, ctx);
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

    fn visit_enum(&mut self, item_id: ItemId, def: &EnumDef, ctx: &AstCtx) {
        let item = ctx.get_item(item_id);
        let _ = writeln!(
            &mut self.out,
            "enum {} [{:?}]",
            ctx.get_str(def.name.name),
            item.span
        );
        for generic in &def.generic_params {
            let _ = writeln!(&mut self.out, "  generic: {}", ctx.get_str(generic.name));
        }
        for variant in &def.variants {
            let _ = write!(
                &mut self.out,
                "  variant {}",
                ctx.get_str(variant.name.name)
            );
            if !variant.payload.is_empty() {
                self.out.push('(');
                for (idx, ty) in variant.payload.iter().enumerate() {
                    if idx > 0 {
                        self.out.push_str(", ");
                    }
                    self.write_type_inline(*ty, ctx);
                }
                self.out.push(')');
            }
            self.out.push('\n');
        }
    }

    fn visit_extend(&mut self, item_id: ItemId, def: &ExtendDef, ctx: &AstCtx) {
        let item = ctx.get_item(item_id);
        let _ = writeln!(
            &mut self.out,
            "extend {} [{:?}]",
            ctx.get_str(def.target.name),
            item.span
        );
        for generic in &def.generic_params {
            let _ = writeln!(&mut self.out, "  generic: {}", ctx.get_str(generic.name));
        }
        for method in &def.methods {
            let _ = writeln!(&mut self.out, "  method {}", ctx.get_str(method.name.name));
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
            ExprKind::Path(path) => {
                let _ = writeln!(
                    &mut self.out,
                    "{}: Path {} [{:?}]",
                    label,
                    path.0
                        .iter()
                        .map(|ident| ctx.get_str(ident.name))
                        .collect::<Vec<_>>()
                        .join("::"),
                    expr.span
                );
            }
            ExprKind::ImplicitMember(member) => {
                let _ = writeln!(
                    &mut self.out,
                    "{}: ImplicitMember {} [{:?}]",
                    label,
                    ctx.get_str(member.name),
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
            ExprKind::Array(array) => {
                let _ = writeln!(&mut self.out, "{}: Array [{:?}]", label, expr.span);
                for element in &array.elements {
                    self.write_expr("    elem", *element, ctx);
                }
            }
            ExprKind::If(if_expr) => {
                let _ = writeln!(&mut self.out, "{}: If [{:?}]", label, expr.span);
                self.write_expr("    cond", if_expr.condition, ctx);
                for stmt in &if_expr.then_block.stmts {
                    self.write_stmt("    then", stmt, ctx);
                }
                match &if_expr.else_branch {
                    IfElseBranch::If(expr) => self.write_expr("    else", *expr, ctx),
                    IfElseBranch::Block(block) => {
                        for stmt in &block.stmts {
                            self.write_stmt("    else", stmt, ctx);
                        }
                    }
                }
            }
        }
    }

    fn write_stmt(&mut self, label: &str, stmt: &Stmt, ctx: &AstCtx) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.write_expr(label, *expr, ctx),
            StmtKind::Assignment(assign) => {
                let binding = match assign.binding {
                    Some(crate::item::BindingKind::Let) => "let ",
                    Some(crate::item::BindingKind::Var) => "var ",
                    None => "",
                };
                let _ = write!(&mut self.out, "{label}: Assign {binding}");
                match &assign.target {
                    AssignTarget::Ident(ident) => {
                        let _ = write!(&mut self.out, "{}", ctx.get_str(ident.name));
                    }
                    AssignTarget::Field { base, fields } => {
                        let _ = write!(&mut self.out, "{}", ctx.get_str(base.name));
                        for field in fields {
                            let _ = write!(&mut self.out, ".{}", ctx.get_str(field.name));
                        }
                    }
                }
                if let Some(ty) = assign.ty {
                    self.out.push_str(": ");
                    self.write_type_inline(ty, ctx);
                }
                let _ = writeln!(&mut self.out, " [{:?}]", stmt.span);
                self.write_expr("    value", assign.value, ctx);
            }
            StmtKind::If(if_stmt) => {
                let _ = writeln!(&mut self.out, "{label}: If [{:?}]", stmt.span);
                self.write_expr("    cond", if_stmt.condition, ctx);
                for stmt in &if_stmt.then_block.stmts {
                    self.write_stmt("    then", stmt, ctx);
                }
                match &if_stmt.else_branch {
                    Some(ElseBranch::If(else_if)) => {
                        let _ = writeln!(&mut self.out, "    else-if");
                        self.write_expr("      cond", else_if.condition, ctx);
                    }
                    Some(ElseBranch::Block(block)) => {
                        for stmt in &block.stmts {
                            self.write_stmt("    else", stmt, ctx);
                        }
                    }
                    None => {}
                }
            }
            StmtKind::ForIn(for_in) => {
                let _ = writeln!(
                    &mut self.out,
                    "{label}: For {} [{:?}]",
                    ctx.get_str(for_in.binding.name),
                    stmt.span
                );
                self.write_expr("    iter", for_in.iter, ctx);
                for stmt in &for_in.body.stmts {
                    self.write_stmt("    body", stmt, ctx);
                }
            }
        }
    }
}

pub fn format_program_ast(items: &[ItemId], ctx: &AstCtx) -> String {
    let mut visitor = DebugAstPrinterVisitor::new();
    walk_program(&mut visitor, items, ctx);
    visitor.into_output()
}
