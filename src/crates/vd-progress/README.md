# vd-progress

Stderr progress reporting for long-running CLIs.

## Owns

- `--progress=text|json` emitters (`Progress`, `ProgressEvent`, `ProgressMode`)
- `ProgressFormat` (config / resolve)

Independent of artifact I/O.

```bash
cargo test -p vd-progress
```
