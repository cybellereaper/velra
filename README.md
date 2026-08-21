# Velra

Velra is a small, expression-oriented programming language focused on readable code, predictable semantics, and a deliberately compact syntax.

Velra is self-hosted at the source-compiler boundary: the compiler in [`compiler/main.vel`](compiler/main.vel) lexes, parses, and emits Velra artifacts, and the released `velra` executable embeds a verified compiled copy of that compiler. End users do **not** need Rust or Cargo to run, check, or compile Velra programs.

Rust remains intentionally small and below the language boundary. It provides the portable runtime/VM, artifact loader, native CLI host, and trusted stage-0 bootstrap used to reproduce the embedded compiler. The production CLI does not use the Rust lexer/parser for user source.

## Install

Tagged releases publish standalone x86-64 binaries for:

- Windows (`velra-x86_64-pc-windows-msvc.zip`)
- Linux (`velra-x86_64-unknown-linux-gnu.tar.gz`)

Each archive has a matching SHA-256 checksum. Extract the archive and place `velra` or `velra.exe` somewhere on your `PATH`.

No Rust installation is required for released binaries.

## Quick start

```bash
velra examples/hello.vel
velra check examples/fibonacci.vel
velra compile examples/fibonacci.vel fibonacci.velc
velra exec fibonacci.velc
velra repl
```

A Velra program stays intentionally terse:

```velra
User(name: String) {
    greeting => "Hello, " + name + "!"
}

greet(user: User) => println user.greeting

greet User("Velra")
```

Bindings are immutable by default. Mutation must be explicit:

```velra
answer = 42
var count = 0
count = count + 1
```

Functions are declarations without a `func` keyword:

```velra
add(a: Number, b: Number) => a + b

fib(n: Int) {
    if n < 2 {
        return n
    }
    fib(n - 1) + fib(n - 2)
}
```

Control flow is expression-oriented:

```velra
label = when status {
    200 => "ok"
    404 => "missing"
    else => "error"
}

name = user?.name ?: "anonymous"
```

## Toolchain architecture

The source-to-artifact path is:

```text
program.vel
    |
    v
embedded compiler/bootstrap.velc
    |  (compiler written in Velra)
    v
VELRA-AST-1 artifact
    |
    v
Rust runtime / artifact loader
    |
    v
program execution
```

The repository keeps two compiler implementations for one specific reason:

- `compiler/main.vel` is the production source compiler and language implementation.
- the Rust lexer/parser/encoder is stage 0: a trusted bootstrap seed and compatibility oracle used to reproduce the Velra compiler artifact.

`compiler/bootstrap.velc` is checked in deliberately. Tests require it to be exactly the artifact produced from `compiler/main.vel`, and the Velra compiler must reproduce the same artifact byte-for-byte when compiling itself.

See [`bootstrap/README.md`](bootstrap/README.md) for the trust chain and reproducibility contract.

## Language surface

Implemented:

- Unicode identifiers
- `name = value` immutable bindings
- `var name = value` mutable bindings
- optional type annotations and nullable `T?`
- expression- and block-bodied functions
- lexical closures and recursion
- `if` / `else` expressions
- `when` expressions with guards
- `for name in iterable` loops
- data declarations with computed fields
- lists, indexing, negative indexes, and list mutation through `push`
- safe member access with `?.`
- null fallback with `?:`
- command-style calls such as `println "hello"`
- strict boolean logic and checked integer arithmetic
- built-ins: `print`, `println`, `len`, `type`, `assert`, `range`, `push`, `read`, `write`
- parser/check mode and REPL
- deterministic compiled `VELRA-AST-1` artifacts
- self-hosted source compilation

Reserved by the grammar but intentionally not enabled until their semantics are specified: `object`, `extend`, `shape`, `async`, `await`, `try`, and `throw`.

`use` declarations are parsed but module loading is not enabled yet. Failing explicitly is preferred to silently giving imports incorrect semantics.

## Types

Velra uses inference by default. An annotation is a runtime contract in the current runtime:

```velra
var age: Int = 20
find(id: Int) => null
User(name: String?)
```

Immutable bindings infer their type; explicit binding annotations are reserved for `var` so there is only one typed-binding form.

Built-in type names are `Null`, `Bool`, `Int`, `Float`, `String`, `List`, `Function`, `Number`, and `Any`. Data declarations introduce their own runtime type name.

## Design constraints

Velra intentionally avoids syntax that does not pay for itself. The current direction is:

- immutable by default
- no declaration keyword for ordinary immutable values or functions
- one null value and one nullable marker
- expressions instead of statement-only control flow where practical
- explicit mutation
- no implicit truthiness
- no silent numeric/string coercion
- small standard-library primitives sufficient to implement the compiler in Velra
- deterministic bootstrap artifacts

`Lang.g4` remains the compatibility grammar. The Velra compiler is now the production source frontend; the Rust stage-0 parser is retained only for bootstrap/reproducibility checks.

## Development

Building the repository from source still requires Rust because Rust is the trusted bootstrap/runtime host:

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

The test suite verifies all three bootstrap stages:

1. Rust stage 0 compiles `compiler/main.vel`.
2. The resulting Velra compiler compiles normal Velra source.
3. The Velra compiler recompiles itself byte-for-byte.

It also verifies that the checked-in `compiler/bootstrap.velc` has not drifted from `compiler/main.vel`.

## Releases

The `Release` GitHub Actions workflow builds and smoke-tests native Windows and Linux binaries on pull requests. Pushing a version tag such as `v0.1.0` builds the same artifacts, computes SHA-256 checksums, and publishes them to a GitHub Release.

## License

GPL-3.0-only. See [`LICENSE`](LICENSE).
