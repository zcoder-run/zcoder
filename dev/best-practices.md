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

## Workspace and Workbench Terminology & Naming

Distinguish between user-facing surfaces and internal code identifiers for workspace and workbench concepts.

### User-Facing Surfaces

- Use full natural words (`workspace`, `workbench`, `worktree`) in user documentation, CLI help output, and user error messages.
- In configuration files (such as `config.toml`), use `[workspace]` as the section table name.

### Internal Code Identifiers

- Use short PascalCase prefixes for Rust types:
  - `WSpace` (for workspace engine components)
  - `WBench` (for workbench context and items)
  - `WTree` (for worktree structures)
  - `WSpaceConfig` (for workspace configuration struct)
- Use short snake_case prefixes for variables, struct fields, and function parameters:
  - `wspace_dir: SPath` (instead of `workspace_dir` or `project_dir`)
  - `wbench_dir: SPath`
  - `wtree_dir: SPath`
  - `wspace: &WSpace`
- Method and builder names follow the same short convention, for example `with_wspace_dir(...)` and `wspace.wbench(...)`.
