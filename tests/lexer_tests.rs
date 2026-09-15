use rpp_compiler::lexer::Lexer;
use rpp_compiler::token::{StringPart, TokenKind};

fn kinds(src: &str) -> Vec<TokenKind> {
    Lexer::new(src)
        .tokenize()
        .expect("lex should succeed")
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

#[test]
fn lexes_keywords_and_identifiers() {
    let k = kinds("main start def uDef emu run close let const if elif else foo123");
    assert_eq!(
        k,
        vec![
            TokenKind::Main,
            TokenKind::Start,
            TokenKind::Def,
            TokenKind::UDef,
            TokenKind::Emu,
            TokenKind::Run,
            TokenKind::Close,
            TokenKind::Let,
            TokenKind::Const,
            TokenKind::If,
            TokenKind::Elif,
            TokenKind::Else,
            TokenKind::Ident("foo123".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn single_line_comment_is_skipped() {
    let k = kinds("let x = 1; / this is a comment\nlet y = 2;");
    // No comment text should leak into tokens.
    assert!(matches!(k[0], TokenKind::Let));
    assert!(k.iter().all(|t| !matches!(t, TokenKind::Ident(s) if s.contains("comment"))));
}

#[test]
fn multiline_comment_is_skipped() {
    let src = "let x = 1;\n/\" this whole\nblock is a comment \"/\nlet y = 2;";
    let k = kinds(src);
    let let_count = k.iter().filter(|t| matches!(t, TokenKind::Let)).count();
    assert_eq!(let_count, 2);
}

#[test]
fn double_quoted_string_preserves_raw_newlines() {
    // println("
    // Hello
    // World
    // ");
    let src = "println(\"\nHello\nWorld\n\");";
    let tokens = Lexer::new(src).tokenize().expect("lex should succeed");
    let str_tok = tokens
        .iter()
        .find_map(|t| match &t.kind {
            TokenKind::Str(parts) => Some(parts.clone()),
            _ => None,
        })
        .expect("expected a string token");
    assert_eq!(str_tok.len(), 1);
    match &str_tok[0] {
        StringPart::Literal(s) => assert_eq!(s, "\nHello\nWorld\n"),
        _ => panic!("expected literal string part"),
    }
}

#[test]
fn triple_quoted_multiline_string() {
    let src = "let html = \"\"\"\n<html>\n</html>\n\"\"\";";
    let tokens = Lexer::new(src).tokenize().expect("lex should succeed");
    let str_tok = tokens
        .iter()
        .find_map(|t| match &t.kind {
            TokenKind::Str(parts) => Some(parts.clone()),
            _ => None,
        })
        .expect("expected a string token");
    match &str_tok[0] {
        StringPart::Literal(s) => assert_eq!(s, "\n<html>\n</html>\n"),
        _ => panic!("expected literal string part"),
    }
}

#[test]
fn string_interpolation_is_lexed_recursively() {
    let src = "println(\"Hello ${name}, umurmu ${age}\");";
    let tokens = Lexer::new(src).tokenize().expect("lex should succeed");
    let str_tok = tokens
        .iter()
        .find_map(|t| match &t.kind {
            TokenKind::Str(parts) => Some(parts.clone()),
            _ => None,
        })
        .expect("expected a string token");

    assert_eq!(str_tok.len(), 4);
    assert_eq!(str_tok[0], StringPart::Literal("Hello ".to_string()));
    match &str_tok[1] {
        StringPart::Interpolation(toks) => {
            assert_eq!(toks.len(), 1);
            assert_eq!(toks[0].kind, TokenKind::Ident("name".to_string()));
        }
        _ => panic!("expected interpolation part"),
    }
    assert_eq!(str_tok[2], StringPart::Literal(", umurmu ".to_string()));
    match &str_tok[3] {
        StringPart::Interpolation(toks) => {
            assert_eq!(toks[0].kind, TokenKind::Ident("age".to_string()));
        }
        _ => panic!("expected interpolation part"),
    }
}

#[test]
fn operator_aliases_and_symbols() {
    let k = kinds("is not and or then >= <= => ==");
    assert_eq!(
        k,
        vec![
            TokenKind::Is,
            TokenKind::Not,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Then,
            TokenKind::Gte,
            TokenKind::Lte,
            TokenKind::Arrow,
            TokenKind::FatEquals,
            TokenKind::Eof,
        ]
    );
}
