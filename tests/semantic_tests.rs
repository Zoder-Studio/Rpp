use rpp_compiler::check_source;
use rpp_compiler::semantic::SemanticError;

fn expect_errors(src: &str) -> Vec<SemanticError> {
    match check_source(src) {
        Ok(_) => panic!("expected semantic errors, but program checked out clean"),
        Err(rpp_compiler::RppError::Semantic(errs)) => errs,
        Err(other) => panic!("expected semantic errors, got lex/parse error: {other}"),
    }
}

fn expect_ok(src: &str) {
    if let Err(e) = check_source(src) {
        panic!("expected program to check out clean, got: {e}");
    }
}

#[test]
fn complete_example_program_checks_out_clean() {
    // Mirrors the spec's "COMPLETE VALIDATED RPP EXAMPLE" (mixed function
    // forms: bare / def / uDef, since `def emu` is optional, never
    // mandatory).
    expect_ok(
        r#"
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
            } else {
                return "Keep learning";
            }
        }

        uDef count_numbers(max: int) = {
            let number: int = 1;
            while number <= max {
                println("Number: " + number);
                if number is 10 {
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
        }
        "#,
    );
}

#[test]
fn calling_unregistered_system_command_is_an_error() {
    // This is exactly the scenario in spec section 4: using `print`
    // without a prior `create.sys = print;` must be a compile-time error.
    let errors = expect_errors(
        r#"
        main start = {
            print("Hello");
        }
        "#,
    );
    assert!(errors
        .iter()
        .any(|e| e.message.contains("print") && e.message.contains("create.sys")));
}

#[test]
fn registering_system_command_fixes_the_error() {
    expect_ok(
        r#"
        create.sys = print;
        main start = {
            print("Hello");
        }
        "#,
    );
}

#[test]
fn star_registration_allows_any_system_command() {
    expect_ok(
        r#"
        create.sys = *;
        main start = {
            println("Hello");
            print("World");
        }
        "#,
    );
}

#[test]
fn assigning_to_const_let_is_an_error() {
    // Spec section 7: `const let score = 10; score = 20;` must be a
    // compile-time error because the binding is immutable.
    let errors = expect_errors(
        r#"
        main start = {
            const let score: int = 10;
            score = 20;
        }
        "#,
    );
    assert!(errors
        .iter()
        .any(|e| e.message.contains("score") && e.message.contains("immutable")));
}

#[test]
fn assigning_to_plain_let_is_allowed() {
    expect_ok(
        r#"
        main start = {
            let score: int = 10;
            score = 20;
        }
        "#,
    );
}

#[test]
fn close_outside_while_is_an_error() {
    let errors = expect_errors(
        r#"
        main start = {
            close;
        }
        "#,
    );
    assert!(errors.iter().any(|e| e.message.contains("close")));
}

#[test]
fn close_inside_while_is_allowed() {
    expect_ok(
        r#"
        main start = {
            let n: int = 0;
            while n <= 10 {
                close;
            }
        }
        "#,
    );
}

#[test]
fn return_outside_function_is_an_error() {
    let errors = expect_errors(
        r#"
        create.sys = println;

        return;

        main start = {
            println("hi");
        }
        "#,
    );
    assert!(errors.iter().any(|e| e.message.contains("return")));
}

#[test]
fn return_inside_function_is_allowed() {
    expect_ok(
        r#"
        def sum_two(a: int, b: int) = {
            return a + b;
        }
        main start = {
            let r = run sum_two(1, 2);
        }
        "#,
    );
}

#[test]
fn call_with_wrong_arity_is_an_error() {
    let errors = expect_errors(
        r#"
        def sum_two(a: int, b: int) = {
            return a + b;
        }
        main start = {
            let r = run sum_two(1);
        }
        "#,
    );
    assert!(errors
        .iter()
        .any(|e| e.message.contains("sum_two") && e.message.contains("2 argument")));
}

#[test]
fn undefined_variable_is_an_error() {
    let errors = expect_errors(
        r#"
        create.sys = println;
        main start = {
            println(unknown_name);
        }
        "#,
    );
    assert!(errors.iter().any(|e| e.message.contains("unknown_name")));
}

#[test]
fn duplicate_function_declaration_is_an_error() {
    let errors = expect_errors(
        r#"
        def helper(x: int) = {
            return x;
        }
        def helper(y: int) = {
            return y;
        }
        main start = {
        }
        "#,
    );
    assert!(errors.iter().any(|e| e.message.contains("helper")));
}

#[test]
fn shadowing_a_variable_in_the_same_scope_is_allowed() {
    // Rust-style shadowing: re-declaring `let` with the same name in the
    // same scope is allowed, not flagged as a duplicate.
    expect_ok(
        r#"
        main start = {
            let x: int = 1;
            let x: int = 2;
        }
        "#,
    );
}

#[test]
fn for_loop_variable_is_visible_in_its_body() {
    expect_ok(
        r#"
        create.sys = println;
        main start = {
            let names = ["Komandan", "Nasa"];
            for name in names {
                println(name);
            }
        }
        "#,
    );
}

#[test]
fn for_loop_over_undefined_iterable_is_an_error() {
    let errors = expect_errors(
        r#"
        create.sys = println;
        main start = {
            for item in undefined_list {
                println(item);
            }
        }
        "#,
    );
    assert!(errors.iter().any(|e| e.message.contains("undefined_list")));
}

#[test]
fn close_inside_for_loop_is_allowed() {
    expect_ok(
        r#"
        main start = {
            let numbers = [1, 2, 3];
            for n in numbers {
                close;
            }
        }
        "#,
    );
}

#[test]
fn for_loop_variable_does_not_leak_outside_its_body() {
    let errors = expect_errors(
        r#"
        create.sys = println;
        main start = {
            let numbers = [1, 2, 3];
            for n in numbers {
                println(n);
            }
            println(n);
        }
        "#,
    );
    assert!(errors.iter().any(|e| e.message.contains("undefined variable 'n'")));
}

#[test]
fn local_create_sys_is_scoped_to_its_block() {
    // create.sys registered inside a function is visible in that function
    // but not from an unrelated function that never registered it.
    let errors = expect_errors(
        r#"
        def only_here() = {
            create.sys = println;
            println("ok");
        }
        def not_here() = {
            println("should fail: not registered in this scope");
        }
        main start = {
            run only_here();
            run not_here();
        }
        "#,
    );
    assert!(errors
        .iter()
        .any(|e| e.message.contains("println") && e.message.contains("create.sys")));
}
