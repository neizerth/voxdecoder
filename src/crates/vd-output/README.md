# vd-output

Output path resolution for VoxDecoder CLIs.

## Owns

- `-o` / `-d` / `--in-place` / `--overwrite`
- Callers pass **`default_file_name`** (naming scheme is not hardcoded)
- Helpers: `fixed_file_name` (`.fixed.`), `stem_ext_file_name`, `segments_sidecar`, `ensure_writable`

Does **not** depend on `vd-artifact`.

```bash
# fix CLI
default_file_name = fixed_file_name(&input, "txt");   // meeting.fixed.txt

# transcription
default_file_name = stem_ext_file_name(&input, "txt"); // meeting.txt
```

```bash
cargo test -p vd-output
```
