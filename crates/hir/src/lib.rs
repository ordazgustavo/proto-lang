pub mod ctx;
pub mod debug;
pub mod diagnostic;
pub mod expr;
pub mod ids;
pub mod item;
pub mod lower;
pub mod ty;

pub use ctx::HirCtx;
pub use debug::format_hir_module;
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
        HirDiagnosticKind, HirLowerer, ModuleSource, debug::format_hir_module, item::ItemKind,
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
}
