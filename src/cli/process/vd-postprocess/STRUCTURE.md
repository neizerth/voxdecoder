# vd-postprocess — project layout

Rust crate: **universal recipe executor** — domain library **and** CLI surface for `use: postprocess` on the shared Executor.

**Status: implemented.** Workspace member: `src/cli/process/vd-postprocess`. Default provider: `stub`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [../vd-pipeline/](../vd-pipeline/)

---

## Philosophy

```text
Artifact(s)
      +
Recipe
      +
Provider
      ↓
Derived Artifacts
```

Not “prompt + LLM”. A recipe is a **document**; a provider is an **execution backend**. The body of a recipe may be a prompt today and a process/HTTP/MCP payload tomorrow — the contract stays the same.

- **No built-in recipes** — empty list is an error.
- **Recipes are user/company assets** — CLI never knows “summary” / “jira” / “RFC”.
- **Inputs are artifacts** — named bindings to `ArtifactRef`, not a single file path.
- **One recipe → many outputs** — declared in the recipe; registered as Job artifacts.
- **Provider = ExecutionProvider** — OpenAI, Claude, Ollama, llama.cpp, local process, Python, HTTP, MCP tool, …
- **Normal DAG capability** — parallel fan-out is first-class.
- **CLI ≡ capability** — same request shape; binder may use library or binary.

Product: [README.md](README.md).

---

## Unified capability contract

Every capability in VoxDecoder shares one shape:

```text
Inputs + Options  →  Capability  →  Artifacts
```

| Capability | In | Out |
|------------|----|-----|
| `transcribe` | audio | transcript |
| `fix-*` | transcript | transcript |
| `diarize` | audio | timeline |
| `meeting-merge` | tracks + timeline + model | meeting |
| **`postprocess`** | artifacts + recipes + provider | artifacts |

`vd-pipeline` / `vd-srv` / MCP stay universal: they never learn summary vs jira — only artifacts and options.

---

## Non-goals

- Shipping corporate recipe packs inside the crate
- Silent fallback recipes when none given
- Treating recipes as “just a prompt string”
- Single `PathBuf` as the only input model
- Assuming every provider is an LLM
- Inventing output paths the recipe did not declare
- Coupling the binder to “must spawn a subprocess”
- Replacing `fix-*` / owning Meeting Model / diarization

---

## Tree (target)

Domain logic lives under `postprocess/` (not a CLI-shaped `run.rs`):

```
src/cli/process/vd-postprocess/
├── Cargo.toml
├── README.md
├── cli.md
├── STRUCTURE.md
├── RUST.md
├── src/
│   ├── main.rs
│   ├── lib.rs                      # plan / execute for MCP, pipeline binder, tests
│   ├── paths.rs
│   ├── cli/                        # flags → PostprocessRequest (thin)
│   ├── config/
│   ├── status/
│   └── postprocess/                # domain
│       ├── mod.rs
│       ├── executor.rs             # plan + execute (dry-run aware)
│       ├── recipe.rs               # load / validate / render RecipeDoc
│       ├── result.rs               # RecipeResult / DerivedArtifact
│       └── provider.rs             # ExecutionProvider trait + backends
│
└── tests/
    ├── unit/
    ├── integration/
    ├── e2e/
    └── fixtures/
        ├── artifacts/
        ├── recipes/                # examples for tests — not product builtins
        └── schemas/
```

| Path | Role |
|------|------|
| `cli/` | UX only → `PostprocessRequest` |
| `postprocess/executor.rs` | Resolve provider → load recipes → validate → plan → execute |
| `postprocess/recipe.rs` | Full recipe document |
| `postprocess/result.rs` | `RecipeResult { outputs }` |
| `postprocess/provider.rs` | `ExecutionProvider` (+ openai / ollama / process / http / mcp / …) |
| `config/` | Default provider / progress |

---

## Domain model (planned)

```rust
/// Library / Job request — artifacts, not bare paths as the primary model.
pub struct PostprocessRequest {
    /// Named bindings: recipe input name → artifact ref (id or path).
    pub inputs: BTreeMap<String, ArtifactRef>,
    /// Recipe documents to run (must be non-empty).
    pub recipes: Vec<ArtifactRef>,       // usually Path; ids if recipes become artifacts later
    pub provider: ExecutionProviderSpec,
    pub variables: BTreeMap<String, String>,
    pub output_dir: Option<PathBuf>,
    pub overwrite: bool,
}

/// How to execute a recipe — not “LLM config” only.
pub struct ExecutionProviderSpec {
    pub r#type: String,                  // openai | anthropic | ollama | process | http | mcp | …
    pub model: Option<String>,
    pub command: Option<String>,
    pub options: BTreeMap<String, ArgValue>, // temperature, base_url, … (from Job / recipe)
}

/// Full recipe document (user-owned).
pub struct RecipeDoc {
    pub version: u32,
    pub id: Option<String>,
    pub name: Option<String>,
    pub inputs: BTreeMap<String, RecipeInputDecl>,
    pub variables: BTreeMap<String, RecipeVarDecl>,
    /// Optional provider knobs merged into ExecutionProviderSpec at plan time.
    pub provider: Option<RecipeProviderHints>,
    /// Provider-specific body (prompt text, argv template, HTTP template, …).
    pub body: RecipeBody,
    pub outputs: Vec<RecipeOutput>,      // required, non-empty
}

pub enum RecipeBody {
    Prompt(String),                      // LLM-style
    // Process { argv: … } | Http { … } | … — grow without changing Executor
}

pub struct RecipeOutput {
    pub id: String,
    pub path: String,                    // may include {{ vars }}
    pub format: Option<String>,
    pub mime: Option<String>,
    pub schema: Option<PathBuf>,
}

pub struct DerivedArtifact {
    pub id: String,
    pub path: PathBuf,
}

/// One recipe invocation may emit many artifacts.
pub struct RecipeResult {
    pub recipe_id: Option<String>,
    pub outputs: Vec<DerivedArtifact>,
}

pub struct ExecutionPlan {
    pub provider: ExecutionProviderSpec,
    pub steps: Vec<PlannedRecipe>,       // recipe + resolved bindings + output paths
}

pub struct PostprocessResult {
    pub results: Vec<RecipeResult>,
}
```

`ArtifactRef` is the same idea as in `vd-pipeline` (artifact id **or** filesystem path). CLI may accept `name=path` and convert to refs; the domain never assumes “one file”.

Empty `recipes` → usage / exit 2.

---

## Recipe document (full)

Executor does not know “summary”. It knows Recipe → Execution Plan → Artifacts.

```yaml
version: 1
id: summary
name: Meeting summary

inputs:
  transcript:
    required: true
  meeting:
    required: false          # or optional: true — pick one spelling at impl

variables:
  audience: Engineering

provider:
  temperature: 0.2
  # type/model usually from Job options; recipe may hint defaults

# body — today often a prompt; other provider types use sibling fields later
prompt: |
  Summarize for {{ audience }}.

  {% if meeting %}Meeting: {{ meeting }}{% endif %}
  Transcript: {{ transcript }}

outputs:
  - id: summary
    path: summary.md
    mime: text/markdown
  - id: decisions
    path: decisions.md
    mime: text/markdown
```

One recipe → **many** outputs in one provider invoke (e.g. summary + decisions + tasks) when the recipe declares them.

---

## Capability wiring (`vd-pipeline`)

```text
Capability::Postprocess
        ↓
  Backend binding          # library call and/or CLI — binder detail
        ↓
  Artifacts (RecipeResult.outputs)
```

Do **not** document the product as “spawn `vd-postprocess`”. Subprocess is one possible binding; in-process library is another. Job authors only see `use: postprocess`.

| Phase | Behavior |
|-------|----------|
| Schema | `Capability::Postprocess` |
| Resolve | Non-empty recipes; map named inputs → `ArtifactRef` |
| Bind | Backend binding → `postprocess::executor` |
| Outputs | Register each `DerivedArtifact.id` → path |
| Progress | Standard step events |

Until implemented: `is_reserved()` includes `Postprocess`.

```text
               Transcript
                    │
        ┌───────────┼────────────┐
        ▼           ▼            ▼
 postprocess   postprocess   postprocess
```

---

## Algorithm

Dry-run must be able to print a full **ExecutionPlan** without invoking the provider:

```text
collect request (inputs, recipes, variables, provider options)
      ↓
reject if no recipes
      ↓
resolve ExecutionProvider
      ↓
load recipes
      ↓
validate inputs against each recipe’s declarations
      ↓
build ExecutionPlan          # bindings, output paths, provider, variables
      ↓
[--dry-run → emit plan → stop]
      ↓
execute plan
      ↓
for each planned recipe:
      invoke ExecutionProvider
      write declared outputs
      validate schema / mime when present
      ↓
return PostprocessResult { results: Vec<RecipeResult> }
```

Order matters: **provider first**, then recipes, then input validation, then plan — so dry-run shows the same resolved provider the run would use.

---

## Tests (planned)

| Topic | Proof |
|-------|--------|
| no recipes | exit 2 |
| multi-input | named `ArtifactRef` map; required keys |
| multi-output | one recipe → `RecipeResult.outputs.len() > 1` |
| ExecutionProvider | process/http stub without LLM |
| ExecutionPlan dry-run | plan JSON lists provider + recipes + outputs; no invoke |
| binder | `Capability::Postprocess` → backend binding (lib or CLI) |
| parallel Job | two postprocess steps share transcript artifact id |
| schema | invalid body → err |

```bash
# once crate exists:
cargo test -p vd-postprocess
./scripts/test.sh vd-postprocess
```

---

## Public contract note

**Recipe + ExecutionProvider + artifact inputs → RecipeResult (many Derived Artifacts).**  
Same Inputs → Capability → Artifacts contract as the rest of the platform. Capability name is `postprocess`, not “llm” or “summarize”.
