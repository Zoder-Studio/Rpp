//! Semantic analysis for R++ / Rpp.
//!
//! This stage walks the AST produced by the parser and checks:
//!
//! - **Scope resolution**: every variable/function reference must resolve
//!   to something declared in an enclosing scope.
//! - **`create.sys` registration** (section 4 of the spec): a call to a
//!   name that is not a declared function must have been registered via
//!   `create.sys` in an enclosing scope, or it is a compile-time error —
//!   exactly the diagnostic shown in the spec's own example.
//! - **Immutability** (section 7): assigning to a name declared with
//!   `const let` is a compile-time error.
//! - **`close` / `return` placement**: `close` is only valid inside a
//!   `while`/`for` loop; `return` is only valid inside a function body or
//!   `main start`.
//! - **Call arity**: a call to a known local function must pass exactly as
//!   many arguments as it declares parameters.
//!
//! ## Design notes on parts the spec does not fully lock down (PROPOSED)
//!
//! - **Hoisting**: function declarations and `create.sys` registrations in
//!   a given scope are visible throughout that whole scope regardless of
//!   their line position (a two-pass collect-then-check per scope), while
//!   `let`/`const let` variables must be declared before use (sequential).
//!   The spec frames `create.sys`/`def` around *scope*, not *line order*,
//!   so this is the most defensible reading, but it has not been
//!   explicitly confirmed.
//! - **Shadowing**: re-declaring the same variable name with `let`/`const
//!   let` in the same scope is allowed (Rust-style shadowing), since the
//!   spec explicitly draws on Rust ergonomics and never states otherwise.
//!   Re-declaring a *function* with the same name in the same scope IS
//!   flagged as an error, since that is a near-universal compile error and
//!   none of the spec's examples rely on redefining a function.
//! - **`uDef`'s special "listen to global scope" behavior** (section 6) is
//!   NOT YET implemented here — a `uDef` function is currently checked
//!   with the same scoping rules as a local `def` function. This is a
//!   known, explicit gap, not a silent one.
//! - String interpolation (`${...}`) contents are not yet re-parsed and
//!   checked by this pass (the lexer/parser already fully support it; this
//!   analyzer just doesn't recurse into it yet).

use std::collections::{HashMap, HashSet};

use crate::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub message: String,
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Default)]
struct Scope {
    functions: HashMap<String, usize>,
    variables: HashMap<String, bool>, // name -> is_const
    systems: HashSet<String>,
}

struct Analyzer {
    scopes: Vec<Scope>,
    errors: Vec<SemanticError>,
}

impl Analyzer {
    fn new() -> Self {
        Analyzer {
            scopes: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn error(&mut self, message: impl Into<String>) {
        self.errors.push(SemanticError {
            message: message.into(),
        });
    }

    fn declare_function(&mut self, name: &str, param_count: usize) {
        let scope = self.scopes.last_mut().expect("at least one scope");
        if scope.functions.contains_key(name) {
            self.errors.push(SemanticError {
                message: format!("function '{name}' is already declared in this scope"),
            });
            return;
        }
        scope.functions.insert(name.to_string(), param_count);
    }

    /// Shadowing is allowed: re-declaring the same name with `let`/`const
    /// let` in the same scope simply replaces the previous binding.
    fn declare_variable(&mut self, name: &str, is_const: bool) {
        let scope = self.scopes.last_mut().expect("at least one scope");
        scope.variables.insert(name.to_string(), is_const);
    }

    fn register_system(&mut self, name: &str) {
        let scope = self.scopes.last_mut().expect("at least one scope");
        scope.systems.insert(name.to_string());
    }

    /// Returns `Some(is_const)` for the innermost matching declaration.
    fn resolve_variable(&self, name: &str) -> Option<bool> {
        for scope in self.scopes.iter().rev() {
            if let Some(is_const) = scope.variables.get(name) {
                return Some(*is_const);
            }
        }
        None
    }

    fn resolve_function(&self, name: &str) -> Option<usize> {
        for scope in self.scopes.iter().rev() {
            if let Some(count) = scope.functions.get(name) {
                return Some(*count);
            }
        }
        None
    }

    fn is_system_registered(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .any(|s| s.systems.contains(name) || s.systems.contains("*"))
    }

    // ---- top-level driver ----

    fn analyze_program(&mut self, program: &Program) {
        self.push_scope(); // global scope

        // Pass A: hoist global function declarations and create.sys registrations.
        for item in &program.items {
            match item {
                Item::Function(f) => self.declare_function(&f.name, f.params.len()),
                Item::CreateSys(name) => self.register_system(name),
                _ => {}
            }
        }

        // Pass B: check bodies / statements in source order.
        for item in &program.items {
            match item {
                Item::Include(_) | Item::CreateSys(_) => {}
                Item::Function(f) => self.check_function(f),
                Item::Main(block) => {
                    self.push_scope();
                    self.check_block(block, false, true);
                    self.pop_scope();
                }
                Item::TopLevel(stmt) => {
                    // Outside `main start` and outside any function: neither
                    // `close` nor `return` is valid here.
                    self.check_stmt(stmt, false, false);
                }
            }
        }

        self.pop_scope();
    }

    fn check_function(&mut self, f: &FunctionDecl) {
        self.push_scope();
        for p in &f.params {
            self.declare_variable(&p.name, false);
        }
        self.check_block(&f.body, false, true);
        self.pop_scope();
    }

    fn check_block(&mut self, block: &Block, in_loop: bool, in_function: bool) {
        self.push_scope();

        // Hoist local function decls / create.sys registrations for this block.
        for stmt in &block.stmts {
            match stmt {
                Stmt::Function(f) => self.declare_function(&f.name, f.params.len()),
                Stmt::CreateSys(name) => self.register_system(name),
                _ => {}
            }
        }

        for stmt in &block.stmts {
            self.check_stmt(stmt, in_loop, in_function);
        }

        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &Stmt, in_loop: bool, in_function: bool) {
        match stmt {
            Stmt::Let {
                is_const,
                name,
                value,
                ..
            } => {
                self.check_expr(value);
                self.declare_variable(name, *is_const);
            }
            Stmt::Assign { name, value } => {
                self.check_expr(value);
                match self.resolve_variable(name) {
                    None => self.error(format!("undefined variable '{name}'")),
                    Some(true) => self.error(format!(
                        "cannot assign to '{name}': it was declared with 'const let' (immutable)"
                    )),
                    Some(false) => {}
                }
            }
            Stmt::ExprStmt(e) => self.check_expr(e),
            Stmt::If {
                branches,
                else_branch,
            } => {
                for (cond, body) in branches {
                    self.check_expr(cond);
                    self.check_block(body, in_loop, in_function);
                }
                if let Some(b) = else_branch {
                    self.check_block(b, in_loop, in_function);
                }
            }
            Stmt::While { condition, body } => {
                self.check_expr(condition);
                self.check_block(body, true, in_function);
            }
            Stmt::For {
                var,
                iterable,
                body,
            } => {
                self.check_expr(iterable);
                self.push_scope();
                self.declare_variable(var, false);
                // Hoist local function decls / create.sys registrations for
                // this loop body, same as check_block does.
                for stmt in &body.stmts {
                    match stmt {
                        Stmt::Function(f) => self.declare_function(&f.name, f.params.len()),
                        Stmt::CreateSys(name) => self.register_system(name),
                        _ => {}
                    }
                }
                for stmt in &body.stmts {
                    self.check_stmt(stmt, true, in_function);
                }
                self.pop_scope();
            }
            Stmt::Return(opt) => {
                if !in_function {
                    self.error("'return' used outside of a function or 'main start'");
                }
                if let Some(e) = opt {
                    self.check_expr(e);
                }
            }
            Stmt::Close => {
                if !in_loop {
                    self.error("'close' used outside of a loop ('while'/'for')");
                }
            }
            Stmt::CreateSys(_) => {
                // Already hoisted at the top of `check_block`/`analyze_program`.
            }
            Stmt::Then { condition, body } => {
                self.check_expr(condition);
                self.check_block(body, in_loop, in_function);
            }
            Stmt::Function(f) => {
                // Already declared during hoisting; now check its body.
                self.check_function(f);
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Int(_)
            | Expr::Float(_)
            | Expr::Char(_)
            | Expr::Bool(_)
            | Expr::None_
            | Expr::Str(_) => {}
            Expr::Ident(name) => {
                if self.resolve_variable(name).is_none() {
                    self.error(format!("undefined variable '{name}'"));
                }
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::Unary { expr, .. } => self.check_expr(expr),
            Expr::Call { callee, args } => {
                match self.resolve_function(callee) {
                    Some(param_count) => {
                        if param_count != args.len() {
                            self.error(format!(
                                "function '{callee}' expects {param_count} argument(s), found {}",
                                args.len()
                            ));
                        }
                    }
                    None => {
                        if !self.is_system_registered(callee) {
                            self.error(format!(
                                "'{callee}' is neither a declared function nor a system command registered with create.sys"
                            ));
                        }
                    }
                }
                for a in args {
                    self.check_expr(a);
                }
            }
            Expr::Run(inner) => self.check_expr(inner),
            Expr::Array(items) => {
                for it in items {
                    self.check_expr(it);
                }
            }
        }
    }
}

/// Runs semantic analysis on a parsed `Program`. Returns every error found
/// (not just the first), so a single run reports as much as possible.
pub fn analyze(program: &Program) -> Result<(), Vec<SemanticError>> {
    let mut analyzer = Analyzer::new();
    analyzer.analyze_program(program);
    if analyzer.errors.is_empty() {
        Ok(())
    } else {
        Err(analyzer.errors)
    }
}
