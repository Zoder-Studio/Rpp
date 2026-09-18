//! `rpp` CLI entry point.
//!
//! `rpp <file.rpp>` lexes, parses, and semantically analyzes the given
//! source file, printing the resulting AST or a diagnostic on failure. No
//! type checking, borrow checking, or code generation has been implemented
//! yet.

use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: rpp <file.rpp>");
        return ExitCode::FAILURE;
    }

    let path = &args[1];
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read '{path}': {e}");
            return ExitCode::FAILURE;
        }
    };

    match rpp_compiler::check_source(&source) {
        Ok(program) => {
            println!("{program:#?}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
