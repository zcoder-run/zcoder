# zc-asset Specification

## Intent

Define the embedded asset runtime used to materialize the `.zcoder` workspace directory in a target project.

`zc-asset` embeds an asset archive at compile time and exposes helpers to read individual assets and to sync missing assets into a target workspace `.zcoder` directory or into the base directory.

The archive carries two namespaces:

- `wspace/`: workspace assets, materialized under `<wspace_dir>/.zcoder/`.
- `base/`: base assets, materialized directly under `<zbase_dir>` as `config-default.toml` and `config-user.toml`.

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
- `update_wspace_dir(wspace_dir)`, with `update_zcoder_project(project_dir)` kept as a compatibility alias
- `update_zbase_assets(zbase_dir)`
- `ZFile` with `as_str()` and `into_string()` helpers

The embedded archive is available as `ASSETS_ZIP`, sourced from the `ASSETS_ZIP` environment variable through `include_bytes!`.

## Behavior

- `extract_asset` looks up a normalized asset path in the archive and returns its binary content.
- `extract_asset_str` returns the same content as a UTF-8 string.
- `extract_zfile` returns a `ZFile` carrying the path and binary content.
- `list_asset_paths(prefix)` returns sorted asset paths matching an optional prefix.
- `update_wspace_dir(wspace_dir)` creates `.zcoder/` in the target workspace and writes only missing `wspace/*` assets, so existing user edits are preserved.
- `update_zcoder_project(project_dir)` delegates to `update_wspace_dir`.
- `update_zbase_assets(zbase_dir)` creates the base directory when missing, always (re)writes `config-default.toml` from `base/config-default.toml`, and creates `config-user.toml` from `base/config-user.toml` only when it does not already exist.

## Error Model

- `zc-asset::Error` is local to the crate and covers missing assets and IO/archive failures.
- `zc-asset` has no dependency on other domain crates.

## Design Considerations

- The asset runtime keeps workspace `.zcoder` state reproducible while preserving user edits, because `update_wspace_dir` writes only missing files.
- The base directory keeps `config-default.toml` managed and refreshed from the embedded asset, while `config-user.toml` is created only when missing so user edits survive restarts.
- Embedding the archive at compile time keeps the runtime self-contained with no external files to ship.
