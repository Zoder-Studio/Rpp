# R++ / Rpp

Compiler foundation for **R++** (alternative written form: **Rpp**, expanded name **Rust++**).

## Status

This is the very first foundation stage of the compiler, built incrementally:

- [x] Lexer (tokenizer)
- [x] Parser (recursive descent) + AST
- [x] Semantic analysis (scope resolution, create.sys enforcement, immutability, close/return placement, call arity)
- [ ] Type checker
- [ ] Ownership / borrow checker
- [ ] Rpp IR
- [ ] Backends (Rust, C++, JS/CJS, Arduino, native, RVM bytecode)

Nothing here is a stub — lexer, parser, and semantic analysis are fully
working and covered by tests (`cargo test`) — but the compiler pipeline
does not yet lower to any IR or backend. Later stages will be added
incrementally, commit by commit.

## Function declaration forms currently supported

- `def name(params) = { ... }`
- `def emu name(params) = { ... }` (`emu` is an optional modifier, never mandatory)
- `name(params) = { ... }` (bare form — `def` itself is optional)
- `uDef name(params) = { ... }` (its own distinct function keyword)

## Usage

```sh
cargo build --release
./target/release/rpp path/to/file.rpp
```

Prints the parsed AST, or a lex/parse/semantic diagnostic on failure.

## Development

```sh
cargo test
```
