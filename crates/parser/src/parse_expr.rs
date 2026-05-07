use ast::{
    common::Ident,
    expr::{
        Arg, BinaryExpr, BinaryOp, CallExpr, Expr, ExprId, ExprKind, FieldExpr, Lit, LitKind,
        UnaryExpr, UnaryOp,
    },
    ty::TypeId,
};
use lexer::token::{Token, TokenKind};

use crate::{PResult, ParseError, Parser};

impl<'a> Parser<'a> {
    pub(crate) fn parse_expr(&mut self) -> PResult<ExprId> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> PResult<ExprId> {
        let mut lhs = self.parse_prefix()?;

        loop {
            lhs = match self.lexer.peek().map(|token| token.kind) {
                Some(TokenKind::Dot) => self.parse_dot_postfix(lhs)?,
                Some(TokenKind::ColonColon | TokenKind::LParen) => self.parse_call_postfix(lhs)?,
                Some(kind) => {
                    let Some((op, left_bp, right_bp)) = infix_bp(kind) else {
                        break;
                    };
                    if left_bp < min_bp {
                        break;
                    }
                    self.lexer.next();
                    let rhs = self.parse_expr_bp(right_bp)?;
                    let span = self
                        .ctx
                        .get_expr(lhs)
                        .span
                        .merge(self.ctx.get_expr(rhs).span);
                    self.ctx.alloc_expr(Expr {
                        kind: ExprKind::Binary(BinaryExpr { op, lhs, rhs }),
                        span,
                    })
                }
                None => break,
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> PResult<ExprId> {
        let token = self
            .lexer
            .next()
            .ok_or_else(|| ParseError::unexpected_eof(None, self.eof_span()))?;

        match token.kind {
            TokenKind::Minus | TokenKind::Bang => {
                let op = match token.kind {
                    TokenKind::Minus => UnaryOp::Neg,
                    TokenKind::Bang => UnaryOp::Not,
                    _ => unreachable!(),
                };
                let expr = self.parse_expr_bp(7)?;
                let span = token.span.merge(self.ctx.get_expr(expr).span);
                Ok(self.ctx.alloc_expr(Expr {
                    kind: ExprKind::Unary(UnaryExpr { op, expr }),
                    span,
                }))
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen, self.ctx.get_expr(expr).span)?;
                Ok(expr)
            }
            TokenKind::Ident | TokenKind::KwSelf => {
                let ident = self.ident_from_token(&token);
                Ok(self.ctx.alloc_expr(Expr {
                    kind: ExprKind::Path(ident),
                    span: token.span,
                }))
            }
            TokenKind::Integer
            | TokenKind::Float
            | TokenKind::KwTrue
            | TokenKind::KwFalse
            | TokenKind::Char
            | TokenKind::Str => Ok(self.parse_literal_from_token(token)),
            TokenKind::Eof => Err(ParseError::unexpected_eof(None, token.span)),
            _ => Err(ParseError::expected_expr(token.kind, token.span)),
        }
    }

    fn parse_dot_postfix(&mut self, lhs: ExprId) -> PResult<ExprId> {
        self.lexer.next();
        let field = self.expect(TokenKind::Ident, self.ctx.get_expr(lhs).span)?;
        let field = self.ident_from_token(&field);
        let field_span = field.span;
        let base = self.ctx.alloc_expr(Expr {
            kind: ExprKind::Field(FieldExpr { base: lhs, field }),
            span: self.ctx.get_expr(lhs).span.merge(field_span),
        });

        if matches!(
            self.lexer.peek().map(|token| token.kind),
            Some(TokenKind::ColonColon | TokenKind::LParen)
        ) {
            self.parse_call_postfix(base)
        } else {
            Ok(base)
        }
    }

    fn parse_call_postfix(&mut self, callee: ExprId) -> PResult<ExprId> {
        let generic_args = if self
            .lexer
            .peek()
            .is_some_and(|token| token.kind == TokenKind::ColonColon)
        {
            self.parse_turbofish()?
        } else {
            Vec::new()
        };

        let lparen = self.expect(TokenKind::LParen, self.ctx.get_expr(callee).span)?;
        let (args, rparen_span) = self.parse_args(lparen.span)?;
        let span = args
            .last()
            .map(|arg| self.ctx.get_expr(arg.value).span)
            .unwrap_or(lparen.span)
            .merge(self.ctx.get_expr(callee).span);

        Ok(self.ctx.alloc_expr(Expr {
            kind: ExprKind::Call(CallExpr {
                callee,
                generic_args,
                args,
            }),
            span: span.merge(rparen_span),
        }))
    }

    fn parse_turbofish(&mut self) -> PResult<Vec<TypeId>> {
        let eof_span = self.eof_span();
        let colon = self.expect(TokenKind::ColonColon, eof_span)?;
        let lt = self.expect(TokenKind::Lt, colon.span)?;
        let mut args = Vec::new();

        loop {
            args.push(self.parse_type(lt.span)?);
            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::Comma)
                .is_none()
            {
                break;
            }
        }

        self.expect(TokenKind::Gt, lt.span)?;
        Ok(args)
    }

    fn parse_args(&mut self, lparen_span: ast::span::Span) -> PResult<(Vec<Arg>, ast::span::Span)> {
        let mut args = Vec::new();
        if let Some(rparen) = self.lexer.next_if(|token| token.kind == TokenKind::RParen) {
            return Ok((args, rparen.span));
        }

        loop {
            let label = self.parse_arg_label();
            let value = self.parse_expr()?;
            args.push(Arg { label, value });

            if self
                .lexer
                .next_if(|token| token.kind == TokenKind::Comma)
                .is_none()
            {
                break;
            }

            if let Some(rparen) = self.lexer.next_if(|token| token.kind == TokenKind::RParen) {
                return Ok((args, rparen.span));
            }
        }

        let prev = args
            .last()
            .map(|arg| self.ctx.get_expr(arg.value).span)
            .unwrap_or(lparen_span);
        let rparen = self.expect(TokenKind::RParen, prev)?;
        Ok((args, rparen.span))
    }

    fn parse_arg_label(&mut self) -> Option<Ident> {
        let mut lookahead = self.lexer.clone();
        let ident = lookahead.next()?;
        if ident.kind != TokenKind::Ident {
            return None;
        }
        let colon = lookahead.next()?;
        if colon.kind != TokenKind::Colon {
            return None;
        }

        let ident = self.lexer.next().expect("lookahead saw ident");
        self.lexer.next().expect("lookahead saw colon");
        Some(self.ident_from_token(&ident))
    }

    fn parse_literal_from_token(&mut self, token: Token) -> ExprId {
        let kind = match token.kind {
            TokenKind::Integer => LitKind::Int,
            TokenKind::Float => LitKind::Float,
            TokenKind::KwTrue => LitKind::Bool(true),
            TokenKind::KwFalse => LitKind::Bool(false),
            TokenKind::Char => LitKind::Char,
            TokenKind::Str => LitKind::Str,
            _ => unreachable!(),
        };
        self.ctx.alloc_expr(Expr {
            kind: ExprKind::Literal(Lit {
                kind,
                span: token.span,
            }),
            span: token.span,
        })
    }

    fn ident_from_token(&mut self, token: &Token) -> Ident {
        let name = self.ctx.intern_str(&self.source[token.span.range()]);
        Ident {
            name,
            span: token.span,
        }
    }

    fn eof_span(&mut self) -> ast::span::Span {
        self.lexer
            .peek()
            .map(|token| token.span)
            .unwrap_or_else(|| {
                ast::span::Span::new(self.source.len(), self.source.len(), ast::span::FileId(0))
            })
    }
}

fn infix_bp(kind: TokenKind) -> Option<(BinaryOp, u8, u8)> {
    let op = match kind {
        TokenKind::Star => BinaryOp::Mul,
        TokenKind::Slash => BinaryOp::Div,
        TokenKind::Percent => BinaryOp::Rem,
        TokenKind::Plus => BinaryOp::Add,
        TokenKind::Minus => BinaryOp::Sub,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::LtEq => BinaryOp::LtEq,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::GtEq => BinaryOp::GtEq,
        TokenKind::EqEq => BinaryOp::Eq,
        TokenKind::BangEq => BinaryOp::NotEq,
        _ => return None,
    };

    let bp = match kind {
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (5, 6),
        TokenKind::Plus | TokenKind::Minus => (3, 4),
        TokenKind::Lt
        | TokenKind::LtEq
        | TokenKind::Gt
        | TokenKind::GtEq
        | TokenKind::EqEq
        | TokenKind::BangEq => (1, 2),
        _ => unreachable!(),
    };

    Some((op, bp.0, bp.1))
}
