# Control-flow and cursor ergonomics

Velra supports stateful algorithms directly without forcing them through counted `for` loops.

```velra
var index = 0
while index < values.len() {
    index += 1
    if should_skip(index) { continue }
    if should_stop(index) { break }
}
```

Membership is expressed with `in`:

```velra
ch in [" ", "\\t", "\\r"]
pair in ["==", "!=", "<=", ">="]
```

Safe indexing and slicing are collection operations:

```velra
next = chars.get(index + 1) ?: ""
text = chars[start..end]
```

For stream-style algorithms, strings and lists expose a mutable cursor through the standard prelude:

```velra
input = source.cursor()
while !input.done() {
    current = input.current()
    next = input.peek()
    prefix = input.peek_string(2)
    input.advance()
}
```

Cursor operations are `current`, `peek`, `peek_string`, `advance`, `take`, `done`, `starts_with`, and `position`. The methods are ordinary UFCS-style prelude functions, so the language does not need lexer-specific syntax.
