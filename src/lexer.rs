//! Lexer for the R++ / Rpp language.
//!
//! Covers (per current locked specification):
//! - identifiers / keywords
//! - integer & float literals
//! - char literals 'x'
//! - string literals "...", including:
//!     - raw newlines preserved literally inside "..." (FINAL requirement)
//!     - triple-quoted multiline strings """ ... """
//!     - ${expr} interpolation, recursively lexed
//! - single-line comments: "/ comment text"
//! - multiline comments: /" ... "/
//! - punctuation & operators (including two-char lookahead tokens)

use crate::token::{lookup_keyword, StringPart, Token, TokenKind};

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error at {}:{}: {}", self.line, self.col, self.message)
    }
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let (line, col) = (self.line, self.col);
            let Some(c) = self.peek() else {
                tokens.push(Token::new(TokenKind::Eof, line, col));
                break;
            };

            let kind = if c.is_ascii_digit() {
                self.scan_number()?
            } else if c == '"' {
                self.scan_string()?
            } else if c == '\'' {
                self.scan_char()?
            } else if is_ident_start(c) {
                self.scan_ident_or_keyword()
            } else {
                self.scan_operator()?
            };

            tokens.push(Token::new(kind, line, col));
        }
        Ok(tokens)
    }

    // ---- low level cursor helpers ----

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn error(&self, message: impl Into<String>) -> LexError {
        LexError {
            message: message.into(),
            line: self.line,
            col: self.col,
        }
    }

    // ---- whitespace & comments ----

    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('"') => {
                    // multiline comment: /" ... "/
                    self.advance(); // '/'
                    self.advance(); // '"'
                    loop {
                        match self.peek() {
                            None => return Err(self.error("unterminated multiline comment")),
                            Some('"') if self.peek_at(1) == Some('/') => {
                                self.advance();
                                self.advance();
                                break;
                            }
                            Some(_) => {
                                self.advance();
                            }
                        }
                    }
                }
                Some('/') => {
                    // single-line comment: consumes to end of line
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    // ---- literals ----

    fn scan_number(&mut self) -> Result<TokenKind, LexError> {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') && self.peek_at(1).map(|c| c.is_ascii_digit()).unwrap_or(false)
        {
            text.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    text.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            let value: f64 = text
                .parse()
                .map_err(|_| self.error(format!("invalid float literal '{text}'")))?;
            return Ok(TokenKind::Float(value));
        }
        let value: i64 = text
            .parse()
            .map_err(|_| self.error(format!("invalid integer literal '{text}'")))?;
        Ok(TokenKind::Int(value))
    }

    fn scan_char(&mut self) -> Result<TokenKind, LexError> {
        self.advance(); // opening '
        let c = match self.advance() {
            Some('\\') => self.scan_escape()?,
            Some(c) => c,
            None => return Err(self.error("unterminated char literal")),
        };
        match self.advance() {
            Some('\'') => Ok(TokenKind::Char(c)),
            _ => Err(self.error("char literal must contain exactly one character")),
        }
    }

    fn scan_escape(&mut self) -> Result<char, LexError> {
        match self.advance() {
            Some('n') => Ok('\n'),
            Some('t') => Ok('\t'),
            Some('r') => Ok('\r'),
            Some('\\') => Ok('\\'),
            Some('"') => Ok('"'),
            Some('\'') => Ok('\''),
            Some('0') => Ok('\0'),
            Some('$') => Ok('$'),
            Some(other) => Ok(other),
            None => Err(self.error("unterminated escape sequence")),
        }
    }

    fn scan_string(&mut self) -> Result<TokenKind, LexError> {
        // Detect triple-quote multiline string """ ... """
        if self.peek_at(1) == Some('"') && self.peek_at(2) == Some('"') {
            self.advance();
            self.advance();
            self.advance();
            let parts = self.scan_string_parts(true)?;
            return Ok(TokenKind::Str(parts));
        }
        self.advance(); // opening "
        let parts = self.scan_string_parts(false)?;
        Ok(TokenKind::Str(parts))
    }

    /// Scans the body of a string literal until its closing delimiter.
    /// `triple` selects `"""` as the closing delimiter instead of a single `"`.
    /// Raw newlines inside the body are preserved as part of the literal value
    /// (this is a FINAL requirement: a bare `"..."` string may legally contain
    /// literal newlines without that being a syntax error).
    fn scan_string_parts(&mut self, triple: bool) -> Result<Vec<StringPart>, LexError> {
        let mut parts = Vec::new();
        let mut current = String::new();

        loop {
            if triple {
                if self.peek() == Some('"') && self.peek_at(1) == Some('"') && self.peek_at(2) == Some('"') {
                    self.advance();
                    self.advance();
                    self.advance();
                    break;
                }
            } else if self.peek() == Some('"') {
                self.advance();
                break;
            }

            match self.peek() {
                None => return Err(self.error("unterminated string literal")),
                Some('\\') => {
                    self.advance();
                    current.push(self.scan_escape()?);
                }
                Some('$') if self.peek_at(1) == Some('{') => {
                    if !current.is_empty() {
                        parts.push(StringPart::Literal(std::mem::take(&mut current)));
                    }
                    self.advance(); // $
                    self.advance(); // {
                    let inner = self.scan_interpolation_body()?;
                    let tokens = Lexer::new(&inner).tokenize().map_err(|e| {
                        self.error(format!("invalid interpolation expression: {e}"))
                    })?;
                    let tokens: Vec<Token> = tokens
                        .into_iter()
                        .filter(|t| t.kind != TokenKind::Eof)
                        .collect();
                    parts.push(StringPart::Interpolation(tokens));
                }
                Some(c) => {
                    // Newline (and any other char) is preserved literally.
                    current.push(c);
                    self.advance();
                }
            }
        }

        if !current.is_empty() || parts.is_empty() {
            parts.push(StringPart::Literal(current));
        }
        Ok(parts)
    }

    /// Consumes raw source text up to the matching `}` of a `${ ... }`
    /// interpolation, honoring nested braces.
    fn scan_interpolation_body(&mut self) -> Result<String, LexError> {
        let mut depth: i32 = 1;
        let mut body = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated interpolation")),
                Some('{') => {
                    depth += 1;
                    body.push('{');
                    self.advance();
                }
                Some('}') => {
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                    body.push('}');
                }
                Some(c) => {
                    body.push(c);
                    self.advance();
                }
            }
        }
        Ok(body)
    }

    fn scan_ident_or_keyword(&mut self) -> TokenKind {
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if is_ident_continue(c) {
                text.push(c);
                self.advance();
            } else {
                break;
            }
        }
        lookup_keyword(&text).unwrap_or(TokenKind::Ident(text))
    }

    fn scan_operator(&mut self) -> Result<TokenKind, LexError> {
        let c = self.advance().expect("checked by caller");
        let kind = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            '?' => TokenKind::Question,
            '@' => TokenKind::At,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '%' => TokenKind::Percent,
            '=' => {
                if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::FatEquals
                } else {
                    TokenKind::Equals
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEquals
                } else {
                    TokenKind::Bang
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Gte
                } else {
                    TokenKind::Gt
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Lte
                } else {
                    TokenKind::Lt
                }
            }
            other => return Err(self.error(format!("unexpected character '{other}'"))),
        };
        Ok(kind)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
