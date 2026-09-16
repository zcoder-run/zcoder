# zc-common Specification

## Intent

Define the shared pure types and small utilities that need to cross crate boundaries.

`zc-common` provides a stable shared data boundary. It stays dependency-light and must not become a global application foundation crate.

## Module Layout

```text
crates/zc-common/src/
  lib.rs         # module registry and public re-exports
  error.rs       # local Error and Result
  cache.rs       # file cache helpers
  event_base.rs  # bounded mpsc channel primitives
  msg_id.rs      # message id newtype
  time.rs        # time helpers
  yaml.rs        # content conversion helpers
```

`lib.rs` registers and re-exports the modules:

```rust
// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub mod cache;
pub mod event_base;
pub mod msg_id;
pub mod time;
pub mod yaml;

// endregion: --- Modules
```

## Modules

- `error`: local `Error` and `Result`, scoped to this crate only
- `event_base`: bounded mpsc channel primitives, `MpscTx`, `MpscRx`, and `new_mpsc_bounded(name, capacity)`
- `msg_id`: the `MsgId` newtype that identifies a message crossing the router boundary
- `time`: time helpers such as `now_micro()`
- `cache`: file cache helpers such as `save_file_cache(name, content)`
- `yaml`: content conversion helpers such as `json_to_yaml_string`

## Design Considerations

- A narrow `zc-common` avoids creating a large shared dependency that every crate must accept, which reduces coupling and keeps ownership decisions explicit.
- `zc-common::Error` is local to shared common behavior and is not a workspace-wide error.
- `zc-common` must not depend on `zc-core`, `zc-router`, `zc-base`, `zc-tui`, `zc-asset`, or the root binary.
