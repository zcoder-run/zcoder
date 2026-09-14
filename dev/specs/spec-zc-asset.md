# zc-asset Specification

## Intent

Define the embedded asset runtime used to materialize the `.zcoder` workspace directory in a target project.

`zc-asset` embeds an asset archive at compile time and exposes helpers to read individual assets or sync missing assets into a project's `.zcoder` directory.

## Module Layout

```text
crates/zc-asset/src/
  lib.rs    # asset runtime, public API, and tests
  error.rs  # local Error and Result
```

## Public API

- `extract_asset(path)` and `extract_asset_str(path)`
- `extract_zfile(path)` returning `ZFile { path, content }`
- `list_asset_paths(prefix)`
- `update_zcoder_project(project_dir)`
- `ZFile` with `as_str()` and `into_string()` helpers

The embedded archive is available as `ASSETS_ZIP`, sourced from the `ASSETS_ZIP` environment variable through `include_bytes!`.

## Behavior

- `extract_asset` looks up a normalized asset path in the archive and returns its binary content.
- `extract_asset_str` returns the same content as a UTF-8 string.
- `extract_zfile` returns a `ZFile` carrying the path and binary content.
- `list_asset_paths(prefix)` returns sorted asset paths matching an optional prefix.
- `update_zcoder_project(project_dir)` creates `.zcoder/` in the target project and writes only missing assets, so existing user edits are preserved.

## Error Model

- `zc-asset::Error` is local to the crate and covers missing assets and IO/archive failures.
- `zc-asset` has no dependency on other domain crates.

## Design Considerations

- The asset runtime keeps workspace `.zcoder` state reproducible while preserving user edits, because `update_zcoder_project` writes only missing files.
- Embedding the archive at compile time keeps the runtime self-contained with no external files to ship.
