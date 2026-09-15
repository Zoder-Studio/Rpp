//! R++ / Rpp compiler foundation crate.
//!
//! Current stage: lexer, parser, and AST. Semantic analysis, type checking,
//! borrow checking, and code-generation backends are not implemented yet.

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;

use lexer::{LexError, Lexer};
use parser::{ParseError, Parser};

#[derive(Debug)]
pub enum RppError {
    Lex(LexError),
    Parse(ParseError),
}

impl std::fmt::Display for RppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RppError::Lex(e) => write!(f, "{e}"),
            RppError::Parse(e) => write!(f, "{e}"),
        }
    }
}

/// Lexes and parses a full R++ / Rpp source file into a `Program` AST.
pub fn parse_source(source: &str) -> Result<ast::Program, RppError> {
    let tokens = Lexer::new(source).tokenize().map_err(RppError::Lex)?;
    Parser::new(tokens).parse_program().map_err(RppError::Parse)
}
