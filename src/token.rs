//! Token definitions for the R++ / Rpp lexer.

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal text content of a string (newlines are preserved as-is).
    Literal(String),
    /// A `${ ... }` interpolation segment, already lexed into tokens.
    Interpolation(Vec<Token>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Ident(String),
    Int(i64),
    Float(f64),
    Str(Vec<StringPart>),
    Char(char),

    // Keywords
    Main,
    Start,
    Def,
    UDef,
    Emu,
    Run,
    Close,
    Return,
    Let,
    Const,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Match,
    Then,
    Is,
    Not,
    And,
    Or,
    Add,
    From,
    Type,
    Enum,
    Interface,
    Impl,
    Public,
    Private,
    Async,
    Await,
    Spawn,
    Defer,
    Timeout,
    Try,
    Catch,
    Assert,
    Test,
    None_,
    True,
    False,
    Self_,
    Create,
    Sys,

    // Punctuation / operators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Question,
    At,
    Arrow,       // =>
    FatEquals,   // ==  (rarely used directly; "is" is the primary alias)
    NotEquals,   // !=
    Equals,      // =
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Gt,
    Gte,
    Lt,
    Lte,
    Bang,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Token { kind, line, col }
    }
}

pub fn lookup_keyword(ident: &str) -> Option<TokenKind> {
    Some(match ident {
        "main" => TokenKind::Main,
        "start" => TokenKind::Start,
        "def" => TokenKind::Def,
        "uDef" => TokenKind::UDef,
        "emu" => TokenKind::Emu,
        "run" => TokenKind::Run,
        "close" => TokenKind::Close,
        "return" => TokenKind::Return,
        "let" => TokenKind::Let,
        "const" => TokenKind::Const,
        "if" => TokenKind::If,
        "elif" => TokenKind::Elif,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "match" => TokenKind::Match,
        "then" => TokenKind::Then,
        "is" => TokenKind::Is,
        "not" => TokenKind::Not,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "add" => TokenKind::Add,
        "from" => TokenKind::From,
        "type" => TokenKind::Type,
        "enum" => TokenKind::Enum,
        "interface" => TokenKind::Interface,
        "impl" => TokenKind::Impl,
        "public" => TokenKind::Public,
        "private" => TokenKind::Private,
        "async" => TokenKind::Async,
        "await" => TokenKind::Await,
        "spawn" => TokenKind::Spawn,
        "defer" => TokenKind::Defer,
        "timeout" => TokenKind::Timeout,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "assert" => TokenKind::Assert,
        "test" => TokenKind::Test,
        "none" => TokenKind::None_,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "self" => TokenKind::Self_,
        "create" => TokenKind::Create,
        "sys" => TokenKind::Sys,
        _ => return None,
    })
}
