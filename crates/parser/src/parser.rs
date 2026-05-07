use std::iter::Peekable;

use ast::{
    common::{Ident, ModPath},
    ctx::AstCtx,
    expr::{Expr, ExprId, ExprKind, Lit, LitKind},
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
    ctx: &'a mut AstCtx,
    source: &'a str,
    lexer: Peekable<Lexer<'a>>,
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

    fn expect(&mut self, kind: TokenKind, span: Span) -> PResult<Token> {
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
        Ok(self.ctx.items.alloc(item))
    }

    fn parse_path(&mut self, span: Span) -> PResult<ModPath> {
        let mut paths = Vec::new();
        let mut expected_ident_span = span;

        loop {
            let first = self.expect(TokenKind::Ident, expected_ident_span)?;
            let name = self.ctx.strings.intern(&self.source[first.span.range()]);
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
        let name = self
            .ctx
            .strings
            .intern(&self.source[name_tok.span.range()]);
        let name_ident = Ident {
            name,
            span: name_tok.span,
        };

        let colon_tok = self.expect(TokenKind::Colon, name_tok.span)?;
        let ty_id = self.parse_type(colon_tok.span)?;
        let ty_span = self.ctx.types.get(ty_id).span;
        let eq_tok = self.expect(TokenKind::Assign, ty_span)?;
        let expr_id = self.parse_literal_expr(eq_tok.span)?;
        let expr_span = self.ctx.exprs.get(expr_id).span;
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
        Ok(self.ctx.items.alloc(item))
    }

    fn parse_type(&mut self, prev_span: Span) -> PResult<TypeId> {
        let path = self.parse_path(prev_span)?;
        let span = match (path.0.first(), path.0.last()) {
            (Some(first), Some(last)) => first.span.merge(last.span),
            _ => prev_span,
        };
        let ty = Type {
            kind: TypeKind::Path(path),
            span,
        };
        Ok(self.ctx.types.alloc(ty))
    }

    fn parse_literal_expr(&mut self, prev_span: Span) -> PResult<ExprId> {
        let token = self
            .lexer
            .next()
            .ok_or_else(|| ParseError::unexpected_eof(None, prev_span))?;
        if token.kind == TokenKind::Eof {
            return Err(ParseError::unexpected_eof(None, prev_span));
        }

        let lit_kind = match token.kind {
            TokenKind::Integer => LitKind::Int,
            TokenKind::Float => LitKind::Float,
            TokenKind::KwTrue => LitKind::Bool(true),
            TokenKind::KwFalse => LitKind::Bool(false),
            TokenKind::Char => LitKind::Char,
            TokenKind::Str => LitKind::Str,
            _ => {
                return Err(ParseError::expected_expr(token.kind, token.span));
            }
        };

        let span = token.span;
        let lit = Lit {
            kind: lit_kind,
            span,
        };
        let expr = Expr {
            kind: ExprKind::Literal(lit),
            span,
        };
        Ok(self.ctx.exprs.alloc(expr))
    }
}

#[cfg(test)]
mod tests {
    use ast::{
        ctx::AstCtx,
        expr::{ExprKind, LitKind},
        item::ItemKind,
        span::FileId,
        ty::TypeKind,
    };
    use lexer::token::TokenKind;

    use super::Parser;
    use crate::error::ParseErrorKind;

    fn parse(
        source: &str,
    ) -> (
        AstCtx,
        Result<Vec<ast::item::ItemId>, crate::error::ParseError>,
    ) {
        let mut ctx = AstCtx::new();
        let mut parser = Parser::new(&mut ctx, source, FileId(0));
        let result = parser.parse_program();
        (ctx, result)
    }

    #[test]
    fn parses_single_segment_import() {
        let source = "import std";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept a simple import");
        assert_eq!(items.len(), 1);

        let item = ctx.items.get(items[0]);
        assert_eq!(item.span.start, 0);
        assert_eq!(item.span.end, source.len());

        let ItemKind::Import(def) = &item.kind else {
            panic!("expected import");
        };
        assert_eq!(def.path.0.len(), 1);
        assert_eq!(ctx.strings.lookup(def.path.0[0].name), "std");
        assert_eq!(def.path.0[0].span.start, 7);
        assert_eq!(def.path.0[0].span.end, 10);
    }

    #[test]
    fn parses_multi_segment_import_and_merges_item_span() {
        let source = "import std::Option";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept multi-segment imports");
        assert_eq!(items.len(), 1);

        let item = ctx.items.get(items[0]);
        assert_eq!(item.span.start, 0);
        assert_eq!(item.span.end, source.len());

        let ItemKind::Import(def) = &item.kind else {
            panic!("expected import");
        };
        assert_eq!(def.path.0.len(), 2);
        assert_eq!(ctx.strings.lookup(def.path.0[0].name), "std");
        assert_eq!(ctx.strings.lookup(def.path.0[1].name), "Option");
        assert_eq!(def.path.0[0].span.start, 7);
        assert_eq!(def.path.0[0].span.end, 10);
        assert_eq!(def.path.0[1].span.start, 12);
        assert_eq!(def.path.0[1].span.end, 18);
    }

    #[test]
    fn errors_on_missing_import_path() {
        let (_ctx, result) = parse("import");
        let err = result.expect_err("missing path should fail");
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
        let (_ctx, result) = parse("import std::");
        let err = result.expect_err("trailing :: should fail");
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
        let (_ctx, result) = parse("import std::1");
        let err = result.expect_err("expected ident after ::");
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
        let source = "const PI: f64 = 3.14";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept const float");
        assert_eq!(items.len(), 1);

        let item = ctx.items.get(items[0]);
        assert_eq!(item.span.start, 0);
        assert_eq!(item.span.end, source.len());

        let ItemKind::Const(def) = &item.kind else {
            panic!("expected const");
        };
        assert_eq!(ctx.strings.lookup(def.name.name), "PI");
        let ty = ctx.types.get(def.ty);
        let TypeKind::Path(path) = &ty.kind;
        assert_eq!(path.0.len(), 1);
        assert_eq!(ctx.strings.lookup(path.0[0].name), "f64");
        let expr = ctx.exprs.get(def.value);
        let ExprKind::Literal(lit) = &expr.kind;
        assert!(matches!(lit.kind, LitKind::Float));
        assert_eq!(&source[lit.span.range()], "3.14");
    }

    #[test]
    fn parses_const_str() {
        let source = r#"const NAME: str = "abc""#;
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept const str");
        assert_eq!(items.len(), 1);

        let item = ctx.items.get(items[0]);
        let ItemKind::Const(def) = &item.kind else {
            panic!("expected const");
        };
        assert_eq!(ctx.strings.lookup(def.name.name), "NAME");
        let ty = ctx.types.get(def.ty);
        let TypeKind::Path(path) = &ty.kind;
        assert_eq!(path.0.len(), 1);
        assert_eq!(ctx.strings.lookup(path.0[0].name), "str");
        let expr = ctx.exprs.get(def.value);
        let ExprKind::Literal(lit) = &expr.kind;
        assert!(matches!(lit.kind, LitKind::Str));
        let quote = source.find('"').expect("opening quote");
        assert_eq!(lit.span.start, quote);
        assert_eq!(lit.span.end, source.len());
    }

    #[test]
    fn parses_const_bool_true() {
        let source = "const FLAG: bool = true";
        let (ctx, result) = parse(source);
        let items = result.expect("parser should accept const bool");
        assert_eq!(items.len(), 1);

        let item = ctx.items.get(items[0]);
        let ItemKind::Const(def) = &item.kind else {
            panic!("expected const");
        };
        assert_eq!(ctx.strings.lookup(def.name.name), "FLAG");
        let ty = ctx.types.get(def.ty);
        let TypeKind::Path(path) = &ty.kind;
        assert_eq!(path.0.len(), 1);
        assert_eq!(ctx.strings.lookup(path.0[0].name), "bool");
        let expr = ctx.exprs.get(def.value);
        let ExprKind::Literal(lit) = &expr.kind;
        assert_eq!(lit.kind, LitKind::Bool(true));
    }

    #[test]
    fn parses_mixed_consts_and_import() {
        let source = "const A: i32 = 1\nimport std\nconst B: str = \"x\"";
        let (ctx, result) = parse(source);
        let items = result.expect("mixed program");
        assert_eq!(items.len(), 3);

        let ItemKind::Const(a) = &ctx.items.get(items[0]).kind else {
            panic!("expected const A");
        };
        assert_eq!(ctx.strings.lookup(a.name.name), "A");

        let ItemKind::Import(imp) = &ctx.items.get(items[1]).kind else {
            panic!("expected import");
        };
        assert_eq!(imp.path.0.len(), 1);
        assert_eq!(ctx.strings.lookup(imp.path.0[0].name), "std");

        let ItemKind::Const(b) = &ctx.items.get(items[2]).kind else {
            panic!("expected const B");
        };
        assert_eq!(ctx.strings.lookup(b.name.name), "B");
        let expr = ctx.exprs.get(b.value);
        let ExprKind::Literal(lit) = &expr.kind;
        assert!(matches!(lit.kind, LitKind::Str));
    }

    #[test]
    fn errors_on_const_missing_name() {
        let (_ctx, result) = parse("const");
        let err = result.expect_err("missing name");
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
        let (_ctx, result) = parse("const X");
        let err = result.expect_err("missing colon");
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
        let (_ctx, result) = parse("const X:");
        let err = result.expect_err("missing type");
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
        let (_ctx, result) = parse("const X: T");
        let err = result.expect_err("missing assign");
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
        let (_ctx, result) = parse("const X: T =");
        let err = result.expect_err("missing rhs");
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
        let (_ctx, result) = parse("const X: T = +");
        let err = result.expect_err("non-literal rhs");
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
        let (_ctx, result) = parse("const 1: T = 0");
        let err = result.expect_err("integer as name");
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
        let items = result.expect("multi-segment type path");
        assert_eq!(items.len(), 1);

        let item = ctx.items.get(items[0]);
        let ItemKind::Const(def) = &item.kind else {
            panic!("expected const");
        };
        let ty = ctx.types.get(def.ty);
        let TypeKind::Path(path) = &ty.kind;
        assert_eq!(path.0.len(), 2);
        assert_eq!(ctx.strings.lookup(path.0[0].name), "std");
        assert_eq!(ctx.strings.lookup(path.0[1].name), "Option");
        let type_snippet = "std::Option";
        let start = source.find(type_snippet).expect("type snippet");
        assert_eq!(ty.span.start, start);
        assert_eq!(ty.span.end, start + type_snippet.len());
        assert_eq!(&source[ty.span.range()], type_snippet);
    }

    #[test]
    fn errors_on_const_trailing_colon_colon_in_type() {
        let (_ctx, result) = parse("const X: std:: = 1");
        let err = result.expect_err("trailing :: in type");
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
