---
applyTo: **/*.rs
---
# Rust Code Generation Guidelines

## Pattern Matching: Always Use `_` for Catch-All Patterns

In Rust pattern matching, never use bare enum variant names as catch-all patterns. Always use `_` instead.
`Option<T>` variant cannot match against None, as this is not a returnable value of an Option.

**Incorrect:**
```rust
match x {
    Option(val) => ...,
    None => ...,  // ❌ WRONG - None is treated as a binding, not a pattern
}
```

**Correct:**
```rust
match x {
    Option(val) => ...,
    _ => ...,  // ✅ Catch-all pattern
}
```

The `_ =>` pattern is the standard way to handle all other cases in Rust - including None-variants. Do not use variant names as bindings in match arms.
