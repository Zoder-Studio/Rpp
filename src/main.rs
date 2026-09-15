//! `rpp` CLI entry point.
//!
//! Current stage: `rpp <file.rpp>` lexes and parses the given source file
//! and prints the resulting AST (or a diagnostic on failure). This is a
//! foundation build only — no semantic analysis, type checking, or code
//! generation has been implemented yet.

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

    match rpp_compiler::parse_source(&source) {
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
