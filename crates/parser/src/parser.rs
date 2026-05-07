use std::iter::Peekable;

use ast::{
    common::{Ident, ModPath},
    ctx::AstCtx,
    item::{ImportDef, Item, ItemId, ItemKind},
    span::{FileId, Span},
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
}

#[cfg(test)]
mod tests {
    use ast::{ctx::AstCtx, item::ItemKind, span::FileId};
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

        let ItemKind::Import(def) = &item.kind;
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

        let ItemKind::Import(def) = &item.kind;
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
}
