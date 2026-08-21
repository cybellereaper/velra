# Velra cleanups v2

This slice builds on the structural pattern and method-call work without introducing a second object, error, or callback system.

## Result and Option

The core runtime provides four ordinary pattern-compatible variants:

```velra
Ok(value)
Err(error)
Some(value)
None()
```

They work with the existing `when` pattern system and method-style helpers such as `is_ok`, `is_err`, `is_some`, `is_none`, `unwrap`, and `unwrap_or`.

## Propagation

Postfix `?` unwraps `Ok` and `Some`. `Err` and `None` return unchanged from the current function:

```velra
load(path) {
    text = read_config(path)?
    parse_config(text)?
}
```

Using `?` with another value is a runtime error rather than an implicit coercion.

## String interpolation

Expressions can be embedded with `${...}` and are converted using the standard `string` function:

```velra
message = "Hello ${user.name}, score=${score + 1}"
```

`$$` produces one literal dollar sign.

## Expression lambdas

Lambdas use one syntax for one or more parameters:

```velra
apply(21, |x| x * 2)
combine(20, 22, |a, b| a + b)
```

They are ordinary lexical closures and use the same call semantics as named functions.

## Patterns in loops

`for` now accepts the same structural patterns already used by assignment and `when`:

```velra
for User(name, age) in users {
    println("${name}: ${age}")
}
```

The parser lowers this through the existing assignment-pattern machinery, keeping the runtime model small.
