pub mod ctx;
pub mod debug;
pub mod def;
pub mod diagnostic;
pub mod expr;
pub mod ids;
pub mod item;
pub mod lower;
pub mod ty;

pub use ctx::HirCtx;
pub use debug::format_hir_module;
pub use def::*;
pub use diagnostic::{HirDiagnostic, HirDiagnosticKind};
pub use expr::*;
pub use ids::*;
pub use item::*;
pub use lower::{HirLowerOutput, HirLowerer, ModuleSource, lower_program};
pub use ty::*;

#[cfg(test)]
mod tests {
    use ast::{ctx::AstCtx, span::FileId};
    use parser::Parser;

    use crate::{
        DefKind, HirDiagnosticKind, HirLowerer, ModuleSource,
        debug::format_hir_module,
        item::{ItemKind, StmtKind},
    };

    fn lower(source: &str) -> crate::HirLowerOutput {
        let mut ast = AstCtx::new();
        let file = FileId(0);
        let mut parser = Parser::new(&mut ast, source, file);
        let items = parser.parse_program().expect("parser accepts source");
        HirLowerer::new(&ast).lower_module(&items, ModuleSource { file })
    }

    fn format(source: &str) -> String {
        let mut ast = AstCtx::new();
        let file = FileId(0);
        let mut parser = Parser::new(&mut ast, source, file);
        let items = parser.parse_program().expect("parser accepts source");
        let output = HirLowerer::new(&ast).lower_module(&items, ModuleSource { file });
        format_hir_module(output.module, &output.hir, &ast)
    }

    #[test]
    fn lowers_import_and_const() {
        assert_eq!(
            format("import std::io\nconst X: i32 = 1"),
            "module FileId(0)\nnamespaces value=1 type=0\nimport std::io\nconst X: i32\n  value Int\n"
        );
    }

    #[test]
    fn lowers_function_params_body_and_locals() {
        assert_eq!(
            format("fn f(a: i32) -> i32 { let x: i32 = a x }"),
            "module FileId(0)\nnamespaces value=1 type=0\nfn f\n  param a: i32\n  return i32\n  body scope params=1 locals=1\n    stmt assign let x: i32\n      value path a\n    stmt path x\n"
        );
    }

    #[test]
    fn lowers_struct_enum_and_extend_method() {
        assert_eq!(
            format(
                "struct Point(x: f32, y: f32)\nenum Option<T> { some(T), none }\nextend Point { fn len(self) { self.x } }"
            ),
            "module FileId(0)\nnamespaces value=0 type=2\nstruct Point\n  field x: f32\n  field y: f32\nenum Option\n  generic T\n  variant some(T)\n  variant none()\nextend Point\n  method Point.len\n  param self: self\n  body scope params=1 locals=0\n    stmt field x\n      base path self\n"
        );
    }

    #[test]
    fn lowers_if_for_assignment_and_array() {
        assert_eq!(
            format("fn f() { var xs = [1, 2] for x in xs { x } if true { xs = [] } }"),
            "module FileId(0)\nnamespaces value=1 type=0\nfn f\n  body scope params=0 locals=1\n    stmt assign var xs\n      value array\n      elem Int\n      elem Int\n    stmt for x\n      iter path xs\n      body scope params=0 locals=1\n    stmt path x\n    stmt if\n      cond Bool(true)\n      then scope params=0 locals=0\n    stmt assign xs\n      value array\n"
        );
    }

    #[test]
    fn diagnostics_duplicate_top_level_namespaces() {
        let output = lower("const A: i32 = 1\nfn A() {}\nstruct A()\nenum A {}");
        assert!(matches!(
            output.diagnostics[0].kind,
            HirDiagnosticKind::DuplicateTopLevelValue { .. }
        ));
        assert!(matches!(
            output.diagnostics[1].kind,
            HirDiagnosticKind::DuplicateTopLevelType { .. }
        ));
    }

    #[test]
    fn value_and_type_same_name_allowed() {
        assert!(lower("fn A() {}\nstruct A()").diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_duplicate_params_and_locals() {
        let output = lower("fn f(a: i32, a: i32) { let x = 1 let x = 2 let a = 3 }");
        assert_eq!(output.diagnostics.len(), 3);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.kind, HirDiagnosticKind::DuplicateParam { .. }))
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.kind, HirDiagnosticKind::DuplicateLocal { .. }))
        );
    }

    #[test]
    fn inner_shadow_allowed() {
        assert!(
            lower("fn f(a: i32) { if true { let a = 1 } }")
                .diagnostics
                .is_empty()
        );
    }

    #[test]
    fn diagnostics_duplicate_fields_variants_generics_methods() {
        let output = lower(
            "struct S<T, T>(x: i32, x: i32)\nenum E { a, a }\nextend S { fn f(self) {} fn f(self) {} }",
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.kind, HirDiagnosticKind::DuplicateGenericParam { .. }))
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.kind, HirDiagnosticKind::DuplicateField { .. }))
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.kind, HirDiagnosticKind::DuplicateVariant { .. }))
        );
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| matches!(diag.kind, HirDiagnosticKind::DuplicateMethod { .. }))
        );
    }

    #[test]
    fn imports_recorded_not_resolved() {
        let output = lower("import missing::Thing");
        let module = output.hir.module(output.module);
        assert_eq!(module.imports.len(), 1);
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn lowers_kitchen_sink() {
        let output = lower(include_str!("../../../assets/kitchen_sink.pr"));
        assert!(matches!(
            output
                .hir
                .item(output.hir.module(output.module).items[0])
                .kind,
            ItemKind::Struct(_)
        ));
    }

    #[test]
    fn exposes_typeck_top_level_and_member_defs() {
        let output =
            lower("const C: i32 = 1\nfn f() {}\nstruct S<T>(x: i32)\nenum E { a, b(i32) }");
        let module = output.hir.module(output.module);

        assert_eq!(output.hir.module_value_items(output.module).len(), 2);
        assert_eq!(output.hir.module_type_items(output.module).len(), 2);

        let const_def = output
            .hir
            .item_def(module.value_items[0])
            .expect("const has def");
        assert!(matches!(output.hir.def(const_def).kind, DefKind::Const(_)));

        let struct_item = module.type_items[0];
        let fields = output.hir.struct_fields(struct_item);
        assert_eq!(fields.len(), 1);
        assert!(matches!(
            output.hir.def(output.hir.field(fields[0]).def).kind,
            DefKind::Field(_)
        ));

        let enum_item = module.type_items[1];
        let variants = output.hir.enum_variants(enum_item);
        assert_eq!(variants.len(), 2);
        assert!(matches!(
            output.hir.def(output.hir.variant(variants[0]).def).kind,
            DefKind::Variant(_)
        ));
    }

    #[test]
    fn exposes_scope_parent_walk_and_shadowing() {
        let output = lower("fn f(a: i32) { let x = 1 if true { let x = 2 a } }");
        let module = output.hir.module(output.module);
        let function = match &output.hir.item(module.value_items[0]).kind {
            ItemKind::Function(function) => function,
            _ => panic!("expected function"),
        };
        let root_body = output.hir.body(function.body);
        let root_scope = root_body.scope;

        assert_eq!(output.hir.scope_params(root_scope).len(), 1);
        assert_eq!(output.hir.scope_locals(root_scope).len(), 1);
        assert!(matches!(
            output
                .hir
                .def(output.hir.param(output.hir.scope_params(root_scope)[0]).def)
                .kind,
            DefKind::Param(_)
        ));

        let if_stmt = root_body
            .stmts
            .iter()
            .find_map(|stmt| match &stmt.kind {
                StmtKind::If(if_stmt) => Some(if_stmt),
                _ => None,
            })
            .expect("if stmt");
        let then_scope = output.hir.body(if_stmt.then_body).scope;
        assert_eq!(output.hir.scope_parent(then_scope), Some(root_scope));
        assert_eq!(output.hir.scope_locals(then_scope).len(), 1);
        assert!(matches!(
            output
                .hir
                .def(output.hir.local(output.hir.scope_locals(then_scope)[0]).def)
                .kind,
            DefKind::Local(_)
        ));
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn exposes_method_container_and_item_path() {
        let output = lower("extend S { fn f(self) { self } }");
        let module = output.hir.module(output.module);
        let extend = module.items[0];
        let methods = output.hir.extend_methods(extend);
        assert_eq!(methods.len(), 1);
        assert_eq!(output.hir.item_path(methods[0]).len(), 2);

        let method_def = output.hir.item_def(methods[0]).expect("method has def");
        assert!(matches!(
            output.hir.def(method_def).kind,
            DefKind::Method(_)
        ));
        assert_eq!(output.hir.def(method_def).parent_item, Some(extend));
    }

    #[test]
    fn records_spans_for_typeck_diagnostics() {
        let output = lower("struct S<T>(field: i32)\nenum E { case(i32) }");
        let module = output.hir.module(output.module);

        let struct_item = module.type_items[0];
        let struct_def = match &output.hir.item(struct_item).kind {
            ItemKind::Struct(def) => def,
            _ => panic!("expected struct"),
        };
        let generic = output.hir.generic_param(struct_def.generic_params[0]);
        let field = output.hir.field(struct_def.fields[0]);
        assert!(generic.span.start < generic.span.end);
        assert!(field.span.start < field.span.end);
        assert_eq!(output.hir.def(generic.def).span.start, generic.span.start);
        assert_eq!(output.hir.def(field.def).span.start, field.span.start);

        let enum_item = module.type_items[1];
        let enum_def = match &output.hir.item(enum_item).kind {
            ItemKind::Enum(def) => def,
            _ => panic!("expected enum"),
        };
        let variant = output.hir.variant(enum_def.variants[0]);
        assert!(variant.span.start < variant.span.end);
        assert_eq!(output.hir.def(variant.def).span.start, variant.span.start);
    }
}
