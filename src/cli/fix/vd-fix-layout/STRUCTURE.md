# vd-fix-layout — project layout

Rust crate for the text-layout fix CLI.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [TODO-languages.md](TODO-languages.md) · shared I/O: [`crates/`](../../../crates/)

---

## Philosophy

**Backend is an implementation detail.**

Tomorrow the backend may be language-specific rule engines, cue lexicons, small local classifiers, TimeMap pause fusion, or an ensemble — **none of that leaks** into `cli.md`, public modules, progress events, or dry-run JSON.

Exit code 4: *Inference backend failed to initialize*. Missing pack / missing TimeMap is not exit 4 when builtins / no-pause path exist.

Naming: domain folder `layout/` — job name, never `engine/` / `model/` / `pipeline/`.

**v1 scope:** paragraph breaks. Name `layout` allows later block-level whitespace work without a rename — still under the primary guarantee:

```text
Never changes lexical content.
```

**Installable packs** optional; `run` uses embedded `ru` / `en` baselines. `auto` resolves to one of those packs at runtime. Shipping a language means *excellent* local behavior — prefer deeper RU/EN before a third language.

---

## Tree (planned)

```
src/cli/fix/vd-fix-layout/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── TODO-languages.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── types.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── run.rs
│   │   ├── install.rs
│   │   ├── remove.rs
│   │   ├── list.rs
│   │   ├── info.rs
│   │   └── config_cmd.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── file.rs
│   │   └── resolve.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── catalog.rs          # ru / en shipping only in v1
│   │   └── pack.rs
│   ├── paths.rs                # VD_FIX_LAYOUT_*
│   └── layout/                 # this binary only
│       ├── mod.rs
│       ├── fixer.rs            # LayoutFixer::load / .fix → FixResult
│       ├── config.rs           # language, density, use_timemap
│       ├── timemap.rs          # discover / load optional TimeMap
│       ├── signals/            # sentences, pauses, density policy
│       └── backend/            # private
│           ├── mod.rs
│           ├── ru.rs
│           └── en.rs
│
├── tests/
│   ├── unit/
│   │   ├── mod.rs
│   │   ├── cli.rs
│   │   ├── layout_ru.rs
│   │   ├── layout_en.rs
│   │   └── guarantees.rs     # no split inside timed unit / speaker
│   └── e2e/
│       ├── mod.rs
│       └── binary.rs
│
└── fixtures/
    ├── input/{ru,en}/
    └── expected/{ru,en}/
```

---

## Domain model

```rust
pub enum ParagraphDensity {
    Compact,
    Normal,
    Relaxed,
}

pub struct LayoutLoadOptions {
    pub language: Language,       // Ru | En | Auto (Auto resolves before backend)
    pub models_dir: PathBuf,
    pub density: ParagraphDensity,
    pub use_timemap: bool,
    /// Abstract binding — not necessarily a filesystem path.
    pub timemap: Option<TimeMapRef>,
}

pub struct FixResult {
    pub text: String,
    pub changed: bool,
}
```

Reject non-`ru`/`en` pack codes at CLI with exit 2. Accept `Language::Auto` as a resolver (see [cli.md](cli.md)), never as a third pack.

**Guarantee tests:** for JSON / Meeting-shaped fixtures, assert no paragraph break is inserted inside a timed segment span or speaker-labeled unit; assert lexical content unchanged.

---

## Shared crates?

| Need | Crate |
|------|-------|
| Artifacts | `vd-artifact` |
| `-o` / `-d` / `--in-place` / `.fixed.` | `vd-output` |
| Progress | `vd-progress` |
| TimeMap types (when shared) | prefer existing preprocess/artifact TimeMap — do not fork |

---

## Per-language backend

```text
backend/ru.rs  — Russian cues, density maps, fixtures
backend/en.rs  — English cues, density maps, fixtures
```

Public API: `LayoutFixer::fix(…) → FixResult`.

---

## Progress phases

Suggested phases in progress / tests: `loading` → `analyzing` → `layout` → `writing`.

TimeMap binding in dry-run / Job context is abstract (`source: artifact` \| `job` \| `runtime` \| `cli` \| `none`) — not a promised filesystem path.

---

## Workspace wiring (when implementing)

1. Member `src/cli/fix/vd-fix-layout` in root `Cargo.toml`.
2. Capability `FixLayout` / `fix-layout` in `vd-pipeline`.
3. Default Job: after `fix-terms`, before postprocess.
4. `scripts/build.sh` + Docker runtime set.
5. Binder: always `-o`, honor `overwrite` / `language` / `density` / TimeMap binding.
