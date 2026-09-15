//! Abstract Syntax Tree for R++ / Rpp.

use crate::token::StringPart;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Top-level items in source order. Any bare statement that is not a
    /// declaration (function/main/create.sys/add) is a "Top-Level System"
    /// statement per the current locked specification: it must run outside
    /// `main start` and outside any `def`/`def emu`/`uDef` function body.
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Include(IncludeDecl),
    CreateSys(String),
    Function(FunctionDecl),
    Main(Block),
    /// A statement written outside `main start` and outside any function —
    /// i.e. a Top-Level System statement.
    TopLevel(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IncludeDecl {
    /// `add utils.rpp;` -> path = "utils.rpp", alias_from = None
    /// `add math from "math";` -> path = "math", alias_from = Some("math")
    pub path: String,
    pub from: Option<String>,
}

/// Function declaration keyword forms.
///
/// Per current locked specification:
/// - `def` is the general function declaration keyword, but writing `def`
///   is OPTIONAL: `name(params) = { ... }` alone is also a valid function
///   declaration.
/// - `emu` MAY optionally follow `def` (`def emu name(...) = {...}`); it is
///   NOT mandatory for ordinary functions.
/// - `uDef` remains its own distinct function keyword (local function that
///   can access/listen to global `def` scope), separate from `def`/`def emu`.
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionKeyword {
    /// Declared with (or without) `def`; `emu` flag records whether the
    /// optional `emu` modifier was present.
    Def { emu: bool },
    /// Declared with `uDef`.
    UDef,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub keyword: FunctionKeyword,
    pub name: String,
    pub params: Vec<Param>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeRef {
    pub name: String,
    pub generics: Vec<TypeRef>,
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        is_const: bool,
        name: String,
        ty: Option<TypeRef>,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    ExprStmt(Expr),
    If {
        branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
    Return(Option<Expr>),
    Close,
    CreateSys(String),
    /// `condition then { ... }` / `condition then expr;`
    Then {
        condition: Expr,
        body: Block,
    },
    /// A local (nested) function declaration. `def`, `def emu`, `uDef`, and
    /// the bare (no-keyword) form are all allowed both globally and locally.
    Function(FunctionDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(Vec<StringPart>),
    Char(char),
    Bool(bool),
    None_,
    Ident(String),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    /// `run <call-expr>`
    Run(Box<Expr>),
    Array(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Is,       // ==
    IsNot,    // !=
    And,
    Or,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}
