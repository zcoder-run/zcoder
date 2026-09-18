# Development Best Practices

## Tracing

- Always use fully qualified `tracing` macros.
- All `tracing::debug!` calls must be prefixed with `->>`.

```rust
tracing::debug!("->> handling request for user {user_id}");
tracing::info!("server started on port {port}");
tracing::warn!("rate limit approaching threshold");
tracing::error!("failed to process transaction: {err}");
```

## Test Organization

Externalize large unit tests to a sibling `[file_name]_tests.rs` file using `#[path = "..."]`.

In `src/model/applier.rs`:

```rust
// region:    --- Tests

#[cfg(test)]
#[path = "applier_tests.rs"]
mod tests;

// endregion: --- Tests
```

In `src/model/applier_tests.rs`:

```rust
type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

use super::*;

#[test]
fn test_applier_basic() -> Result<()> {
    // -- Setup & Fixtures

    // -- Exec

    // -- Check

    Ok(())
}
```
