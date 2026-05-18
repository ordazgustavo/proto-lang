use ast::{
    common::Ident,
    ctx::AstCtx,
    expr as ast_expr, item as ast_item,
    span::{FileId, Span},
    ty as ast_ty,
};

use crate::{
    ctx::HirCtx,
    diagnostic::{HirDiagnostic, HirDiagnosticKind},
    expr::*,
    ids::*,
    item::*,
    ty::{Type, TypeKind},
};

#[derive(Debug, Clone, Copy)]
pub struct ModuleSource {
    pub file: FileId,
}

#[derive(Debug)]
pub struct HirLowerOutput {
    pub hir: HirCtx,
    pub module: ModuleId,
    pub diagnostics: Vec<HirDiagnostic>,
}

pub fn lower_program(
    ast_items: &[ast_item::ItemId],
    ast: &AstCtx,
    file: FileId,
) -> (HirCtx, ModuleId) {
    let output = HirLowerer::new(ast).lower_module(ast_items, ModuleSource { file });
    (output.hir, output.module)
}

pub struct HirLowerer<'a> {
    ast: &'a AstCtx,
    hir: HirCtx,
    diagnostics: Vec<HirDiagnostic>,
    scopes: Vec<ScopeId>,
}

impl<'a> HirLowerer<'a> {
    pub fn new(ast: &'a AstCtx) -> Self {
        Self {
            ast,
            hir: HirCtx::new(),
            diagnostics: Vec::new(),
            scopes: Vec::new(),
        }
    }

    pub fn lower_module(
        mut self,
        ast_items: &[ast_item::ItemId],
        source: ModuleSource,
    ) -> HirLowerOutput {
        let mut imports = Vec::new();
        let mut items = Vec::new();
        let mut value_items = Vec::new();
        let mut type_items = Vec::new();
        let mut span = None;
        let mut values = NameSet::new();
        let mut types = NameSet::new();

        for &ast_item_id in ast_items {
            let ast_item = self.ast.get_item(ast_item_id);
            span = Some(span.map_or(ast_item.span, |prev: Span| prev.merge(ast_item.span)));
            match &ast_item.kind {
                ast_item::ItemKind::Import(import) => imports.push(self.hir.alloc_import(Import {
                    path: import.path.clone(),
                    span: ast_item.span,
                })),
                ast_item::ItemKind::Const(def) => {
                    values.insert(def.name, duplicate_top_level_value, &mut self.diagnostics);
                    let item = self.lower_item(ast_item);
                    value_items.push(item);
                    items.push(item);
                }
                ast_item::ItemKind::Function(def) => {
                    values.insert(def.name, duplicate_top_level_value, &mut self.diagnostics);
                    let item = self.lower_item(ast_item);
                    value_items.push(item);
                    items.push(item);
                }
                ast_item::ItemKind::Struct(def) => {
                    types.insert(def.name, duplicate_top_level_type, &mut self.diagnostics);
                    let item = self.lower_item(ast_item);
                    type_items.push(item);
                    items.push(item);
                }
                ast_item::ItemKind::Enum(def) => {
                    types.insert(def.name, duplicate_top_level_type, &mut self.diagnostics);
                    let item = self.lower_item(ast_item);
                    type_items.push(item);
                    items.push(item);
                }
                ast_item::ItemKind::Extend(_) => items.push(self.lower_item(ast_item)),
            }
        }

        let module = self.hir.alloc_module(Module {
            file: source.file,
            imports,
            items,
            value_items,
            type_items,
            span,
        });
        HirLowerOutput {
            hir: self.hir,
            module,
            diagnostics: self.diagnostics,
        }
    }

    fn lower_item(&mut self, item: &ast_item::Item) -> ItemId {
        let kind = match &item.kind {
            ast_item::ItemKind::Import(_) => unreachable!("imports are module-level"),
            ast_item::ItemKind::Const(def) => ItemKind::Const(Const {
                name: def.name,
                ty: self.lower_type(def.ty),
                value: self.lower_expr(def.value),
            }),
            ast_item::ItemKind::Function(def) => ItemKind::Function(self.lower_function(def)),
            ast_item::ItemKind::Struct(def) => ItemKind::Struct(Struct {
                name: def.name,
                generic_params: self.lower_generic_params(&def.generic_params),
                fields: self.lower_fields(&def.fields),
            }),
            ast_item::ItemKind::Enum(def) => ItemKind::Enum(Enum {
                name: def.name,
                generic_params: self.lower_generic_params(&def.generic_params),
                variants: self.lower_variants(&def.variants),
            }),
            ast_item::ItemKind::Extend(def) => {
                let generic_params = self.lower_generic_params(&def.generic_params);
                let mut methods = Vec::new();
                let mut names = NameSet::new();
                for method in &def.methods {
                    names.insert(method.name, duplicate_method, &mut self.diagnostics);
                    methods.push(self.lower_method(def.target, method, item.span));
                }
                ItemKind::Extend(Extend {
                    target: def.target,
                    generic_params,
                    methods,
                })
            }
        };
        self.hir.alloc_item(Item {
            kind,
            span: item.span,
        })
    }

    fn lower_function(&mut self, def: &ast_item::FunctionDef) -> Function {
        let generic_params = self.lower_generic_params(&def.generic_params);
        let scope = self.push_scope(None);
        let params = self.lower_params(&def.params, scope);
        let body = self.lower_block_with_scope(&def.body, scope);
        self.pop_scope();
        Function {
            name: def.name,
            generic_params,
            params,
            return_type: def.return_type.map(|ty| self.lower_type(ty)),
            body,
        }
    }

    fn lower_method(&mut self, target: Ident, def: &ast_item::FunctionDef, span: Span) -> ItemId {
        let generic_params = self.lower_generic_params(&def.generic_params);
        let scope = self.push_scope(None);
        let params = self.lower_params(&def.params, scope);
        let body = self.lower_block_with_scope(&def.body, scope);
        self.pop_scope();
        let return_type = def.return_type.map(|ty| self.lower_type(ty));
        self.hir.alloc_item(Item {
            kind: ItemKind::Method(Method {
                target,
                name: def.name,
                generic_params,
                params,
                return_type,
                body,
            }),
            span,
        })
    }

    fn lower_generic_params(&mut self, params: &[Ident]) -> Vec<GenericParamId> {
        let mut names = NameSet::new();
        params
            .iter()
            .map(|param| {
                names.insert(*param, duplicate_generic_param, &mut self.diagnostics);
                self.hir.alloc_generic_param(GenericParam { name: *param })
            })
            .collect()
    }

    fn lower_params(&mut self, params: &[ast_item::Param], scope: ScopeId) -> Vec<ParamId> {
        let mut names = NameSet::new();
        params
            .iter()
            .map(|param| {
                names.insert(param.name, duplicate_param, &mut self.diagnostics);
                let ty = self.lower_type(param.ty);
                let id = self.hir.alloc_param(Param {
                    label: lower_param_label(&param.label),
                    name: param.name,
                    ty,
                });
                self.hir.scope_mut(scope).params.push(id);
                id
            })
            .collect()
    }

    fn lower_fields(&mut self, fields: &[ast_item::Param]) -> Vec<FieldId> {
        let mut names = NameSet::new();
        fields
            .iter()
            .map(|field| {
                names.insert(field.name, duplicate_field, &mut self.diagnostics);
                let ty = self.lower_type(field.ty);
                self.hir.alloc_field(Field {
                    label: lower_param_label(&field.label),
                    name: field.name,
                    ty,
                })
            })
            .collect()
    }

    fn lower_variants(&mut self, variants: &[ast_item::EnumVariant]) -> Vec<VariantId> {
        let mut names = NameSet::new();
        variants
            .iter()
            .map(|variant| {
                names.insert(variant.name, duplicate_variant, &mut self.diagnostics);
                let payload = variant
                    .payload
                    .iter()
                    .map(|ty| self.lower_type(*ty))
                    .collect();
                self.hir.alloc_variant(Variant {
                    name: variant.name,
                    payload,
                    span: variant.name.span,
                })
            })
            .collect()
    }

    fn lower_block(&mut self, block: &ast_item::Block) -> BodyId {
        let parent = self.scopes.last().copied();
        let scope = self.push_scope(parent);
        let body = self.lower_block_with_scope(block, scope);
        self.pop_scope();
        body
    }

    fn lower_block_with_scope(&mut self, block: &ast_item::Block, scope: ScopeId) -> BodyId {
        let stmts = block
            .stmts
            .iter()
            .map(|stmt| self.lower_stmt(stmt, scope))
            .collect();
        self.hir.alloc_body(Body {
            stmts,
            scope,
            span: block.span,
        })
    }

    fn lower_stmt(&mut self, stmt: &ast_item::Stmt, scope: ScopeId) -> Stmt {
        let kind = match &stmt.kind {
            ast_item::StmtKind::Expr(expr) => StmtKind::Expr(self.lower_expr(*expr)),
            ast_item::StmtKind::Assignment(assign) => {
                let ty = assign.ty.map(|ty| self.lower_type(ty));
                let binding = assign.binding.as_ref().map(lower_binding);
                let local = match (binding, &assign.target) {
                    (Some(binding), ast_item::AssignTarget::Ident(name)) => {
                        Some(self.add_local(scope, binding, *name, ty, stmt.span))
                    }
                    _ => None,
                };
                StmtKind::Assignment(Assignment {
                    binding,
                    local,
                    target: lower_assign_target(&assign.target),
                    ty,
                    value: self.lower_expr(assign.value),
                })
            }
            ast_item::StmtKind::If(if_stmt) => StmtKind::If(self.lower_if_stmt(if_stmt)),
            ast_item::StmtKind::ForIn(for_in) => {
                let parent = self.scopes.last().copied();
                let body_scope = self.push_scope(parent);
                let local = self.add_local(
                    body_scope,
                    BindingKind::Let,
                    for_in.binding,
                    None,
                    for_in.binding.span,
                );
                let iter = self.lower_expr(for_in.iter);
                let body = self.lower_block_with_scope(&for_in.body, body_scope);
                self.pop_scope();
                StmtKind::ForIn(ForInStmt { local, iter, body })
            }
        };
        Stmt {
            kind,
            span: stmt.span,
        }
    }

    fn add_local(
        &mut self,
        scope: ScopeId,
        binding: BindingKind,
        name: Ident,
        ty: Option<TypeId>,
        span: Span,
    ) -> LocalId {
        for param in &self.hir.scope(scope).params {
            let existing = self.hir.param(*param).name;
            if existing.name == name.name {
                self.diagnostics.push(HirDiagnostic::new(
                    HirDiagnosticKind::DuplicateLocal {
                        name,
                        first: existing.span,
                    },
                    name.span,
                ));
                break;
            }
        }
        for local in &self.hir.scope(scope).locals {
            let existing = self.hir.local(*local).name;
            if existing.name == name.name {
                self.diagnostics.push(HirDiagnostic::new(
                    HirDiagnosticKind::DuplicateLocal {
                        name,
                        first: existing.span,
                    },
                    name.span,
                ));
                break;
            }
        }
        let id = self.hir.alloc_local(Local {
            binding,
            name,
            ty,
            span,
        });
        self.hir.scope_mut(scope).locals.push(id);
        id
    }

    fn lower_if_stmt(&mut self, if_stmt: &ast_item::IfStmt) -> IfStmt {
        IfStmt {
            condition: self.lower_expr(if_stmt.condition),
            then_body: self.lower_block(&if_stmt.then_block),
            else_branch: if_stmt.else_branch.as_ref().map(|branch| match branch {
                ast_item::ElseBranch::If(if_stmt) => {
                    ElseBranch::If(Box::new(self.lower_if_stmt(if_stmt)))
                }
                ast_item::ElseBranch::Block(block) => ElseBranch::Body(self.lower_block(block)),
            }),
        }
    }

    fn lower_expr(&mut self, id: ast_expr::ExprId) -> ExprId {
        let expr = self.ast.get_expr(id);
        let kind = match &expr.kind {
            ast_expr::ExprKind::Literal(lit) => ExprKind::Literal(Lit {
                kind: lower_lit_kind(&lit.kind),
                span: lit.span,
            }),
            ast_expr::ExprKind::Path(path) => ExprKind::Path(path.clone()),
            ast_expr::ExprKind::ImplicitMember(member) => ExprKind::ImplicitMember(*member),
            ast_expr::ExprKind::Unary(unary) => ExprKind::Unary(UnaryExpr {
                op: lower_unary_op(&unary.op),
                expr: self.lower_expr(unary.expr),
            }),
            ast_expr::ExprKind::Binary(binary) => ExprKind::Binary(BinaryExpr {
                op: lower_binary_op(&binary.op),
                lhs: self.lower_expr(binary.lhs),
                rhs: self.lower_expr(binary.rhs),
            }),
            ast_expr::ExprKind::Call(call) => ExprKind::Call(CallExpr {
                callee: self.lower_expr(call.callee),
                generic_args: call
                    .generic_args
                    .iter()
                    .map(|ty| self.lower_type(*ty))
                    .collect(),
                args: call
                    .args
                    .iter()
                    .map(|arg| Arg {
                        label: arg.label,
                        value: self.lower_expr(arg.value),
                    })
                    .collect(),
            }),
            ast_expr::ExprKind::Field(field) => ExprKind::Field(FieldExpr {
                base: self.lower_expr(field.base),
                field: field.field,
            }),
            ast_expr::ExprKind::Array(array) => ExprKind::Array(ArrayExpr {
                elements: array
                    .elements
                    .iter()
                    .map(|expr| self.lower_expr(*expr))
                    .collect(),
            }),
            ast_expr::ExprKind::If(if_expr) => ExprKind::If(IfExpr {
                condition: self.lower_expr(if_expr.condition),
                then_body: self.lower_block(&if_expr.then_block),
                else_branch: match &if_expr.else_branch {
                    ast_expr::IfElseBranch::If(expr) => IfElseBranch::If(self.lower_expr(*expr)),
                    ast_expr::IfElseBranch::Block(block) => {
                        IfElseBranch::Body(self.lower_block(block))
                    }
                },
            }),
        };
        self.hir.alloc_expr(Expr {
            kind,
            span: expr.span,
        })
    }

    fn lower_type(&mut self, id: ast_ty::TypeId) -> TypeId {
        let ty = self.ast.get_type(id);
        let kind = match &ty.kind {
            ast_ty::TypeKind::Path { path, generic_args } => TypeKind::Path {
                path: path.clone(),
                generic_args: generic_args.iter().map(|ty| self.lower_type(*ty)).collect(),
            },
        };
        self.hir.alloc_type(Type {
            kind,
            span: ty.span,
        })
    }

    fn push_scope(&mut self, parent: Option<ScopeId>) -> ScopeId {
        let scope = self.hir.alloc_scope(Scope {
            parent,
            params: Vec::new(),
            locals: Vec::new(),
        });
        self.scopes.push(scope);
        scope
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}

struct NameSet(Vec<(Ident, Span)>);

impl NameSet {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn insert(
        &mut self,
        name: Ident,
        kind: fn(Ident, Span) -> HirDiagnosticKind,
        diagnostics: &mut Vec<HirDiagnostic>,
    ) {
        if let Some((_, span)) = self
            .0
            .iter()
            .find(|(existing, _)| existing.name == name.name)
        {
            diagnostics.push(HirDiagnostic::new(kind(name, *span), name.span));
        } else {
            self.0.push((name, name.span));
        }
    }
}

fn lower_param_label(label: &ast_item::ParamLabel) -> ParamLabel {
    match label {
        ast_item::ParamLabel::Implicit => ParamLabel::Implicit,
        ast_item::ParamLabel::Explicit(label) => ParamLabel::Explicit(*label),
        ast_item::ParamLabel::Suppressed => ParamLabel::Suppressed,
    }
}

fn duplicate_top_level_value(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateTopLevelValue { name, first }
}

fn duplicate_top_level_type(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateTopLevelType { name, first }
}

fn duplicate_param(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateParam { name, first }
}

fn duplicate_field(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateField { name, first }
}

fn duplicate_variant(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateVariant { name, first }
}

fn duplicate_generic_param(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateGenericParam { name, first }
}

fn duplicate_method(name: Ident, first: Span) -> HirDiagnosticKind {
    HirDiagnosticKind::DuplicateMethod { name, first }
}

fn lower_binding(binding: &ast_item::BindingKind) -> BindingKind {
    match binding {
        ast_item::BindingKind::Let => BindingKind::Let,
        ast_item::BindingKind::Var => BindingKind::Var,
    }
}

fn lower_assign_target(target: &ast_item::AssignTarget) -> AssignTarget {
    match target {
        ast_item::AssignTarget::Ident(ident) => AssignTarget::Ident(*ident),
        ast_item::AssignTarget::Field { base, fields } => AssignTarget::Field {
            base: *base,
            fields: fields.clone(),
        },
    }
}

fn lower_lit_kind(kind: &ast_expr::LitKind) -> LitKind {
    match kind {
        ast_expr::LitKind::Int => LitKind::Int,
        ast_expr::LitKind::Float => LitKind::Float,
        ast_expr::LitKind::Bool(value) => LitKind::Bool(*value),
        ast_expr::LitKind::Char => LitKind::Char,
        ast_expr::LitKind::Str => LitKind::Str,
    }
}

fn lower_unary_op(op: &ast_expr::UnaryOp) -> UnaryOp {
    match op {
        ast_expr::UnaryOp::Neg => UnaryOp::Neg,
        ast_expr::UnaryOp::Not => UnaryOp::Not,
    }
}

fn lower_binary_op(op: &ast_expr::BinaryOp) -> BinaryOp {
    match op {
        ast_expr::BinaryOp::Add => BinaryOp::Add,
        ast_expr::BinaryOp::Sub => BinaryOp::Sub,
        ast_expr::BinaryOp::Mul => BinaryOp::Mul,
        ast_expr::BinaryOp::Div => BinaryOp::Div,
        ast_expr::BinaryOp::Rem => BinaryOp::Rem,
        ast_expr::BinaryOp::Lt => BinaryOp::Lt,
        ast_expr::BinaryOp::LtEq => BinaryOp::LtEq,
        ast_expr::BinaryOp::Gt => BinaryOp::Gt,
        ast_expr::BinaryOp::GtEq => BinaryOp::GtEq,
        ast_expr::BinaryOp::Eq => BinaryOp::Eq,
        ast_expr::BinaryOp::NotEq => BinaryOp::NotEq,
    }
}
