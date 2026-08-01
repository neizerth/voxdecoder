# vd-assets — project layout

Rust crate for the project-assets CLI (prepare knowledge for `vd-fix-*`).

**Status: implemented.** Workspace member: `src/cli/process/vd-assets`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · process overview: [../README.md](../README.md) · shared I/O: [`crates/`](../../../crates/)

---

## Philosophy

**Extractors are an implementation detail.**

Tomorrow PDF/Office may use different crates or shell helpers — **none of that leaks** into `cli.md`, progress events, or dry-run JSON beyond `--ocr` / `--force`.

Naming: `convert/` + `dict/` — job names, never `engine/` / `model/`.

**This CLI prepares project knowledge; it does not fix transcripts.** Rewrite authority stays in `vd-fix-*`.

**OCR** is optional and local. No cloud conversion APIs. Engine brand is outside the public contract.

---

## Non-goals

`vd-assets` intentionally does **not**:

- rewrite transcript artifacts (`vd-fix-*`)
- lock terminology in place of `vd-fix-terms`
- repair ASR wording (`vd-fix-asr`)
- restyle presentation (`vd-fix-casing`)
- require downloadable packs / `install` before `run`
- invent glossary forms not grounded in source text

---

## Tree

Crate lives at `src/cli/process/vd-assets/`:

```
src/cli/process/vd-assets/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs                  # also consumed by vd-fix-* (load_dictionary)
│   ├── types.rs
│   ├── paths.rs                # VD_ASSETS_* via vd_artifact::paths
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs
│   │   └── config_cmd.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── file.rs
│   ├── convert/                # Office/PDF → text/Markdown
│   │   ├── mod.rs              # ConvertRequest / run pipeline
│   │   ├── cache.rs            # extract cache (fingerprint)
│   │   └── extract/
│   │       ├── mod.rs          # OcrMode, resolve_document
│   │       ├── plain.rs
│   │       ├── pdf.rs
│   │       ├── docx.rs
│   │       ├── xlsx.rs
│   │       └── ocr.rs          # OCR helper (implementation detail)
│   └── dict/                   # terms.yml + assets-dir loaders for fix CLIs
│       ├── mod.rs              # load_dictionary / write_terms / is_assets_dir
│       ├── glossary.rs
│       └── tokenize.rs
│
└── tests/
    ├── unit/                   # cli, convert, dict
    └── e2e/                    # binary
```

---

## Domain model

```rust
pub struct Dictionary {
    pub forms: BTreeSet<String>,
    pub entries: Vec<TermEntry>,
    pub source_paths: Vec<PathBuf>,
}

pub struct TermEntry {
    pub canonical: String,
    pub variants: Vec<String>,
}

pub enum OcrMode { Off, Auto, On }

pub struct ConvertRequest {
    pub inputs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub ocr: OcrMode,
    pub force: bool,
}
```

Public lib surface used by fix CLIs:

```rust
vd_assets::load_dictionary(paths, &DictionaryOptions::default())?;
// Prefer ./assets (terms.yml + md/). Also text / md / terms.yml.
// Not Office/PDF.
```

---

## Modules

| Path | Role |
|------|------|
| `vd-artifact` / `vd-output` / `vd-progress` | paths helpers, progress (output naming not primary here) |
| `cli/` | UX from [cli.md](cli.md) |
| `config/` | Persist progress / ocr defaults |
| `convert/` | Extract + write `md/` + drive terms build |
| `dict/` | Build / load / write `terms.yml`; assets-dir ingest for fix CLIs |
| `paths.rs` | `VD_ASSETS_CONFIG`, `VD_ASSETS_CACHE` |

---

## Conversion rules

Highest-level product rules:

1. **Text/Markdown present** among inputs → convert Office/PDF as well when present; always copy text sources into `md/`.
2. **No text/Markdown** → Office/PDF conversion is **mandatory**; fail if nothing convertible.
3. Terms file is always built from the **processed** Markdown/text set under `output/md/` (plus structured glossary parses).

Source precedence inside the terms merge: later files in the processed list extend forms/entries (structured glossary entries append; forms union).

---

## Pipeline

```text
collect inputs (-i)
classify textish vs convertible
convert Office/PDF → output/md/*.md   (cache unless --force)
copy textish → output/md/
load_dictionary(processed) → Dictionary
write_terms → output/terms.yml
```

Assets directory is the unit passed to fix CLIs:

```text
vd-fix-asr   --context ./assets
vd-fix-terms --terms ./assets
```

---

## Shared crates?

**Yes — lean on [`crates/`](../../../crates/)** for progress and platform paths. Do not put Office extractors into shared crates; they stay in this binary/lib.

| Keep in shared crates | Keep in this binary/lib |
|------------------------|-------------------------|
| `vd-progress`, `vd_artifact::paths` | `convert/`, `dict/` |
| — | clap / config UX |

Fix CLIs depend on **`vd-assets` as a library** for `load_dictionary` (assets dir / text / `terms.yml`).

---

## Progress

| Command | Events |
|---------|--------|
| `run` | `start` → `phase converting` → `phase terms` → `done` / `error` |

Same `--progress=text|json` and `-q` as [cli.md](cli.md).

---

## Tests

| Path | Role |
|------|------|
| `tests/unit/cli.rs` | clap / shorthand / `--ocr` |
| `tests/unit/convert.rs` | docx → md when no text; md-only terms |
| `tests/unit/dict.rs` | yaml load / write; assets dir; reject PDF |
| `tests/e2e/binary.rs` | dry-run, missing input, markdown run |

```bash
cargo test -p vd-assets
./scripts/test.sh vd-assets
```

---

## Build

```bash
cd src/cli/process/vd-assets
cargo build --release
cargo test
cargo run -- run -i ../../fix/vd-fix-terms/fixtures/terms -o /tmp/vd-assets-out --dry-run
```

Binary name: `vd-assets`.  
Workspace member: `src/cli/process/vd-assets`.

---

## Public contract note

Extractor crates, XML stripping, and OCR engine details are intentionally **outside** the public CLI contract.
