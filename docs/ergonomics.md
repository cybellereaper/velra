# Velra ergonomics

This experimental branch keeps Velra small by lowering convenience syntax into the existing AST/runtime model instead of introducing overlapping concepts. The `VELRA-AST-1` artifact format remains unchanged; stage 0 and the embedded self-hosted compiler are expected to emit byte-identical artifacts for this syntax.

## Structural destructuring

```velra
User(name: String, age: Int) {}
user = User("Ada", 20)
User(name, age) = user
[first, second] = [1, 2]
```

`_` ignores a value. Destructuring is structural and validates constructor names and arity.

## Pattern-aware `when`

```velra
when user {
    User(name, age) if age >= 18 => name
    User(name, _) => name + " (minor)"
    _ => "unknown"
}
```

Lowercase identifiers bind values inside that case. `_` is a wildcard. Constructor and list patterns recurse, and guards run with the bindings from the successful pattern. Each case alternative receives its own scope so failed pattern bindings cannot leak into another alternative.

## Method-style calls without a second function system

If a value has no concrete member with the requested name, `value.function(args)` resolves `function` in scope and invokes it as `function(value, args)`.

```velra
name = "  velra  ".trim().upper()
items = [1, 2, 3]
first = items.first()
```

Concrete members take precedence. This keeps free functions and method syntax interoperable rather than building two parallel APIs.

Built-in helpers in this slice include `contains`, `first`, `last`, `is_empty`, `trim`, `upper`, and `lower`.

## Ranges

```velra
exclusive = 1..4   // [1, 2, 3]
inclusive = 1..=4  // [1, 2, 3, 4]
```

Ranges lower to `range(start, end)` and `range_inclusive(start, end)`. Chained range operators are rejected rather than given surprising precedence.

## Pipelines

```velra
normalize(value) => value.trim().upper()
name = " velra " |> normalize
```

`value |> function` lowers to `function(value)`. The deliberately small rule avoids placeholder syntax or multiple partial-application conventions.

## Preconditions

```velra
require user != null
require age >= 18, "adult user required"
```

`require` lowers to the existing `assert` runtime primitive, so it adds readability without another error mechanism.

## Block values

Function blocks already evaluate to their final expression, so explicit `return` is only needed for early exits:

```velra
distance(a, b) {
    dx = b.x - a.x
    dy = b.y - a.y
    sqrt(dx * dx + dy * dy)
}
```

## Validation contract

The ergonomics suite executes representative programs through both the Rust stage-0 compiler and the embedded Velra compiler. It also checks byte-for-byte stage-0/self-host artifact equality for the new syntax and retains the compiler self-reproduction test. The self-hosted compiler's top-level parse loop is iterative so compiler growth does not consume one host stack frame per declaration.
