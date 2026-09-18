use rpp_compiler::ast::*;
use rpp_compiler::parse_source;

#[test]
fn parses_def_function_without_emu() {
    let program = parse_source(
        r#"
        def greet(name: string) = {
            println("Hello, " + name + "!");
        }
        "#,
    )
    .expect("should parse");

    assert_eq!(program.items.len(), 1);
    match &program.items[0] {
        Item::Function(f) => {
            assert_eq!(f.name, "greet");
            assert_eq!(f.keyword, FunctionKeyword::Def { emu: false });
            assert_eq!(f.params.len(), 1);
            assert_eq!(f.params[0].name, "name");
        }
        other => panic!("expected Function item, got {other:?}"),
    }
}

#[test]
fn parses_def_emu_function_as_optional_form() {
    let program = parse_source(
        r#"
        def emu sum_two(a: int, b: int) = {
            return a + b;
        }
        "#,
    )
    .expect("should parse");

    match &program.items[0] {
        Item::Function(f) => {
            assert_eq!(f.keyword, FunctionKeyword::Def { emu: true });
        }
        other => panic!("expected Function item, got {other:?}"),
    }
}

#[test]
fn parses_bare_function_without_def_keyword() {
    // "def" is optional per current spec: a bare `name(...) = { ... }` is
    // also a valid function declaration.
    let program = parse_source(
        r#"
        sum_two(a: int, b: int) = {
            return a + b;
        }
        "#,
    )
    .expect("should parse");

    match &program.items[0] {
        Item::Function(f) => {
            assert_eq!(f.name, "sum_two");
            assert_eq!(f.keyword, FunctionKeyword::Def { emu: false });
        }
        other => panic!("expected Function item, got {other:?}"),
    }
}

#[test]
fn parses_udef_function_as_distinct_keyword() {
    let program = parse_source(
        r#"
        uDef helper(x: int) = {
            return x;
        }
        "#,
    )
    .expect("should parse");

    match &program.items[0] {
        Item::Function(f) => {
            assert_eq!(f.keyword, FunctionKeyword::UDef);
            assert_eq!(f.name, "helper");
        }
        other => panic!("expected Function item, got {other:?}"),
    }
}

#[test]
fn parses_main_start_block() {
    let program = parse_source(
        r#"
        main start = {
            println("Hello World!");
        }
        "#,
    )
    .expect("should parse");

    match &program.items[0] {
        Item::Main(block) => assert_eq!(block.stmts.len(), 1),
        other => panic!("expected Main item, got {other:?}"),
    }
}

#[test]
fn parses_create_sys_dotted_and_star() {
    let program = parse_source(
        r#"
        create.sys = println;
        create.sys = strip.html;
        create.sys = *;
        "#,
    )
    .expect("should parse");

    let values: Vec<&str> = program
        .items
        .iter()
        .map(|i| match i {
            Item::CreateSys(v) => v.as_str(),
            other => panic!("expected CreateSys item, got {other:?}"),
        })
        .collect();
    assert_eq!(values, vec!["println", "strip.html", "*"]);
}

#[test]
fn parses_if_elif_else() {
    let program = parse_source(
        r#"
        main start = {
            if score is 100 {
                println("Perfect");
            } elif score >= 80 {
                println("Great");
            } else {
                println("Keep learning");
            }
        }
        "#,
    )
    .expect("should parse");

    let Item::Main(block) = &program.items[0] else {
        panic!("expected main");
    };
    match &block.stmts[0] {
        Stmt::If {
            branches,
            else_branch,
        } => {
            assert_eq!(branches.len(), 2);
            assert!(else_branch.is_some());
        }
        other => panic!("expected If stmt, got {other:?}"),
    }
}

#[test]
fn parses_one_line_then_form() {
    let program = parse_source(
        r#"
        main start = {
            if score is 100 then println("Perfect");
        }
        "#,
    )
    .expect("should parse");

    let Item::Main(block) = &program.items[0] else {
        panic!("expected main");
    };
    match &block.stmts[0] {
        Stmt::If {
            branches,
            else_branch,
        } => {
            assert_eq!(branches.len(), 1);
            assert!(else_branch.is_none());
        }
        other => panic!("expected If stmt, got {other:?}"),
    }
}

#[test]
fn parses_while_with_close() {
    let program = parse_source(
        r#"
        def count_numbers(max: int) = {
            let number: int = 1;
            while number <= max {
                println("Number: " + number);
                if number is 10 {
                    close;
                }
                number = number + 1;
            }
        }
        "#,
    )
    .expect("should parse");

    let Item::Function(f) = &program.items[0] else {
        panic!("expected function");
    };
    match &f.body.stmts[1] {
        Stmt::While { body, .. } => {
            assert!(matches!(body.stmts[1], Stmt::If { .. }));
            assert!(matches!(body.stmts[2], Stmt::Assign { .. }));
        }
        other => panic!("expected While stmt, got {other:?}"),
    }
}

#[test]
fn parses_top_level_system_statement() {
    // A system call written outside `main start` and outside any function
    // is a "Top-Level System" statement.
    let program = parse_source(
        r#"
        create.sys = println;

        println("Hello from top-level system");

        main start = {
            println("Hello from main");
        }
        "#,
    )
    .expect("should parse");

    assert_eq!(program.items.len(), 3);
    assert!(matches!(program.items[0], Item::CreateSys(_)));
    match &program.items[1] {
        Item::TopLevel(Stmt::ExprStmt(Expr::Call { callee, .. })) => {
            assert_eq!(callee, "println");
        }
        other => panic!("expected TopLevel println call, got {other:?}"),
    }
    assert!(matches!(program.items[2], Item::Main(_)));
}

#[test]
fn parses_complete_example_program_with_mixed_function_forms() {
    // Same shape as the spec's "COMPLETE VALIDATED RPP EXAMPLE", but
    // exercising all three allowed function forms: bare, `def`, and `uDef`
    // (per the current rule that `def emu` is optional, never mandatory).
    let src = r#"
        add utils.rpp;

        create.sys = print;
        create.sys = println;

        greet(name: string) = {
            println("Hello, " + name + "!");
        }

        def calculate_score(score: int) = {
            if score is 100 {
                return "Perfect";
            } elif score >= 80 {
                return "Great";
            } elif score >= 60 {
                return "Good";
            } else {
                return "Keep learning";
            }
        }

        uDef count_numbers(max: int) = {
            let number: int = 1;

            while number <= max {
                println("Number: " + number);

                if number is 10 {
                    println("Reached ten!");
                    close;
                }

                number = number + 1;
            }
        }

        main start = {
            let name: string = "Komandan";
            let score: int = 100;

            println("=== Rpp Test Program ===");

            run greet(name);

            let result = run calculate_score(score);

            println("Score: " + result);

            run count_numbers(20);

            println("Program finished.");
        }
    "#;

    let program = parse_source(src).expect("complete example should parse");

    // add + 2x create.sys + 3 functions + main = 7 items
    assert_eq!(program.items.len(), 7);

    assert!(matches!(program.items[0], Item::Include(_)));
    assert!(matches!(program.items[1], Item::CreateSys(_)));
    assert!(matches!(program.items[2], Item::CreateSys(_)));

    match &program.items[3] {
        Item::Function(f) => assert_eq!(f.keyword, FunctionKeyword::Def { emu: false }),
        other => panic!("expected bare-form function, got {other:?}"),
    }
    match &program.items[4] {
        Item::Function(f) => assert_eq!(f.keyword, FunctionKeyword::Def { emu: false }),
        other => panic!("expected def-form function, got {other:?}"),
    }
    match &program.items[5] {
        Item::Function(f) => assert_eq!(f.keyword, FunctionKeyword::UDef),
        other => panic!("expected uDef-form function, got {other:?}"),
    }
    assert!(matches!(program.items[6], Item::Main(_)));
}

#[test]
fn parses_const_let_declaration() {
    // Regression test: `const let` must not consume the `Let` token twice.
    let program = parse_source("main start = { const let score: int = 10; }").expect("should parse");
    let Item::Main(block) = &program.items[0] else {
        panic!("expected main");
    };
    match &block.stmts[0] {
        Stmt::Let { is_const, name, .. } => {
            assert!(*is_const);
            assert_eq!(name, "score");
        }
        other => panic!("expected Let stmt, got {other:?}"),
    }
}

#[test]
fn parses_arithmetic_precedence() {
    let program = parse_source("main start = { let result = 10 + 5 * 2; }").expect("should parse");
    let Item::Main(block) = &program.items[0] else {
        panic!("expected main");
    };
    let Stmt::Let { value, .. } = &block.stmts[0] else {
        panic!("expected let");
    };
    // 10 + (5 * 2)
    match value {
        Expr::Binary { left, op: BinOp::Add, right } => {
            assert!(matches!(**left, Expr::Int(10)));
            assert!(matches!(**right, Expr::Binary { op: BinOp::Mul, .. }));
        }
        other => panic!("expected top-level Add, got {other:?}"),
    }
}
