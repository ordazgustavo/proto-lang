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
        if token.kind == kind {
            Ok(token)
        } else {
            Err(ParseError::expected(kind, token.kind, token.span))
        }
    }

    fn parse_import_def(&mut self, span: Span) -> PResult<ItemId> {
        let path = self.parse_path(span)?;
        let def = ImportDef { path };
        let item = Item {
            kind: ItemKind::Import(def),
            span,
        };
        Ok(self.ctx.items.alloc(item))
    }

    fn parse_path(&mut self, span: Span) -> PResult<ModPath> {
        let mut paths = Vec::new(); // also was missing `mut`

        loop {
            let first = self.expect(TokenKind::Ident, span)?;
            let name = self.ctx.strings.intern(&self.source[first.span.range()]);
            paths.push(Ident {
                name,
                span: first.span,
            });

            if self
                .lexer
                .peek()
                .map_or(false, |t| t.kind == TokenKind::ColonColon)
            {
                self.lexer.next(); // consume `::`
            } else {
                break;
            }
        }
        Ok(ModPath(paths))
    }
}
