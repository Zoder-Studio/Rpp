# R++ / Rpp

Compiler foundation for **R++** (alternative written form: **Rpp**, expanded name **Rust++**).

## Status

This is the very first foundation stage of the compiler, built incrementally:

- [x] Lexer (tokenizer)
- [x] Parser (recursive descent) + AST
- [ ] Semantic analysis
- [ ] Type checker
- [ ] Ownership / borrow checker
- [ ] Rpp IR
- [ ] Backends (Rust, C++, JS/CJS, Arduino, native, RVM bytecode)

Nothing here is a stub — the lexer and parser are fully working and covered
by tests (`cargo test`) — but the compiler pipeline only goes as far as
producing an AST for now. Later stages will be added incrementally, commit
by commit.

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

Prints the parsed AST, or a lex/parse diagnostic on failure.

## Development

```sh
cargo test
```
