# Ergonomics v3

This layer reduces compiler and parser ceremony without adding parallel language concepts.

## Sum types

```velra
enum Token {
    Ident(text: String)
    Number(value: Int)
    Eof
}

when token {
    Ident(text) => text
    Number(value) => "${value}"
    Eof => "done"
}
```

Zero-payload variants are singleton values. Payload variants are constructors. The runtime type of every variant is the enum name.

## Maps and sets

```velra
keywords = {"if": "if", "while": "while"}
symbols = #{"==", "!=", "=>"}

keywords.get("if")
"==" in symbols
```

Maps preserve insertion order for deterministic display/artifacts while equality and lookup are key-based. Sets deduplicate by Velra value equality.

## Cursor operations

String/list cursors support `take_while`, `take_until`, `skip_while`, `skip_until`, `consume`, `expect_next`, and `match_longest` in addition to the existing lookahead operations.

```velra
input = source.cursor()
word = input.take_while(|ch| ch.is_alphanumeric())
op = input.match_longest(#{"...", "..=", ".."})
```

## Unicode character helpers

Single-character strings support Unicode-aware `is_digit`, `is_upper`, `is_lower`, `is_letter`, `is_alphanumeric`, and `is_whitespace` through normal method-call fallback.

## Pattern improvements

```velra
when ch = input.current() {
    _ if ch.is_digit() => scan_number(input)
    else => scan_symbol(input)
}

when values {
    [first, second, ...rest] => rest
    else => []
}
```
