use std::iter::Peekable;

use ast::{
    common::{Ident, ModPath},
    ctx::AstCtx,
    item::{ConstDef, ImportDef, Item, ItemId, ItemKind},
    span::{FileId, Span},
    ty::{Type, TypeId, TypeKind},
};
use lexer::{
    Lexer,
    token::{Token, TokenKind},
};

use crate::error::{PResult, ParseError};

pub struct Parser<'a> {
    pub(crate) ctx: &'a mut AstCtx,
    pub(crate) source: &'a str,
    pub(crate) lexer: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(ctx: &'a mut AstCtx, source: &'a str, file: FileId) -> Self {
        Self {
            ctx,
            source,
            lexer: Lexer::new(source, file).peekable(),
        }
    }

    pub fn parse_program(&mut self) -> PResult<Vec<ItemId>> {
        let mut items = Vec::new();
        while let Some(token) = self.lexer.next() {
            if token.kind == TokenKind::Eof {
                break;
            }
            let item = self.parse_item(token)?;
            items.push(item);
        }
        Ok(items)
    }

    fn parse_item(&mut self, token: Token) -> PResult<ItemId> {
        match token.kind {
            TokenKind::KwImport => self.parse_import_def(token.span),
            TokenKind::KwConst => self.parse_const_def(token.span),
            _ => Err(ParseError::expected_item(token.kind, token.span)),
        }
    }

    pub(crate) fn expect(&mut self, kind: TokenKind, span: Span) -> PResult<Token> {
        let token = self
            .lexer
            .next()
            .ok_or_else(|| ParseError::unexpected_eof(Some(kind), span))?;
        if token.kind == TokenKind::Eof {
            return Err(ParseError::unexpected_eof(Some(kind), span));
        }
        if token.kind == kind {
            Ok(token)
        } else {
            Err(ParseError::expected(kind, token.kind, token.span))
        }
    }

    fn parse_import_def(&mut self, span: Span) -> PResult<ItemId> {
        let path = self.parse_path(span)?;
        let item_span = path
            .0
            .last()
            .map(|ident| span.merge(ident.span))
            .unwrap_or(span);
        let def = ImportDef { path };
        let item = Item {
            kind: ItemKind::Import(def),
            span: item_span,
        };
        Ok(self.ctx.alloc_item(item))
    }

    fn parse_path(&mut self, span: Span) -> PResult<ModPath> {
        let mut paths = Vec::new();
        let mut expected_ident_span = span;

        loop {
            let first = self.expect(TokenKind::Ident, expected_ident_span)?;
            let name = self.ctx.intern_str(&self.source[first.span.range()]);
            paths.push(Ident {
                name,
                span: first.span,
            });

            if let Some(separator) = self.lexer.next_if(|t| t.kind == TokenKind::ColonColon) {
                // If we hit EOF after `::`, report against the separator location.
                expected_ident_span = separator.span;
            } else {
                break;
            }
        }
        Ok(ModPath(paths))
    }

    fn parse_const_def(&mut self, const_span: Span) -> PResult<ItemId> {
        let name_tok = self.expect(TokenKind::Ident, const_span)?;
        let name = self.ctx.intern_str(&self.source[name_tok.span.range()]);
        let name_ident = Ident {
            name,
            span: name_tok.span,
        };

        let colon_tok = self.expect(TokenKind::Colon, name_tok.span)?;
        let ty_id = self.parse_type(colon_tok.span)?;
        let ty_span = self.ctx.get_type(ty_id).span;
        let eq_tok = self.expect(TokenKind::Assign, ty_span)?;
        if self
            .lexer
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Eof)
        {
            return Err(ParseError::unexpected_eof(None, eq_tok.span));
        }
        let expr_id = self.parse_expr()?;
        let expr_span = self.ctx.get_expr(expr_id).span;
        let item_span = const_span.merge(expr_span);

        let def = ConstDef {
            name: name_ident,
            ty: ty_id,
            value: expr_id,
        };
        let item = Item {
            kind: ItemKind::Const(def),
            span: item_span,
        };
        Ok(self.ctx.alloc_item(item))
    }

    pub(crate) fn parse_type(&mut self, prev_span: Span) -> PResult<TypeId> {
        let path = self.parse_path(prev_span)?;
        let span = match (path.0.first(), path.0.last()) {
            (Some(first), Some(last)) => first.span.merge(last.span),
            _ => prev_span,
        };
        let ty = Type {
            kind: TypeKind::Path(path),
            span,
        };
        Ok(self.ctx.alloc_type(ty))
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use ast::{
        AstVisitor,
        ctx::AstCtx,
        expr::{ExprId, ExprKind, LitKind},
        item::{ConstDef, ImportDef, ItemId},
        span::FileId,
        ty::TypeKind,
        walk_program,
    };
    use lexer::token::TokenKind;

    use super::Parser;
    use crate::error::{ParseError, ParseErrorKind};

    struct ParsedProgram {
        ctx: AstCtx,
        items: Vec<ItemId>,
    }

    impl ParsedProgram {
        fn compact_ast(&self) -> String {
            let mut formatter = CompactAstFormatter::new();
            walk_program(&mut formatter, &self.items, &self.ctx);
            formatter.into_output()
        }
    }

    struct CompactAstFormatter {
        out: String,
    }

    impl CompactAstFormatter {
        fn new() -> Self {
            Self { out: String::new() }
        }

        fn into_output(self) -> String {
            self.out
        }

        fn write_type(&mut self, ty: ast::ty::TypeId, ctx: &AstCtx) {
            match &ctx.get_type(ty).kind {
                TypeKind::Path(path) => self.write_path(path, ctx),
            }
        }

        fn write_path(&mut self, path: &ast::common::ModPath, ctx: &AstCtx) {
            for (idx, ident) in path.0.iter().enumerate() {
                if idx > 0 {
                    self.out.push_str("::");
                }
                self.out.push_str(ctx.get_str(ident.name));
            }
        }

        fn write_expr(&mut self, label: &str, expr_id: ExprId, ctx: &AstCtx) {
            let expr = ctx.get_expr(expr_id);
            match &expr.kind {
                ExprKind::Literal(lit) => match &lit.kind {
                    LitKind::Bool(value) => {
                        let _ = writeln!(&mut self.out, "{label}: bool {value}");
                    }
                    LitKind::Int => {
                        let _ = writeln!(&mut self.out, "{label}: int");
                    }
                    LitKind::Float => {
                        let _ = writeln!(&mut self.out, "{label}: float");
                    }
                    LitKind::Char => {
                        let _ = writeln!(&mut self.out, "{label}: char");
                    }
                    LitKind::Str => {
                        let _ = writeln!(&mut self.out, "{label}: str");
                    }
                },
                ExprKind::Path(ident) => {
                    let _ = writeln!(&mut self.out, "{label}: path {}", ctx.get_str(ident.name));
                }
                ExprKind::Unary(unary) => {
                    let _ = writeln!(&mut self.out, "{label}: unary {:?}", unary.op);
                    self.write_expr("  expr", unary.expr, ctx);
                }
                ExprKind::Binary(binary) => {
                    let _ = writeln!(&mut self.out, "{label}: binary {:?}", binary.op);
                    self.write_expr("  lhs", binary.lhs, ctx);
                    self.write_expr("  rhs", binary.rhs, ctx);
                }
                ExprKind::Call(call) => {
                    let _ = writeln!(&mut self.out, "{label}: call");
                    self.write_expr("  callee", call.callee, ctx);
                    for generic_arg in &call.generic_args {
                        self.out.push_str("  generic: ");
                        self.write_type(*generic_arg, ctx);
                        self.out.push('\n');
                    }
                    for arg in &call.args {
                        match &arg.label {
                            Some(label) => {
                                let _ = writeln!(
                                    &mut self.out,
                                    "  arg label: {}",
                                    ctx.get_str(label.name)
                                );
                            }
                            None => self.out.push_str("  arg\n"),
                        }
                        self.write_expr("    value", arg.value, ctx);
                    }
                }
                ExprKind::Field(field) => {
                    let _ = writeln!(
                        &mut self.out,
                        "{label}: field {}",
                        ctx.get_str(field.field.name)
                    );
                    self.write_expr("  base", field.base, ctx);
                }
            }
        }
    }

    impl AstVisitor for CompactAstFormatter {
        fn visit_import(&mut self, _item_id: ItemId, import: &ImportDef, ctx: &AstCtx) {
            self.out.push_str("import ");
            self.write_path(&import.path, ctx);
            self.out.push('\n');
        }

        fn visit_const(&mut self, _item_id: ItemId, def: &ConstDef, ctx: &AstCtx) {
            let _ = write!(&mut self.out, "const {}: ", ctx.get_str(def.name.name));
            self.write_type(def.ty, ctx);
            self.out.push('\n');
            self.write_expr("  value", def.value, ctx);
        }
    }

    fn parse(source: &str) -> (AstCtx, Result<Vec<ItemId>, ParseError>) {
        let mut ctx = AstCtx::new();
        let mut parser = Parser::new(&mut ctx, source, FileId(0));
        let result = parser.parse_program();
        (ctx, result)
    }

    fn parse_ok(source: &str) -> ParsedProgram {
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        ParsedProgram { ctx, items }
    }

    fn parse_err(source: &str) -> ParseError {
        let (_ctx, result) = parse(source);
        result.expect_err("parser should reject source")
    }

    #[test]
    fn parses_single_segment_import() {
        let parsed = parse_ok("import std");

        assert_eq!(parsed.compact_ast(), "import std\n");
    }

    #[test]
    fn parses_multi_segment_import_and_merges_item_span() {
        let source = "import std::Option";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        assert_eq!(items.len(), 1);

        let item = ctx.get_item(items[0]);
        assert_eq!(item.span.start, 0);
        assert_eq!(item.span.end, source.len());
        assert_eq!(
            ParsedProgram { ctx, items }.compact_ast(),
            "import std::Option\n"
        );
    }

    #[test]
    fn errors_on_missing_import_path() {
        let err = parse_err("import");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, 6);
    }

    #[test]
    fn errors_on_trailing_path_separator_at_separator_span() {
        let err = parse_err("import std::");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 10);
        assert_eq!(err.span.end, 12);
        assert_eq!(err.span.file, FileId(0));
    }

    #[test]
    fn errors_on_non_ident_after_separator() {
        let err = parse_err("import std::1");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
        assert_eq!(err.span.start, 12);
        assert_eq!(err.span.end, 13);
    }

    #[test]
    fn parses_const_float() {
        let parsed = parse_ok("const PI: f64 = 3.14");

        assert_eq!(parsed.compact_ast(), "const PI: f64\n  value: float\n");
    }

    #[test]
    fn parses_const_str() {
        let parsed = parse_ok(r#"const NAME: str = "abc""#);

        assert_eq!(parsed.compact_ast(), "const NAME: str\n  value: str\n");
    }

    #[test]
    fn parses_const_bool_true() {
        let parsed = parse_ok("const FLAG: bool = true");

        assert_eq!(
            parsed.compact_ast(),
            "const FLAG: bool\n  value: bool true\n"
        );
    }

    #[test]
    fn parses_mixed_consts_and_import() {
        let parsed = parse_ok("const A: i32 = 1\nimport std\nconst B: str = \"x\"");

        assert_eq!(
            parsed.compact_ast(),
            "const A: i32\n  value: int\nimport std\nconst B: str\n  value: str\n"
        );
    }

    #[test]
    fn parses_expr_precedence() {
        let parsed = parse_ok("const X: i32 = 1 + 2 * 3");

        assert_eq!(
            parsed.compact_ast(),
            "const X: i32\n  value: binary Add\n  lhs: int\n  rhs: binary Mul\n  lhs: int\n  rhs: int\n"
        );
    }

    #[test]
    fn parses_unary_field_and_call() {
        let parsed = parse_ok("const X: bool = !foo.bar::<T>(a: 1, 2)");

        assert_eq!(
            parsed.compact_ast(),
            "const X: bool\n  value: unary Not\n  expr: call\n  callee: field bar\n  base: path foo\n  generic: T\n  arg label: a\n    value: int\n  arg\n    value: int\n"
        );
    }

    #[test]
    fn errors_on_const_missing_name() {
        let err = parse_err("const");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 0);
        assert_eq!(err.span.end, 5);
    }

    #[test]
    fn errors_on_const_missing_colon() {
        let err = parse_err("const X");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Colon)
            }
        ));
        assert_eq!(err.span.start, 6);
        assert_eq!(err.span.end, 7);
    }

    #[test]
    fn errors_on_const_missing_type_after_colon() {
        let err = parse_err("const X:");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Ident)
            }
        ));
        assert_eq!(err.span.start, 7);
        assert_eq!(err.span.end, 8);
    }

    #[test]
    fn errors_on_const_missing_assign() {
        let err = parse_err("const X: T");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof {
                expected: Some(TokenKind::Assign)
            }
        ));
        assert_eq!(err.span.start, 9);
        assert_eq!(err.span.end, 10);
    }

    #[test]
    fn errors_on_const_missing_rhs() {
        let err = parse_err("const X: T =");
        assert!(matches!(
            err.kind,
            ParseErrorKind::UnexpectedEof { expected: None }
        ));
        let source = "const X: T =";
        assert_eq!(err.span.start, source.len() - 1);
        assert_eq!(err.span.end, source.len());
    }

    #[test]
    fn errors_on_const_non_literal_rhs() {
        let err = parse_err("const X: T = +");
        assert!(matches!(
            err.kind,
            ParseErrorKind::ExpectedExpr {
                found: TokenKind::Plus
            }
        ));
        let source = "const X: T = +";
        let plus = source.rfind('+').expect("plus");
        assert_eq!(err.span.start, plus);
        assert_eq!(err.span.end, plus + 1);
    }

    #[test]
    fn errors_on_const_integer_as_name() {
        let err = parse_err("const 1: T = 0");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Integer
            }
        ));
        assert_eq!(err.span.start, 6);
        assert_eq!(err.span.end, 7);
    }

    #[test]
    fn parses_const_multi_segment_type_path() {
        let source = "const OPT: std::Option = 1";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept source");
        assert_eq!(items.len(), 1);

        let item = ctx.get_item(items[0]);
        let ast::item::ItemKind::Const(def) = &item.kind else {
            panic!("expected const");
        };
        let ty = ctx.get_type(def.ty);
        let TypeKind::Path(path) = &ty.kind;
        assert_eq!(path.0.len(), 2);
        assert_eq!(ctx.get_str(path.0[0].name), "std");
        assert_eq!(ctx.get_str(path.0[1].name), "Option");
        let type_snippet = "std::Option";
        let start = source.find(type_snippet).expect("type snippet");
        assert_eq!(ty.span.start, start);
        assert_eq!(ty.span.end, start + type_snippet.len());
        assert_eq!(&source[ty.span.range()], type_snippet);
        assert_eq!(
            ParsedProgram { ctx, items }.compact_ast(),
            "const OPT: std::Option\n  value: int\n"
        );
    }

    #[test]
    fn errors_on_const_trailing_colon_colon_in_type() {
        let err = parse_err("const X: std:: = 1");
        assert!(matches!(
            err.kind,
            ParseErrorKind::Expected {
                expected: TokenKind::Ident,
                found: TokenKind::Assign
            }
        ));
        let source = "const X: std:: = 1";
        let eq = source.find('=').expect("=");
        assert_eq!(err.span.start, eq);
        assert_eq!(err.span.end, eq + 1);
    }
}
