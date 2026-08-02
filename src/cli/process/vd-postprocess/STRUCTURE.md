# vd-postprocess — project layout

Rust crate: **universal recipe executor** — domain library **and** CLI surface for `use: postprocess` on the shared Executor.

**Status: implemented.** Workspace member: `src/cli/process/vd-postprocess`. Default provider: `stub`.

Related: [README.md](README.md) · [cli.md](cli.md) · [RUST.md](RUST.md) · [../README.md](../README.md) · [../vd-pipeline/](../vd-pipeline/)

---

## Philosophy

```text
ArtifactRef(s)
      +
Recipe  (portable: default Runner + execution graph + outputs + secrets refs)
      +
optional Runner override
      ↓
ExecutionPlan
      ↓
Derived Artifacts
```

Not “prompt + LLM”. A **Recipe** is a portable **execution graph**; an **`ExecutionRunner`** executes a graph node. Job only selects recipes and may override the runner.

- **No built-in recipes** — empty list is an error.
- **Recipes are user/company assets** — packs are portable because the default runner lives *in* the recipe.
- **Inputs are unified InputRef** — `{ artifact }` or `{ from: node.output }`.
- **Recipe = graph** — every node has a Runner (explicit or inherited); multi-step needs no second schema.
- **Nodes without `needs` execute in parallel.**
- **Outputs declare `artifact` + `type`** (+ optional `path`).
- **`variables` ≠ `secrets`** — secrets are refs (`env:…`), never plain keys in packs.
- **`ExecutionPlan` is first-class** — dry-run emits it; execute consumes it.
- **Runner resolution: `CLI > Job > Config > Recipe`** (same wording everywhere).
- **Normal DAG capability** — parallel fan-out across Job steps is first-class.
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
| `preprocess` | media + filters | prepared media |
| `transcribe` | audio | transcript |
| `fix-*` | transcript | transcript |
| `diarize` | audio | timeline |
| `meeting-merge` | tracks + timeline + model | meeting |
| **`postprocess`** | ArtifactRef(s) + recipes (+ optional runner override) | artifacts |

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
│       ├── executor.rs             # build ExecutionPlan + execute (dry-run aware)
│       ├── recipe.rs               # load / validate / render RecipeDoc
│       ├── result.rs               # RecipeResult / DerivedArtifact
│       └── provider.rs             # v0 name → target runner.rs (ExecutionRunner)
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
| `postprocess/executor.rs` | Load recipes → resolve runners (`CLI > Job > Config > Recipe`) → build **ExecutionPlan** → execute |
| `postprocess/recipe.rs` | Full recipe document (graph + outputs + default runner + secrets) |
| `postprocess/result.rs` | `RecipeResult { outputs }` |
| `postprocess/provider.rs` | `ExecutionRunner` backends (v0 module name; target: `runner.rs`) |
| `config/` | Config layer in `CLI > Job > Config > Recipe` |

---

## Domain model (target)

```rust
pub struct PostprocessRequest {
    pub inputs: BTreeMap<String, InputRef>,
    pub recipes: Vec<PathBuf>,
    /// Optional override — wins over Config and Recipe default; CLI wins over this.
    pub runner: Option<RunnerSpec>,
    pub variables: BTreeMap<String, String>,
    pub output_dir: Option<PathBuf>,
    pub overwrite: bool,
}

pub struct RunnerSpec {
    pub r#type: String,                  // openai | ollama | process | http | mcp | …
    pub model: Option<String>,
    pub command: Option<String>,
    pub options: BTreeMap<String, ArgValue>,
}

pub enum InputRef {
    Artifact {
        artifact: String,
        format: Option<String>,
        // selector / segments — later
    },
    From { node: String, output: String },
}

pub struct RecipeDoc {
    pub version: u32,
    pub id: Option<String>,
    pub name: Option<String>,
    pub runner: Option<RunnerSpec>,      // recipe default
    pub inputs: BTreeMap<String, RecipeInputDecl>,
    pub variables: BTreeMap<String, RecipeVarDecl>,
    pub secrets: BTreeMap<String, SecretRef>,  // env:… / vault:… — never plain
    pub outputs: BTreeMap<String, ArtifactOutputDecl>,
    pub graph: Vec<GraphNode>,
}

pub struct GraphNode {
    pub id: String,
    pub runner: Option<RunnerSpec>,      // else inherit resolved recipe default
    pub needs: Vec<String>,              // empty ⇒ parallel with other roots
    pub inputs: BTreeMap<String, InputRef>,
    pub outputs: BTreeMap<String, ArtifactOutputDecl>,
    pub body: NodeBody,                  // Prompt | Command | Http | Mcp | …
    pub foreach: Option<ForeachSpec>,    // reserved
}

pub struct ArtifactOutputDecl {
    pub artifact: String,
    pub r#type: String,                  // markdown | json | …
    pub path: Option<String>,
    pub mime: Option<String>,
    pub schema: Option<PathBuf>,
}

/// First-class — dry-run emits this; execute consumes it.
pub struct ExecutionPlan {
    pub nodes: Vec<ExecutionNode>,
    pub outputs: Vec<ArtifactOutput>,
}

pub struct ExecutionNode {
    pub id: String,
    pub runner: RunnerSpec,              // fully resolved
    pub needs: Vec<String>,
    pub inputs: BTreeMap<String, InputRef>,
    pub body: NodeBody,
    pub outputs: Vec<ArtifactOutput>,
}

pub struct ArtifactOutput {
    pub artifact: String,
    pub r#type: String,
    pub path: PathBuf,
    pub mime: Option<String>,
    pub schema: Option<PathBuf>,
}

pub trait ExecutionRunner {
    fn execute(&self, node: &ExecutionNode, ctx: &RunContext) -> Result<NodeResult, PostprocessError>;
}

pub struct RecipeResult {
    pub recipe_id: Option<String>,
    pub outputs: Vec<DerivedArtifact>,
}

pub struct PostprocessResult {
    pub results: Vec<RecipeResult>,
}
```

Empty `recipes` → usage / exit 2. **Runner resolution: `CLI > Job > Config > Recipe`.**

---

## Recipe document (full)

Executor does not know “summary”. It knows Recipe → **ExecutionPlan** → Artifacts.

```yaml
version: 1
id: summary
name: Meeting summary

runner:
  type: openai
  model: gpt-5
  temperature: 0.2

inputs:
  transcript:
    required: true
  meeting:
    required: false

variables:
  audience: Engineering

secrets:
  jira_token: env:JIRA_TOKEN

outputs:
  summary:
    artifact: summary
    type: markdown
    path: reports/summary.md

graph:
  - id: summarize
    prompt: |
      Summarize for {{ audience }}.
      Transcript: {{ transcript }}
    outputs:
      summary:
        artifact: summary
        type: markdown

  - id: render
    needs: [summarize]
    runner:
      type: process
      command: render.py
    inputs:
      draft:
        from: summarize.summary
```

**Nodes without `needs` execute in parallel.**

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
| Resolve | Non-empty recipes; map named inputs → `InputRef` |
| Bind | Backend binding → `postprocess::executor` |
| Outputs | Register each `ArtifactOutput.artifact` → path |
| Progress | Standard step events |

Until reserved cleared: treat as normal capability once binder lands.

```text
               Transcript
                    │
        ┌───────────┼────────────┐
        ▼           ▼            ▼
 postprocess   postprocess   postprocess
```

---

## Algorithm

Dry-run must print a full **ExecutionPlan** without invoking any `ExecutionRunner`:

```text
collect request (inputs, recipes, variables, runner override)
      ↓
reject if no recipes
      ↓
load recipes
      ↓
resolve runners per node   # CLI > Job > Config > Recipe
      ↓
expand foreach (reserved)
      ↓
validate InputRefs
      ↓
build ExecutionPlan        # nodes + ArtifactOutputs; parallel groups via needs
      ↓
[--dry-run → emit plan → stop]
      ↓
execute plan               # no needs → parallel; else wait on needs
      ↓
for each ready ExecutionNode:
      ExecutionRunner::execute
      write declared outputs
      validate schema / mime when present
      ↓
return PostprocessResult { results: Vec<RecipeResult> }
```

---

## Tests (planned)

| Topic | Proof |
|-------|--------|
| no recipes | exit 2 |
| multi-input | InputRef map; required keys |
| multi-output | one recipe → many `ArtifactOutput`s |
| ExecutionRunner | process/http stub without LLM |
| ExecutionPlan dry-run | plan JSON lists nodes + runners + outputs; no invoke |
| parallel graph | nodes without `needs` scheduled together |
| secrets | `env:` resolved; never logged in plan by default |
| binder | `Capability::Postprocess` → backend binding (lib or CLI) |
| parallel Job | two postprocess steps share transcript artifact id |

```bash
cargo test -p vd-postprocess
./scripts/test.sh vd-postprocess
```

---

## Public contract note

**Recipe + ExecutionRunner + InputRef → ExecutionPlan → Derived Artifacts.**  
Runner priority **`CLI > Job > Config > Recipe`**. Capability name is `postprocess`, not “llm” or “summarize”.