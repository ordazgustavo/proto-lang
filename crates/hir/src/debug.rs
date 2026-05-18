use std::fmt::Write;

use ast::{
    common::{Ident, ModPath},
    ctx::AstCtx,
};

use crate::{ctx::HirCtx, expr::*, ids::*, item::*, ty::TypeKind};

pub fn format_hir_module(module: ModuleId, hir: &HirCtx, ast: &AstCtx) -> String {
    let mut out = String::new();
    let module = hir.module(module);
    let _ = writeln!(out, "module {:?}", module.file);
    let _ = writeln!(
        out,
        "namespaces value={} type={}",
        module.value_items.len(),
        module.type_items.len()
    );
    for import in &module.imports {
        let import = hir.import(*import);
        let _ = writeln!(out, "import {}", fmt_path(&import.path, ast));
    }
    for &item in &module.items {
        write_item(&mut out, item, hir, ast, "");
    }
    out
}

fn write_item(out: &mut String, id: ItemId, hir: &HirCtx, ast: &AstCtx, indent: &str) {
    match &hir.item(id).kind {
        ItemKind::Const(def) => {
            let _ = writeln!(
                out,
                "{indent}const {}: {}",
                name(def.name, ast),
                fmt_type(def.ty, hir, ast)
            );
            write_expr(out, "  value", def.value, hir, ast);
        }
        ItemKind::Function(def) => {
            let _ = writeln!(out, "{indent}fn {}", name(def.name, ast));
            for generic in &def.generic_params {
                let _ = writeln!(
                    out,
                    "  generic {}",
                    name(hir.generic_param(*generic).name, ast)
                );
            }
            for param in &def.params {
                write_param(out, "  param", *param, hir, ast);
            }
            if let Some(return_type) = def.return_type {
                let _ = writeln!(out, "  return {}", fmt_type(return_type, hir, ast));
            }
            write_body(out, "  body", def.body, hir, ast);
        }
        ItemKind::Method(def) => {
            let _ = writeln!(
                out,
                "{indent}method {}.{}",
                name(def.target, ast),
                name(def.name, ast)
            );
            for param in &def.params {
                write_param(out, "  param", *param, hir, ast);
            }
            write_body(out, "  body", def.body, hir, ast);
        }
        ItemKind::Struct(def) => {
            let _ = writeln!(out, "{indent}struct {}", name(def.name, ast));
            for generic in &def.generic_params {
                let _ = writeln!(
                    out,
                    "  generic {}",
                    name(hir.generic_param(*generic).name, ast)
                );
            }
            for field in &def.fields {
                let field = hir.field(*field);
                let _ = writeln!(
                    out,
                    "  field {}: {}",
                    name(field.name, ast),
                    fmt_type(field.ty, hir, ast)
                );
            }
        }
        ItemKind::Enum(def) => {
            let _ = writeln!(out, "{indent}enum {}", name(def.name, ast));
            for generic in &def.generic_params {
                let _ = writeln!(
                    out,
                    "  generic {}",
                    name(hir.generic_param(*generic).name, ast)
                );
            }
            for variant in &def.variants {
                let variant = hir.variant(*variant);
                let payload = variant
                    .payload
                    .iter()
                    .map(|ty| fmt_type(*ty, hir, ast))
                    .collect::<Vec<_>>()
                    .join(", ");
                let _ = writeln!(out, "  variant {}({})", name(variant.name, ast), payload);
            }
        }
        ItemKind::Extend(def) => {
            let _ = writeln!(out, "{indent}extend {}", name(def.target, ast));
            for method in &def.methods {
                write_item(out, *method, hir, ast, "  ");
            }
        }
    }
}

fn write_param(out: &mut String, label: &str, id: ParamId, hir: &HirCtx, ast: &AstCtx) {
    let param = hir.param(id);
    let _ = writeln!(
        out,
        "{label} {}: {}",
        name(param.name, ast),
        fmt_type(param.ty, hir, ast)
    );
}

fn write_body(out: &mut String, label: &str, id: BodyId, hir: &HirCtx, ast: &AstCtx) {
    let body = hir.body(id);
    let scope = hir.scope(body.scope);
    let _ = writeln!(
        out,
        "{label} scope params={} locals={}",
        scope.params.len(),
        scope.locals.len()
    );
    for stmt in &body.stmts {
        write_stmt(out, "    stmt", stmt, hir, ast);
    }
}

fn write_stmt(out: &mut String, label: &str, stmt: &Stmt, hir: &HirCtx, ast: &AstCtx) {
    match &stmt.kind {
        StmtKind::Expr(expr) => write_expr(out, label, *expr, hir, ast),
        StmtKind::Assignment(assign) => {
            let binding = match assign.binding {
                Some(BindingKind::Let) => "let ",
                Some(BindingKind::Var) => "var ",
                None => "",
            };
            let _ = write!(out, "{label} assign {binding}");
            write_target(out, &assign.target, ast);
            if let Some(ty) = assign.ty {
                let _ = write!(out, ": {}", fmt_type(ty, hir, ast));
            }
            out.push('\n');
            write_expr(out, "      value", assign.value, hir, ast);
        }
        StmtKind::If(if_stmt) => {
            let _ = writeln!(out, "{label} if");
            write_expr(out, "      cond", if_stmt.condition, hir, ast);
            write_body(out, "      then", if_stmt.then_body, hir, ast);
        }
        StmtKind::ForIn(for_in) => {
            let local = hir.local(for_in.local);
            let _ = writeln!(out, "{label} for {}", name(local.name, ast));
            write_expr(out, "      iter", for_in.iter, hir, ast);
            write_body(out, "      body", for_in.body, hir, ast);
        }
    }
}

fn write_expr(out: &mut String, label: &str, id: ExprId, hir: &HirCtx, ast: &AstCtx) {
    match &hir.expr(id).kind {
        ExprKind::Literal(lit) => {
            let _ = writeln!(out, "{label} {:?}", lit.kind);
        }
        ExprKind::Path(path) => {
            let _ = writeln!(out, "{label} path {}", fmt_path(path, ast));
        }
        ExprKind::ImplicitMember(member) => {
            let _ = writeln!(out, "{label} .{}", name(*member, ast));
        }
        ExprKind::Unary(unary) => {
            let _ = writeln!(out, "{label} unary {:?}", unary.op);
            write_expr(out, "      expr", unary.expr, hir, ast);
        }
        ExprKind::Binary(binary) => {
            let _ = writeln!(out, "{label} binary {:?}", binary.op);
            write_expr(out, "      lhs", binary.lhs, hir, ast);
            write_expr(out, "      rhs", binary.rhs, hir, ast);
        }
        ExprKind::Call(call) => {
            let _ = writeln!(out, "{label} call");
            write_expr(out, "      callee", call.callee, hir, ast);
            for arg in &call.args {
                write_expr(out, "      arg", arg.value, hir, ast);
            }
        }
        ExprKind::Field(field) => {
            let _ = writeln!(out, "{label} field {}", name(field.field, ast));
            write_expr(out, "      base", field.base, hir, ast);
        }
        ExprKind::Array(array) => {
            let _ = writeln!(out, "{label} array");
            for element in &array.elements {
                write_expr(out, "      elem", *element, hir, ast);
            }
        }
        ExprKind::If(if_expr) => {
            let _ = writeln!(out, "{label} if-expr");
            write_expr(out, "      cond", if_expr.condition, hir, ast);
            write_body(out, "      then", if_expr.then_body, hir, ast);
        }
    }
}

fn write_target(out: &mut String, target: &AssignTarget, ast: &AstCtx) {
    match target {
        AssignTarget::Ident(ident) => out.push_str(name(*ident, ast)),
        AssignTarget::Field { base, fields } => {
            out.push_str(name(*base, ast));
            for field in fields {
                let _ = write!(out, ".{}", name(*field, ast));
            }
        }
    }
}

fn fmt_type(id: TypeId, hir: &HirCtx, ast: &AstCtx) -> String {
    match &hir.ty(id).kind {
        TypeKind::Path { path, generic_args } => {
            let mut out = fmt_path(path, ast);
            if !generic_args.is_empty() {
                out.push('<');
                out.push_str(
                    &generic_args
                        .iter()
                        .map(|ty| fmt_type(*ty, hir, ast))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                out.push('>');
            }
            out
        }
    }
}

fn fmt_path(path: &ModPath, ast: &AstCtx) -> String {
    path.0
        .iter()
        .map(|ident| name(*ident, ast))
        .collect::<Vec<_>>()
        .join("::")
}

fn name(ident: Ident, ast: &AstCtx) -> &str {
    ast.get_str(ident.name)
}
