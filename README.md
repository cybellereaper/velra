# Velra

Velra is a small, expression-oriented programming language focused on readable code, predictable semantics, and a deliberately compact syntax.

The repository contains the Rust **stage-0 bootstrap implementation**. It has no runtime dependencies: the lexer, parser, evaluator, standard built-ins, CLI, and diagnostics are handwritten in Rust.

> Self-hosting is a bootstrap property, not a reason to duplicate the implementation. The Rust frontend is the reference bootstrap until the stage-1 compiler can compile itself; the bootstrap contract is documented in [`bootstrap/README.md`](bootstrap/README.md).

## Quick start

```bash
cargo run -- examples/hello.vel
cargo run -- check examples/fibonacci.vel
cargo run -- repl
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

## Language surface

Implemented in stage 0:

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

Reserved by the grammar but intentionally not enabled until their semantics are specified: `object`, `extend`, `shape`, `async`, `await`, `try`, and `throw`.

`use` declarations are parsed but module loading is not enabled in stage 0 yet. Failing explicitly is preferred to silently giving imports incorrect semantics.

## Types

Velra uses inference by default. An annotation is a runtime contract in the bootstrap implementation:

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
- small standard library primitives that can support the self-hosted compiler

`Lang.g4` remains the compatibility grammar while the handwritten Rust parser is bootstrapped. Where implementation and grammar differ, tests should make the intended behavior explicit before the grammar is changed.

## Development

```bash
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

The CI workflow runs the same checks.

## License

GPL-3.0-only. See [`LICENSE`](LICENSE).
