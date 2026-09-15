//! Recursive-descent parser for R++ / Rpp.
//!
//! Function declaration rules implemented here (current locked spec):
//! - `def` is optional: `name(params) = { ... }` is a valid function decl.
//! - `emu` is an optional modifier that may follow `def`
//!   (`def emu name(...) = {...}`); it is never required.
//! - `uDef` is its own distinct function keyword, unrelated to `def`/`emu`.

use crate::ast::*;
use crate::token::{Token, TokenKind};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at {}:{}: {}", self.line, self.col, self.message)
    }
}

type PResult<T> = Result<T, ParseError>;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse_program(mut self) -> PResult<Program> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    // ---- cursor helpers ----

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_at(&self, offset: usize) -> &TokenKind {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn here(&self) -> (usize, usize) {
        let t = &self.tokens[self.pos];
        (t.line, t.col)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn match_tok(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            let (line, col) = self.here();
            Err(ParseError {
                message: format!("expected {:?}, found {:?}", kind, self.peek()),
                line,
                col,
            })
        }
    }

    fn expect_ident(&mut self) -> PResult<String> {
        let (line, col) = self.here();
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(ParseError {
                message: format!("expected identifier, found {other:?}"),
                line,
                col,
            }),
        }
    }

    #[allow(dead_code)]
    fn error(&self, message: impl Into<String>) -> ParseError {
        let (line, col) = self.here();
        ParseError {
            message: message.into(),
            line,
            col,
        }
    }

    // ---- top-level items ----

    fn parse_item(&mut self) -> PResult<Item> {
        match self.peek() {
            TokenKind::Add => Ok(Item::Include(self.parse_include()?)),
            TokenKind::Create => {
                let value = self.parse_create_sys_value()?;
                Ok(Item::CreateSys(value))
            }
            TokenKind::Main => Ok(Item::Main(self.parse_main()?)),
            TokenKind::Def | TokenKind::UDef => Ok(Item::Function(self.parse_function_decl()?)),
            TokenKind::Ident(_) if self.looks_like_bare_function_decl() => {
                Ok(Item::Function(self.parse_function_decl()?))
            }
            _ => Ok(Item::TopLevel(self.parse_stmt()?)),
        }
    }

    fn parse_include(&mut self) -> PResult<IncludeDecl> {
        self.expect(TokenKind::Add)?;
        // `add utils.rpp;` -> path is lexed as Ident("utils") Dot Ident("rpp")
        // (since '.' is a separate token). We reassemble the dotted form.
        // `add math from "math";` -> path = "math" (ident), from = Some(lib name)
        let first = self.expect_ident()?;
        let mut path = first;
        while self.match_tok(&TokenKind::Dot) {
            let part = self.expect_ident()?;
            path.push('.');
            path.push_str(&part);
        }
        let from = if self.match_tok(&TokenKind::From) {
            Some(self.expect_string_literal()?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(IncludeDecl { path, from })
    }

    fn expect_string_literal(&mut self) -> PResult<String> {
        let (line, col) = self.here();
        match self.peek().clone() {
            TokenKind::Str(parts) => {
                self.advance();
                Ok(flatten_literal_string(&parts))
            }
            other => Err(ParseError {
                message: format!("expected string literal, found {other:?}"),
                line,
                col,
            }),
        }
    }

    /// Parses the right-hand side of `create.sys = <value>;` and returns the
    /// dotted/asterisk value as a string (e.g. "println", "strip.html", "*").
    fn parse_create_sys_value(&mut self) -> PResult<String> {
        self.expect(TokenKind::Create)?;
        self.expect(TokenKind::Dot)?;
        self.expect(TokenKind::Sys)?;
        self.expect(TokenKind::Equals)?;
        let value = if self.match_tok(&TokenKind::Star) {
            "*".to_string()
        } else {
            let mut value = self.expect_ident()?;
            while self.match_tok(&TokenKind::Dot) {
                let part = self.expect_ident()?;
                value.push('.');
                value.push_str(&part);
            }
            value
        };
        self.expect(TokenKind::Semicolon)?;
        Ok(value)
    }

    fn parse_main(&mut self) -> PResult<Block> {
        self.expect(TokenKind::Main)?;
        self.expect(TokenKind::Start)?;
        self.expect(TokenKind::Equals)?;
        self.parse_block()
    }

    // ---- function declarations ----

    /// Looks ahead from the current `Ident` to see whether it forms a bare
    /// (no `def`/`uDef`) function declaration: `IDENT ( ... ) = {`.
    fn looks_like_bare_function_decl(&self) -> bool {
        if !matches!(self.peek(), TokenKind::Ident(_)) {
            return false;
        }
        if !matches!(self.peek_at(1), TokenKind::LParen) {
            return false;
        }
        let mut i = self.pos + 1; // at LParen
        let mut depth = 0i32;
        loop {
            if i >= self.tokens.len() {
                return false;
            }
            match &self.tokens[i].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        // i is at the matching RParen
        matches!(self.tokens.get(i + 1).map(|t| &t.kind), Some(TokenKind::Equals))
            && matches!(self.tokens.get(i + 2).map(|t| &t.kind), Some(TokenKind::LBrace))
    }

    fn parse_function_decl(&mut self) -> PResult<FunctionDecl> {
        let keyword = if self.match_tok(&TokenKind::UDef) {
            FunctionKeyword::UDef
        } else if self.match_tok(&TokenKind::Def) {
            let emu = self.match_tok(&TokenKind::Emu);
            FunctionKeyword::Def { emu }
        } else {
            // Bare declaration: no `def`/`uDef` keyword written at all.
            FunctionKeyword::Def { emu: false }
        };

        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) {
            let pname = self.expect_ident()?;
            let ty = if self.match_tok(&TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            params.push(Param { name: pname, ty });
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;
        self.expect(TokenKind::Equals)?;
        let body = self.parse_block()?;
        Ok(FunctionDecl {
            keyword,
            name,
            params,
            body,
        })
    }

    fn parse_type(&mut self) -> PResult<TypeRef> {
        let name = self.expect_ident()?;
        let mut generics = Vec::new();
        if self.match_tok(&TokenKind::Lt) {
            loop {
                generics.push(self.parse_type()?);
                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt)?;
        }
        // trailing `[]` marks an array type: wrap as array<name<generics>>
        let mut ty = TypeRef {
            name,
            generics,
            nullable: false,
        };
        while self.check(&TokenKind::LBracket) && matches!(self.peek_at(1), TokenKind::RBracket) {
            self.advance(); // [
            self.advance(); // ]
            ty = TypeRef {
                name: "array".to_string(),
                generics: vec![ty],
                nullable: false,
            };
        }
        if self.match_tok(&TokenKind::Question) {
            ty.nullable = true;
        }
        Ok(ty)
    }

    // ---- blocks & statements ----

    fn parse_block(&mut self) -> PResult<Block> {
        self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Block { stmts })
    }

    /// Parses either a `{ ... }` block or a single statement, used by the
    /// one-line `then` form (e.g. `if score is 100 then println("Perfect");`).
    fn parse_block_or_single_stmt(&mut self) -> PResult<Block> {
        if self.check(&TokenKind::LBrace) {
            self.parse_block()
        } else {
            Ok(Block {
                stmts: vec![self.parse_stmt()?],
            })
        }
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek().clone() {
            TokenKind::Let => self.parse_let(false),
            TokenKind::Const => {
                self.advance();
                self.expect(TokenKind::Let)?;
                self.parse_let(true)
            }
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Return => {
                self.advance();
                if self.match_tok(&TokenKind::Semicolon) {
                    Ok(Stmt::Return(None))
                } else {
                    let e = self.parse_expr()?;
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::Return(Some(e)))
                }
            }
            TokenKind::Close => {
                self.advance();
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Close)
            }
            TokenKind::Create => {
                let value = self.parse_create_sys_value()?;
                Ok(Stmt::CreateSys(value))
            }
            TokenKind::Def | TokenKind::UDef => Ok(Stmt::Function(self.parse_function_decl()?)),
            TokenKind::Ident(ref name)
                if matches!(self.peek_at(1), TokenKind::Equals)
                    && !matches!(self.peek_at(2), TokenKind::LBrace) =>
            {
                let name = name.clone();
                self.advance(); // ident
                self.advance(); // =
                let value = self.parse_expr()?;
                self.expect(TokenKind::Semicolon)?;
                Ok(Stmt::Assign { name, value })
            }
            TokenKind::Ident(_) if self.looks_like_bare_function_decl() => {
                Ok(Stmt::Function(self.parse_function_decl()?))
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.match_tok(&TokenKind::Then) {
                    let body = self.parse_block_or_single_stmt()?;
                    Ok(Stmt::Then {
                        condition: expr,
                        body,
                    })
                } else {
                    self.expect(TokenKind::Semicolon)?;
                    Ok(Stmt::ExprStmt(expr))
                }
            }
        }
    }

    fn parse_let(&mut self, is_const: bool) -> PResult<Stmt> {
        self.expect(TokenKind::Let)?;
        let name = self.expect_ident()?;
        let ty = if self.match_tok(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Equals)?;
        let value = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;
        Ok(Stmt::Let {
            is_const,
            name,
            ty,
            value,
        })
    }

    fn parse_if(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::If)?;
        let cond = self.parse_expr()?;

        if self.match_tok(&TokenKind::Then) {
            let body = self.parse_block_or_single_stmt()?;
            return Ok(Stmt::If {
                branches: vec![(cond, body)],
                else_branch: None,
            });
        }

        let body = self.parse_block()?;
        let mut branches = vec![(cond, body)];
        let mut else_branch = None;
        loop {
            if self.match_tok(&TokenKind::Elif) {
                let c = self.parse_expr()?;
                let b = self.parse_block()?;
                branches.push((c, b));
            } else if self.match_tok(&TokenKind::Else) {
                else_branch = Some(self.parse_block()?);
                break;
            } else {
                break;
            }
        }
        Ok(Stmt::If {
            branches,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> PResult<Stmt> {
        self.expect(TokenKind::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While { condition, body })
    }

    // ---- expressions (precedence climbing) ----

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut left = self.parse_and()?;
        while self.match_tok(&TokenKind::Or) {
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut left = self.parse_equality()?;
        while self.match_tok(&TokenKind::And) {
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> PResult<Expr> {
        let mut left = self.parse_comparison()?;
        loop {
            if self.match_tok(&TokenKind::Is) {
                let op = if self.match_tok(&TokenKind::Not) {
                    BinOp::IsNot
                } else {
                    BinOp::Is
                };
                let right = self.parse_comparison()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            } else if self.match_tok(&TokenKind::FatEquals) {
                let right = self.parse_comparison()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::Is,
                    right: Box::new(right),
                };
            } else if self.match_tok(&TokenKind::NotEquals) {
                let right = self.parse_comparison()?;
                left = Expr::Binary {
                    left: Box::new(left),
                    op: BinOp::IsNot,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> PResult<Expr> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek() {
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Gte => BinOp::Gte,
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Lte => BinOp::Lte,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> PResult<Expr> {
        let mut left = self.parse_factor()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_factor()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> PResult<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if self.match_tok(&TokenKind::Not) || self.match_tok(&TokenKind::Bang) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.match_tok(&TokenKind::Minus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        let (line, col) = self.here();
        match self.peek().clone() {
            TokenKind::Int(v) => {
                self.advance();
                Ok(Expr::Int(v))
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(Expr::Float(v))
            }
            TokenKind::Str(parts) => {
                self.advance();
                Ok(Expr::Str(parts))
            }
            TokenKind::Char(c) => {
                self.advance();
                Ok(Expr::Char(c))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::None_ => {
                self.advance();
                Ok(Expr::None_)
            }
            TokenKind::Run => {
                self.advance();
                let inner = self.parse_unary()?;
                Ok(Expr::Run(Box::new(inner)))
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !self.check(&TokenKind::RBracket) {
                    items.push(self.parse_expr()?);
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::Array(items))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.match_tok(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) {
                        args.push(self.parse_expr()?);
                        if !self.match_tok(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr::Call { callee: name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => Err(ParseError {
                message: format!("unexpected token {other:?} in expression"),
                line,
                col,
            }),
        }
    }
}

fn flatten_literal_string(parts: &[crate::token::StringPart]) -> String {
    parts
        .iter()
        .map(|p| match p {
            crate::token::StringPart::Literal(s) => s.clone(),
            crate::token::StringPart::Interpolation(_) => String::new(),
        })
        .collect()
}
