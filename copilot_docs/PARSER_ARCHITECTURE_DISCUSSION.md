# Delphi LSP Parser Architecture — Discussion Notes

> Discussion held March 3–4, 2026. This document captures findings, decisions, and open questions from the architectural planning session for adding Delphi source parsing to DDK.

---

## Table of Contents

1. [Current State of DDK](#1-current-state-of-ddk)
2. [What Needs to Be Built](#2-what-needs-to-be-built)
3. [Parser Framework Selection](#3-parser-framework-selection)
4. [The Compiler Directive Problem](#4-the-compiler-directive-problem)
5. [Three-State Directive Resolution Model](#5-three-state-directive-resolution-model)
6. [Preprocessor Output Format](#6-preprocessor-output-format)
7. [Parallel Parsing Strategy](#7-parallel-parsing-strategy)
8. [Async + Parallel: tokio + rayon](#8-async--parallel-tokio--rayon)
9. [Fire-and-Forget Indexing](#9-fire-and-forget-indexing)
10. [Cache Architecture](#10-cache-architecture)
11. [String Interning](#11-string-interning)
12. [CLI Strategy for Parser Development](#12-cli-strategy-for-parser-development)
13. [Dependencies to Add](#13-dependencies-to-add)
14. [Open Questions](#14-open-questions)
15. [Appendix: dproj-rs Capabilities](#appendix-a-dproj-rs-capabilities)
16. [Appendix: Delphi Compiler Directives Reference](#appendix-b-delphi-compiler-directives-reference)
17. [Appendix: Existing Parsing in the Ecosystem](#appendix-c-existing-parsing-in-the-ecosystem)

---

## 1. Current State of DDK

DDK is a Delphi Development Kit for VS Code (v2.0.5), structured as a Rust + TypeScript monorepo:

```
┌──────────────────────────────────────────────────────────────┐
│                    VS Code Extension (TypeScript)             │
│   Projects Tree • Compiler UI • DFM Support • MCP Client     │
└───────────┬──────────────────────┬───────────────────────────┘
            │ LSP (stdio)          │ MCP (stdio)
            ▼                      ▼
┌───────────────────┐  ┌───────────────────────┐   ┌──────────┐
│  ddk-server (LSP) │  │ ddk-mcp-server (MCP)  │   │ ddk CLI  │
│  tower-lsp based  │  │  rust-mcp-sdk based   │   │ clap     │
└────────┬──────────┘  └────────┬──────────────┘   └────┬─────┘
         │                      │                       │
         └──────────────┬───────┴───────────────────────┘
                        ▼
              ┌──────────────────┐
              │    ddk-core      │
              │  Shared library  │
              └──────────────────┘
```

### What EXISTS today

| Area | Implementation | Notes |
|------|---------------|-------|
| `.dproj` XML parsing | `dproj-rs` crate (external) | Uses `chumsky` + `roxmltree`. Full MSBuild condition evaluation. |
| `.groupproj` XML parsing | Manual `roxmltree` | Extracts `ItemGroup/Projects[@Include]` |
| MSBuild output parsing | Regex in `diag.rs` | Parses `File(Line,Col): Kind Code: Message` |
| `.dproj` cache | `HashMap<usize, Dproj>` in `dproj.rs` | Cache with invalidation per project or bulk |
| Compiler orchestration | `compiler.rs` | Spawns MSBuild, streams output, publishes LSP diagnostics |
| State persistence | RON files with `fslock` | `projects.ron`, `compiler_configurations.ron` |
| File watching | `notify` crate | Watches RON state files for cross-process sync |
| Encoding | `encoding_rs` | Full OEM/CP850/CP437/Windows codepage support |
| DFM → PAS navigation | Regex in TypeScript | Handler name → `procedure TForm.Handler` |
| Formatting | Delegates to Embarcadero's `Formatter.exe` | Not building our own |
| LSP capabilities | `ServerCapabilities::default()` | **No document sync, no textDocument features** |

### What does NOT exist

- ❌ No Delphi source code lexer/tokenizer
- ❌ No AST / syntax tree
- ❌ No grammar definition
- ❌ No uses clause parsing
- ❌ No unit dependency graph
- ❌ No `{$IFDEF}` / `{$DEFINE}` / `{$INCLUDE}` handling
- ❌ No semantic analysis / symbol table
- ❌ No go-to-definition, find-references, completion, hover
- ❌ No `textDocument/didOpen`, `didChange`, `didClose`
- ❌ No git branch switch detection

---

## 2. What Needs to Be Built

### Pipeline Overview

```
Source File (.pas/.dpr/.dpk/.inc)
        │
        ▼
┌──────────────┐
│  Lexer       │  logos — fast DFA tokenizer
│  (stateless) │  Directives are first-class tokens
└──────┬───────┘
       │ Token stream with Spans
       ▼
┌──────────────────┐
│  Preprocessor    │  Evaluates {$IFDEF}, {$IF}, {$INCLUDE}
│  (Tier 1+2)      │  Uses DefineContext from dproj + compiler
│                  │  Resolves includes via search paths
│                  │  Defers Tier 3 ({$IF Declared(X)})
└──────┬───────────┘
       │ Annotated token stream + resolution map
       ▼
┌──────────────────┐
│  Parser          │  Hand-written recursive descent
│                  │  Builds serializable AST
│                  │  Both branches of conditionals preserved
└──────┬───────────┘
       │ AST with ConditionalBlock nodes
       ▼
┌──────────────────┐
│  Index Builder   │  Extracts interface stubs
│  (parallel)      │  Builds unit dependency graph
│                  │  Resolves Tier 3 directives (second pass)
└──────┬───────────┘
       │ UnitIndex + DepGraph
       ▼
┌──────────────────┐
│  LSP Features    │  Document symbols, goto-def, references,
│  (on demand)     │  completion, hover, rename...
└──────────────────┘
```

### Proposed Module Structure

```
core/src/parser/
    mod.rs              — public API, re-exports
    lexer.rs            — logos-based tokenizer
    tokens.rs           — Token enum, Span, SyntaxKind
    preprocessor.rs     — directive evaluation (Tier 1+2+3)
    define_context.rs   — DefineContext construction from dproj + compiler
    parser.rs           — recursive-descent parser
    ast.rs              — AST node types (rkyv-serializable)
    interner.rs         — lasso ThreadedRodeo lifecycle
    index.rs            — parallel indexing, UnitIndex, UnitStub
    dep_graph.rs        — unit dependency graph (DAG)
    resolver.rs         — unit name → file path resolution
    cache.rs            — disk cache (rkyv + mmap + bitcode)
    symbols.rs          — symbol table types
```

---

## 3. Parser Framework Selection

### Evaluated Options

| Option | Verdict | Rationale |
|--------|---------|-----------|
| **tree-sitter** | ❌ Rejected | No maintained Delphi grammar exists (`tree-sitter-pascal` repos return 404). Directive handling would require a separate preprocessing layer that fights tree-sitter's model. |
| **logos + chumsky** | ⚠️ Partial | `chumsky` is already a transitive dep via `dproj-rs`. Good for expression parsing (directive conditions). Not ideal as the main parser — combinator overhead for a grammar this large. |
| **logos + hand-written recursive descent** | ✅ Selected | Maximum control over directive interleaving. Proven approach (rust-analyzer uses this for Rust). |
| **rowan (lossless CST)** | 🔄 Under consideration | Structural sharing for incremental edits of open files. NOT serializable. See [Open Questions](#10-open-questions). |
| **oak-delphi** | ❌ Rejected | v0.0.5, immature, docs don't build, unclear directive support, 239 downloads total. |

### Selected Stack

- **Lexer**: `logos` — compile-time DFA, extremely fast, handles Delphi tokens + directive classification
- **Preprocessor**: Walks token stream, emits structured `Vec<OneOf<Token, ConditionalBlock>>` before parsing
- **Parser**: Hand-written recursive descent consuming preprocessed tokens, emitting custom AST types
- **Expression evaluator** (for `{$IF}`): Small chumsky-based or hand-written Pratt parser
- **Syntax tree**: Custom `#[derive(Archive)]` AST types (serializable via rkyv). Rowan may be used as an additional layer for open files only.

---

## 6. Preprocessor Output Format

### Why Preprocess Before Parsing?

Compiler directives can appear after **any single token**. If the parser checks for directives after every token, it becomes unmaintainable:

```rust
fn parse_something(&mut self) -> Result<AstNode> {
    let tok = self.consume_token()?;
    self.check_for_directive_after_token()?;  // ← after EVERY token
    let next = self.consume_token()?;
    self.check_for_directive_after_token()?;  // ← after EVERY token
    // ... repeating N times per file
}
```

Instead, **preprocess directives into a structured tree before parsing.** The parser never sees raw directive tokens — only a clean hierarchical input.

### PreprocessedToken Format

```rust
enum PreprocessedToken {
    Token(Token),
    ConditionalBlock {
        condition: DirectiveCondition,
        resolution: DirectiveResolution,     // Resolved(bool) or Deferred
        if_branch: Vec<PreprocessedToken>,   // recursive
        else_branch: Option<Vec<PreprocessedToken>>,
    },
}
```

### Why This Architecture Is Superior

1. ✅ **Directives evaluated exactly once** — in the preprocessor, not scattered through parser
2. ✅ **Parser is clean** — doesn't care about directives, just recurses through token tree
3. ✅ **Both branches preserved** — even `Resolved(true)` branches store `else_branch` (needed for IDE features)
4. ✅ **Nested conditionals work naturally** — recursive `Vec<PreprocessedToken>` structure handles nesting
5. ✅ **Testable** — preprocessor tested independently; parser consumes synthetic inputs

---

## 4. The Compiler Directive Problem

This is the central architectural challenge. Compiler directives can appear after ANY token and fundamentally alter which code is visible to the parser.

### Directive Tiers

| Tier | Directives | Evaluable Without Parsing? | Examples |
|------|-----------|---------------------------|----------|
| **Tier 1 — Static** | `{$IFDEF}`, `{$IFNDEF}`, `{$IF Defined()}`, `{$IF CompilerVersion >= N}`, `{$IF SizeOf(Pointer) = N}` | ✅ Yes — only needs `DefineContext` | `{$IFDEF DEBUG}`, `{$IF CompilerVersion >= 33.0}` |
| **Tier 2 — Include** | `{$I file}`, `{$INCLUDE file}` | ✅ Yes — needs search path resolution + file I/O | `{$I MySettings.inc}` |
| **Tier 3 — Semantic** | `{$IF Declared(X)}`, `{$IF SizeOf(TMyRecord)}`, `{$IF MyConst > 5}` | ❌ No — needs partial symbol table | `{$IF Declared(TMyNewType)}` |

### DefineContext Construction

The `DefineContext` is **immutable per project configuration** and built from three sources:

**Source 1 — Compiler-implicit symbols** (from `CompilerConfiguration`):

| Symbol | Source | Example |
|--------|--------|---------|
| `VERxxx` | `compiler_config.condition` | `VER360` for Delphi 12 Athens |
| `CompilerVersion` | `compiler_config.compiler_version` as float | `36.0` |
| `RTLVersion` | Same as CompilerVersion | `36.0` |
| `UNICODE` | Always defined for Delphi 2009+ | — |
| `CONDITIONALEXPRESSIONS` | Always defined for Delphi 6+ | — |
| `MSWINDOWS`, `WIN32`/`WIN64` | From `Project.active_platform` | — |
| `CPUX86`/`CPUX64` | From `Project.active_platform` | — |
| `CONSOLE` | From dproj project type | — |

Full platform symbol table:

| Platform | Defined Symbols |
|----------|----------------|
| Win32 | `MSWINDOWS`, `WIN32`, `CPUX86` |
| Win64 | `MSWINDOWS`, `WIN64`, `CPUX64` |
| Linux64 | `LINUX`, `LINUX64`, `CPUX64` |
| Android | `ANDROID`, `ANDROID32`, `CPUARM`, `CPUARM32` |
| Android64 | `ANDROID`, `ANDROID64`, `CPUARM`, `CPUARM64` |
| iOSDevice64 | `IOS`, `IOS64`, `CPUARM`, `CPUARM64` |
| OSX64 | `MACOS`, `MACOS64`, `CPUX64` |
| OSXARM64 | `MACOS`, `MACOS64`, `CPUARM`, `CPUARM64` |

**Source 2 — Project-level defines** (from `dproj-rs`):

```rust
let pg = dproj.active_property_group()?;
let defines: Vec<&str> = pg.dcc_options.define
    .as_deref()
    .unwrap_or("")
    .split(';')
    .collect();
// e.g., ["DEBUG", "MSWINDOWS", "MY_CUSTOM_DEFINE"]
```

**Source 3 — Built-in constant table**:

| Constant | Type | Value Source |
|----------|------|-------------|
| `CompilerVersion` | Float | `compiler_version` field |
| `RTLVersion` | Float | Same as CompilerVersion |
| `SizeOf(Pointer)` | Integer | 4 for 32-bit platforms, 8 for 64-bit |
| `SizeOf(Integer)` | Integer | 4 (always in modern Delphi) |
| `SizeOf(Extended)` | Integer | Platform-dependent (10 on x86, 8 on x64) |

### `{$IF}` Expression Capabilities

The `{$IF}` evaluator must handle:

| Expression | Example | Complexity |
|------------|---------|------------|
| `Defined(symbol)` | `{$IF Defined(DEBUG)}` | Tier 1 — lookup in define set |
| Boolean operators | `{$IF Defined(A) and not Defined(B)}` | Tier 1 — trivial |
| Numeric comparison | `{$IF CompilerVersion >= 33.0}` | Tier 1 — lookup in constant table |
| `SizeOf(type)` for built-in types | `{$IF SizeOf(Pointer) = 8}` | Tier 1 — lookup in constant table |
| `Declared(identifier)` | `{$IF Declared(TMyType)}` | **Tier 3** — needs symbol table |
| `SizeOf(user_type)` | `{$IF SizeOf(TMyRecord) = 16}` | **Tier 3** — needs type layout |
| Arithmetic | `{$IF (CompilerVersion * 10) >= 330}` | Tier 1 — evaluate in constant table |
| User constants | `{$IF MyConst > 5}` | **Tier 3** — needs constant evaluation |

### `$DEFINE` / `$UNDEF` Scoping Rules

- Scope is **file-level**: a `$DEFINE` in one unit does NOT affect other units
- Runs from the directive to end of file (or until `$UNDEF`)
- **Project-level defines** from `.dproj` are active at start of every file
- `$DEFINE` in an included file **remains active** after the include returns
- Defines inside an inactive branch (`$IFDEF FALSE`) are NOT processed
- `{$LEGACYIFEND ON/OFF}` (exposed by `dproj-rs` as `DccOptions.legacy_ifend`) controls whether `{$IFEND}` vs `{$ENDIF}` closes `{$IF}` blocks

### `{$INCLUDE}` Behavior

- `{$I filename}` / `{$INCLUDE filename}` — textual insertion
- Search order: same directory as current file → `DccOptions.include_path` directories
- **Recursive**: included files can include other files
- **Interacts with conditionals**: `{$IFDEF}/{$ENDIF}` blocks can span include boundaries
- Included file's `{$DEFINE}`s persist after returning to the parent

---

## 5. Three-State Directive Resolution Model

### Core Concept

**Always parse and preserve both branches of every conditional block.** Resolution is metadata, not a structural decision.

```
ConditionalBlock {
    condition: DirectiveCondition,          // raw expression text + parsed form
    resolution: DirectiveResolution,        // Resolved(true) | Resolved(false) | Deferred
    if_branch: Vec<AstNode>,               // always populated
    else_branch: Option<Vec<AstNode>>,      // {$ELSE} / {$ELSEIF} chain, always populated if present
}
```

```rust
enum DirectiveResolution {
    /// Tier 1/2: fully evaluated, we know which branch is active
    Resolved(bool),
    /// Tier 3: needs semantic context we don't have yet
    Deferred,
}
```

### Why preserve both branches?

1. **Config switching**: When the user switches from `Debug` → `Release` (defines change), we re-evaluate conditions against the new `DefineContext` without re-parsing. Only the resolution overlays change.
2. **IDE features**: Inactive branches can be greyed out in the editor. Go-to-definition should still work inside inactive branches (so you can navigate code that's compiled on other platforms).
3. **Deferred resolution**: Tier 3 directives (`Declared()`, user constants) get resolved in a second pass after the index is built. Both branches are already parsed and ready.

### Consumer API

```rust
impl ConditionalBlock {
    /// Returns the active branch based on resolution, or None if deferred
    fn active_branch(&self) -> Option<&[AstNode]> {
        match self.resolution {
            DirectiveResolution::Resolved(true) => Some(&self.if_branch),
            DirectiveResolution::Resolved(false) => self.else_branch.as_deref(),
            DirectiveResolution::Deferred => None, // caller must handle both
        }
    }
}
```

Consumers that need to handle `Deferred`:
- **Symbol table builder**: Walk both branches, mark symbols from each as `conditional`
- **Uses clause extractor**: Collect uses from both branches, mark conditional ones
- **Dependency graph**: Include edges from both branches (conservative — may include unused deps)

After the second pass upgrades `Deferred` → `Resolved`, these can be tightened.

### No separate "partial" vs "complete" AST types needed

One AST type covers all states. The `DirectiveResolution` enum on each `ConditionalBlock` is the only thing that changes between passes.

---

## 6. Parallel Parsing Strategy

### Architecture: Map-Reduce Indexing

Delphi is a **good fit** for the Map-Reduce approach (similar to IntelliJ/Sorbet) because:
- Each `.pas` unit starts with a `unit` declaration → immediate FQN
- `uses` clauses are explicit imports → trivially indexable
- No macros that create new top-level declarations
- "Declaration before use" within a unit
- The `interface` section is essentially a **header** — stubs extractable from interface only

```
┌─────────────────────────────────────────────────┐
│  .dproj parse (dproj-rs)                        │
│  → defines, search paths, config/platform       │
└──────────────────┬──────────────────────────────┘
                   │
         ┌─────────▼─────────┐
         │  Discover files    │  walk unit_search_path
         │  (parallel I/O)    │  + compiler source paths
         └─────────┬─────────┘
                   │
    ┌──────────────▼──────────────┐
    │  Per-file: Lex + Preprocess  │  rayon::par_iter
    │  + Stub-parse (parallel)     │  shared DefineContext (immutable)
    │                              │  each file → UnitStub
    └──────────────┬──────────────┘
                   │
         ┌─────────▼─────────┐
         │  Merge into index  │  single-threaded
         │  UnitName → Stub   │  build dep graph
         └─────────┬─────────┘
                   │
    ┌──────────────▼──────────────┐
    │  Second pass: resolve Tier 3 │  parallel over files with Deferred
    │  directives using index       │  upgrade Deferred → Resolved
    └──────────────┬──────────────┘
                   │
    ┌──────────────▼──────────────┐
    │  On-demand full parse        │  per open file, lazy
    │  + name resolution via index │  triggered by didOpen/didChange
    └─────────────────────────────┘
```

### Parallelism details

- **rayon `par_iter`** for the indexing phase — each file is independent (receives shared immutable `DefineContext`, produces a `UnitStub`)
- **String interner**: `lasso::ThreadedRodeo` is `Send + Sync` — safe to call `get_or_intern()` from rayon workers
- **Include file resolution**: Each worker resolves `{$INCLUDE}` independently. Include files are small and may be read by multiple workers (OS page cache handles this efficiently). Results are cached in a concurrent map.

### Linking resolution

After the parallel phase, merge all stubs into a single `UnitIndex` (`HashMap<Spur, UnitStub>` where `Spur` is an interned unit name). Then resolve `uses` clauses against the `UnitName → FilePath` map to build the **unit dependency graph** (a DAG). This merge is single-threaded but fast (just hashmap insertions).

### RTL/VCL source parsing

The Delphi installation ships ~500K+ LOC of RTL/VCL source. Recommended approach: **parse on demand and cache aggressively**. Only parse RTL units that are actually referenced via `uses` clauses. Cache stubs persistently — RTL source doesn't change between sessions (only on Delphi version upgrade).

### Package (`.dpk`) handling

Since `.dcp` files are opaque binary, for packages where source is available: parse the `.dpk`'s `contains` clause to discover constituent units, then parse those units' interface sections for stubs. Same resolution quality as applications, just sourced differently.

---

## 8. Async + Parallel: tokio + rayon

### Compatibility

**rayon and tokio have no hard conflict** — they operate at different levels:

| Dimension | tokio | rayon |
|-----------|-------|-------|
| **Execution model** | Async (many tasks per thread via green threads) | Sync (one task per OS thread, work-stealing) |
| **Thread pool** | One per runtime instance | Global + configurable |
| **Use case** | I/O-bound (network, files, timers) | CPU-bound (parsing, indexing) |
| **Scheduling** | Task-based, preemptive | Work-based, cooperative |

### Safe Usage Pattern: spawn_blocking

**The idiomatic approach** is to run rayon work inside `tokio::task::spawn_blocking()`:

```rust
// Inside an async LSP handler (tokio context)
async fn handle_config_change(state: &State) -> Result<()> {
    let result = tokio::task::spawn_blocking({
        let project = state.active_project.clone();
        let defines = state.define_context.clone();
        move || {
            // CPU-bound work on rayon thread pool
            (0..num_files).into_par_iter()
                .map(|i| parse_file(i))
                .collect::<Vec<_>>()
        }
    }).await??;
    Ok(())
}
```

### Why This Works

- **rayon threads are isolated** — on separate OS thread pool, don't starve tokio executor
- **tokio remains responsive** — can schedule other async work while rayon computes
- **Natural separation** — CPU-bound (rayon) + I/O-bound (tokio) don't interfere

---

## 9. Fire-and-Forget Indexing

### Problem: Blocking the LSP Handler

If you `await` the spawn_blocking future, the LSP message handler blocks until indexing completes (1-3 seconds). Meanwhile, VS Code can't process other requests — the UI freezes.

### Solution: Background Indexing with Progress Notifications

Don't await the indexing result. **Spawn it as a background task:**

```rust
async fn handle_config_change(state: &State) -> Result<()> {
    let state_clone = state.clone();  // Arc-wrapped, cheap

    // Fire-and-forget: spawn background indexing
    tokio::spawn(async move {
        state_clone.notify_progress("Indexing project...", 0)?;

        match tokio::task::spawn_blocking({
            let project = state_clone.active_project.clone();
            let defines = state_clone.define_context.clone();
            move || index_project_with_rayon(&project, &defines)
        })
        .await
        {
            Ok(Ok(index)) => {
                state_clone.project_index.write().store(index);
                state_clone.notify_progress("Indexing complete", 100)?;
            }
            Ok(Err(e)) => {
                state_clone.notify_error(format!("Index failed: {}", e))?;
            }
            Err(e) => {
                state_clone.notify_error(format!("Spawn failed: {}", e))?;
            }
        }
    });

    // Return immediately to LSP client
    Ok(())
}
```

### Benefits

1. ✅ **LSP stays responsive** — can process other messages while indexing
2. ✅ **Users see progress** — status bar shows "Indexing project..." with percentage
3. ✅ **Fresh results available atomically** — when done, index is updated via `Arc<RwLock<>>`
4. ✅ **Matches production behavior** — rust-analyzer, TypeScript Server, Sorbet all do this

### Stale Reads During Indexing

While background indexing is in progress, LSP features (goto-def, completion) use the old index. This is **acceptable**:
- Results are briefly stale (1-3 seconds)
- UI is always responsive
- Most users find this better than UI freezes

### State Management: Arc<RwLock<T>>

Use `parking_lot::RwLock` (not `std::sync::RwLock`):

```rust
pub struct State {
    // readers don't block writers, O(1) reads
    project_index: Arc<parking_lot::RwLock<ProjectIndex>>,
}

// Background task updates (rare)
state.project_index.write().store(new_index);

// LSP handler reads (frequent, lock-free)
let index = state.project_index.read();
```

---

## 10. Cache Architecture

### Design Principle: Three-Part Separation

**Separate by:**
1. **RTL vs Project** — different update frequencies
2. **AST vs Resolution** — ASTs are directive-agnostic, resolution overlays update independently
3. **Interface vs Implementation** — interface is stable, implementation changes constantly

When user switches from `Debug` to `Release`, the `define_context_hash` changes. Only resolution overlays are invalidated (cheap re-evaluation), NOT the full ASTs.

**RTL-specific insight**: RTL source is immutable (only on Delphi version/patch upgrade). Cache the full AST — never re-parse RTL files.

### Disk Layout: Dual RTL + Project Caches

```
~/.config/ddk/cache/

  # RTL cache — per compiler version, shared across all projects
  rtl_cache/
    25.0/                    ← compiler version
      manifest.bin           ← bitcode: file → content_hash
      strings.bin            ← bitcode: serialized lasso Rodeo (RTL symbols)
      index.bin              ← rkyv: mmap'd RTL unit index
      units/
        {content_hash}.ast   ← rkyv: FULL AST (never re-parse static files)
        {content_hash}.res   ← rkyv: resolution overlay (Tier 3)
    24.0/
      ...

  # Project cache — per project hash + config
  project_cache/
    {project_hash}/
      {define_context_hash}/    ← changes on config/platform switch
        manifest.bin
        strings.bin
        index.bin
        units/
          {content_hash}.ast
          {content_hash}.res
```

### Why Separate RTL from Project?

1. ✅ **Multiple Delphi versions coexist** — Delphi 23, 24, 25 installed side-by-side
2. ✅ **RTL is immutable** — never changes unless Delphi is reinstalled/patched
3. ✅ **Multi-project efficiency** — switch between projects using same compiler → RTL cache shared, only project cache changes
4. ✅ **Cleaner invalidation** — RTL invalidates only on compiler version/patch; project invalidates on source edit or config change
5. ✅ **Lazy loading** — cache other compiler versions on-demand; only active compiler's RTL is eagerly loaded

### Three Tiers of Mutability

Each cache layer has different update frequency and strategy:

| Tier | What | Scope | Storage | Invalidation | Re-parse? |
|---|---|---|---|---|---|
| **Tier 1 — Immutable (RTL)** | Full AST + stub + dep graph | Per compiler version | Disk (rkyv, mmap'd) | On Delphi version/patch only | ❌ Never — full AST cached |
| **Tier 2 — Semi-mutable (Project stubs)** | Stubs + interface AST | Per project + config | Disk (rkyv) | On source edit OR config switch | ✅ Only changed files |
| **Tier 3 — Mutable (Open files)** | Full body ASTs for open files | Per session | Memory only | On every `didChange` | ✅ Full re-parse (fast, <10ms) |

**Rationale**:
- **RTL (Tier 1)**: Never re-parse. Full AST cached. Deserialize and use.
- **Project interface (Tier 2)**: Relatively stable. Cache stubs + interface ASTs. Re-parse on actual file change.
- **Project implementation (Tier 3)**: Changes constantly. Parse on `didOpen`, update on `didChange`, discard on `didClose`. Don't cache.

### I/O Optimization Strategies

For 8M LOC across 2000-3000 units, file I/O is the major bottleneck (~2-3 seconds). Strategies:

| Optimization | Impact | Effort | Implementation |
|---|---|---|---|
| **Sequential prefetch** | +10-15% | Low | Sort files by size descending before rayon; improves disk scheduling |
| **Async file reads + spawn_blocking** | +15-25% | Medium | Read all files first (tokio::fs), then parse (rayon). Needs 1-2 GB memory. |
| **Include file caching** | +5-30% | Low | Cache resolved `.inc` file contents in `DashMap` during preprocessing; hit cache on re-inclusion |
| **Lazy RTL parsing** | +20-30% (if RTL not needed) | Medium | Parse RTL units only when first referenced via uses clause |
| **Incremental indexing** | **60-80%** (huge!) | High | Only re-parse changed files + transitive dependents, not whole project |

**Revised timing estimate for 8M LOC**:
- Baseline parallel (16 cores, no optimization): **2-3 seconds**
- With sequential prefetch: **1.5-2.5 seconds**
- With async I/O + prefetch: **1-2 seconds**
- With all optimizations: **0.5-1 second**

**Recommendation**: Start with **sequential prefetch** — easy win, no architecture changes. Measure on your actual codebase. If faster needed, layer in other optimizations.

### Serialization Format Decisions

**rkyv for ASTs (full + stubs)**: Zero-copy deserialization, mmap-friendly, perfect for large persistent data structures

**bitcode for small data (manifest, interner, configs)**: Compact binary, serde-compatible for schema evolution

**NOT serde for ASTs**: ASTs use rkyv's own `#[derive(Archive, rkyv::Serialize, rkyv::Deserialize)]` to avoid allocation overhead during deserialization (serde allocates into owned types)

**NOT rowan**: `GreenNode` is pointer-rich, can't serialize directly (would need flatten + rebuild on load)

**NOT SQL**: Schema rigidity, query overhead, no zero-copy, heavy dependency

**NOT RON**: Text-based; ~5-10x slower than binary, ~3-5x larger files (fine for config, not for AST caches)

### Invalidation Strategy

| Trigger | Action |
|---------|--------|
| **Single file edit** (`didChange`) | Invalidate that file's AST cache + resolution overlay. Re-lex, re-preprocess, re-parse. Then walk reverse dep graph edges to invalidate transitive dependents' stubs (if interface changed). |
| **Config/platform switch** | `define_context_hash` changes → invalidate ALL resolution overlays (re-evaluate directives, no re-parse). Rebuild index stubs where resolution changes affected interface-level declarations. |
| **dproj change** (search paths or defines) | Same as config switch + re-resolve unit file paths (search paths may have changed). |
| **Git branch switch** | Detect via file watcher on `.git/HEAD` or bulk `didChange` storm (>N files in T seconds). Pause incremental updates → full re-index with rayon → resume. Show "Re-indexing..." via `notifications/compiler/progress`. |
| **DDK version upgrade** | `CACHE_VERSION` in manifest doesn't match → discard entire cache directory, rebuild from scratch. |

### Memory Budget Estimate

For ~2000 units averaging 3000 LOC:

| Component | In-Memory | On-Disk |
|-----------|-----------|---------|
| Unit index (stubs) | ~2-4 MB | ~2-4 MB (mmap'd, same bytes) |
| Dep graph | ~200 KB | ~200 KB |
| String interner | ~5-10 MB | ~5-10 MB |
| Full ASTs (open files only, ~5-10 files) | ~50-100 MB | — |
| Per-file AST cache (all files) | — (on disk only) | ~100-400 MB |
| **Total in memory** | **~60-115 MB** | |
| **Total on disk** | | **~110-415 MB** |

Full ASTs are loaded on `didOpen` and evicted on `didClose`. Only stubs + dep graph + interner are resident.

### Why NOT rowan for persistence

rowan's `GreenNode` is a pointer-rich DAG using `triomphe::ThinArc` (custom atomic reference counting). It:
- Does NOT implement `Serialize`/`Deserialize`
- Cannot be serialized due to pointer-based identity and node deduplication via `NodeCache`
- Would require a "walk and flatten" step to serialize, then rebuild via `GreenNodeBuilder` on load

### Why NOT SQL

- Schema rigidity fights the evolving AST structure
- Query overhead for tree-structured data
- No zero-copy — every query allocates
- Adds a heavy dependency (SQLite)

### Why NOT RON (current state format)

- Text-based: ~5-10x slower than binary, ~3-5x larger files
- Fine for small config files (current use), not for AST caches

---

## 11. String Interning

### Why intern strings?

For an 8M LOC codebase with millions of identifier references:
- A `String` is 24 bytes (pointer + length + capacity) + heap allocation
- A `Spur` (lasso's key type) is 4 bytes, no heap allocation
- Unit names, symbol names, file paths are heavily repeated across uses clauses, definition sites, and reference sites
- Estimated savings: hundreds of MB

### Selected: `lasso::ThreadedRodeo`

| Feature | lasso | string-interner |
|---------|-------|-----------------|
| Thread-safe interner | ✅ `ThreadedRodeo` | ❌ No thread safety |
| Serialization | ✅ serde (feature `serialize`) | ❌ None |
| Read-only mode | ✅ `RodeoReader` (zero-overhead) | ❌ |
| Key types | `Spur` (32-bit) | `DefaultSymbol` (32-bit) |
| Parallel interning | ✅ `&ThreadedRodeo` is `Send + Sync` | ❌ Requires `&mut` |

`string-interner` was rejected because it lacks thread safety and serialization — both critical for parallel rayon indexing and cache persistence.

### Lifecycle

```
Cold Start:
  strings.bin exists?
    YES → deserialize Rodeo from bitcode → convert to ThreadedRodeo
          All Spur keys in cached ASTs/indexes remain valid
    NO  → create fresh ThreadedRodeo

Indexing Phase:
  ThreadedRodeo shared across rayon workers
  Workers call get_or_intern() concurrently (lock-free reads, sharded writes)

Steady State:
  Convert to RodeoReader (zero-overhead, lock-free reads)
  All LSP features use RodeoReader::resolve(&key) → &str

Shutdown / Checkpoint:
  Convert back to Rodeo
  Serialize to strings.bin via bitcode
```

### What gets interned

- **Unit names**: `SysUtils`, `System.Classes`, `Vcl.Forms`, etc.
- **Symbol names**: type names, function names, variable names, constant names
- **File paths**: used in diagnostics, cache keys, dep graph edges
- **Directive symbols**: `DEBUG`, `RELEASE`, `MSWINDOWS`, etc.

All AST nodes and index entries store `Spur` instead of `String`.

---

## 12. CLI Strategy for Parser Development

### Why a `ddk parse` CLI Command?

1. **Decouples parser development from LSP** — develop/test parser in isolation without async/tokio complexity
2. **Builds RTL cache offline** — pre-warm caches before users open IDE
3. **Validates projects** — diagnostic reports on project parsing health
4. **Debugs directives** — `--show-directives` flag shows directive evaluation per line
5. **Operational visibility** — cache statistics, circular dependency detection, unused unit reports

### Proposed Commands

#### `ddk parse rtl [OPTIONS]`

Pre-build RTL cache for a compiler version.

```bash
ddk parse rtl --version 12.0                    # Build RTL cache
ddk parse rtl --list-versions                   # Show installed compilers
ddk parse rtl --version 12.0 --validate         # Verify cache integrity
```

Example output:
```
RTL (Delphi 12.0 Athens)
  Source: C:\Program Files\Embarcadero\Studio\12.0\source\rtl\
  Files: 487 | LOC: 523,421 | Parsing time: 2.34s

Cache Location: ~/.config/ddk/cache/rtl_cache/12.0/
  manifest.bin: 12 KB | strings.bin: 8.2 MB | index.bin: 3.4 MB | units/: 73.1 MB

Status: ✓ Cache built successfully
```

#### `ddk parse project [OPTIONS]`

Parse a Delphi project and build/validate its index cache.

```bash
ddk parse project ./MyProject.dproj
ddk parse project ./MyProject.dproj --force-rebuild
ddk parse project ./MyProject.dproj --format json --output report.json
```

Example output:
```
Project: MyProject.dproj
  Compiler: Delphi 12.0 (Athens) | Platform: Win64 | Config: Release
  Root: D:\Projects\MyProject\

Parse Results:
  Files discovered: 247 | Files indexed: 247 (1.45s) | Errors: 0

Unit Index:
  Units: 203 | With uses: 201 | Avg LOC: 2,845 | Max LOC: 8,243 (Engine.pas)

Dependency Graph:
  DAG edges: 412 | Circular deps: 0 | Unused units: 5

Cache:
  Location: ~/.config/ddk/cache/{project_hash}/
  Size: 287 MB | Status: ✓ Valid
```

#### `ddk parse file [OPTIONS]`

Parse a single file and show AST/directives (for debugging).

```bash
ddk parse file ./Main.pas --project ./MyProject.dproj
ddk parse file ./Main.pas --show-directives    # Show {$IF}, {$IFDEF} evaluation
ddk parse file ./Main.pas --show-uses           # Show resolved uses clauses
```

#### `ddk parse check [OPTIONS]`

Lightweight syntax validation.

```bash
ddk parse check ./MyProject.dproj               # Quick validation
ddk parse check ./Main.pas --file-only          # Syntax only
```

### Implementation Benefits

1. ✅ **Parser development is isolated** — no LSP protocol complexity
2. ✅ **Test harness** — snapshot-test CLI output
3. ✅ **RTL pre-warming** — users don't see "Parsing RTL" on first IDE open
4. ✅ **Debugging** — `--show-directives` helps diagnose parser issues
5. ✅ **Progressive** — start with `parse project` and `parse rtl`; add features later

### Recommended First Milestone

**Add `ddk parse` BEFORE building full LSP features.** It:
- Forces disciplined parser architecture
- Provides instant feedback loop
- Pre-builds RTL cache for fast LSP cold starts
- Gives users operational visibility

---

## 13. Dependencies to Add

```toml
# core/Cargo.toml additions

[dependencies]
# Lexer
logos = "0.14"

# String interning (parallel + serializable)
lasso = { version = "0.7", features = ["multi-threaded", "serialize"] }

# Zero-copy serialization for ASTs and indexes
rkyv = { version = "0.8", features = ["bytecheck"] }

# Memory-mapped files for instant cold starts
memmap2 = "0.9"

# Compact binary serialization for small data (manifest, interner, configs)
bitcode = { version = "0.6", features = ["serde"] }

# Parallel file indexing
rayon = "1.10"

# Better RwLock for state management (readers don't block writers)
parking_lot = "0.12"

# Already present — no changes needed:
# serde, tokio, notify, fslock, encoding_rs, dproj, roxmltree
```

### Considered but deferred

| Crate | Purpose | Why Deferred |
|-------|---------|-------------|
| `rowan` | Lossless CST for open files | See [Open Questions](#10-open-questions) — may add later for incremental re-parsing of live edits |
| `salsa` | Incremental computation framework | Major architectural commitment. Evaluate after profiling whether re-computation is a bottleneck. |
| `bumpalo` | Arena allocator for scratch computation | Only useful for temporary per-request buffers. Add if profiling shows allocation pressure. |

---

## 14. Open Questions

### Q1: Rowan — dual representation or single?

**Option A — Single representation (custom rkyv AST only)**:
- Simpler: one tree type everywhere
- Fully serializable
- Incremental re-parsing of open files requires rebuilding the full AST from scratch on every edit

**Option B — Dual representation (rkyv AST for cache + rowan CST for open files)**:
- Rowan gives structural sharing: edit one node, share the rest — efficient incremental updates
- But: two tree types to maintain, conversion between them
- Rowan CST derived from rkyv AST on `didOpen`, discarded on `didClose`

**Decision needed**: Is the complexity of dual representation worth the incremental re-parsing benefit? For files averaging 3000 LOC, full re-parse takes ~1-5ms — fast enough that incremental may not matter.

### Q2: `{$IF Declared(X)}` frequency

**Need to grep the 8M LOC codebase** for:
- `{$IF Declared` — how common?
- `{$IF SizeOf(` with user types — how common?
- `{$IF` with user constants — how common?

If Tier 3 directives are rare (mainly RTL feature detection), the "defer + fallback to else" strategy is sufficient. If pervasive, the parser needs a running symbol table during first pass (complicates parallelism — files with Tier 3 directives may need serial processing after deps are indexed).

### Q3: Cache size tolerance

~100-400 MB on disk for 2000 units. Acceptable? Could add LRU eviction for ASTs of files not in the active project's dependency closure.

### Q4: First LSP feature target

Recommended progression:
1. **Document Symbols** (outline view) — proves the parser works
2. **Unit dependency graph** (uses clause web) — project-specific
3. **Go to Definition** (unit-level first: `uses SysUtils` → jump to `SysUtils.pas`)
4. **Go to Definition** (symbol-level: `SysUtils.ExtractFileName` → jump to declaration)
5. **Find References** / **Hover** / **Completion**

### Q5: $ELSEIF chains

`{$IF}/{$ELSEIF}/{$ELSEIF}/{$ELSE}/{$ENDIF}` — these form a chain, not a binary branch. The AST node needs to support:

```rust
ConditionalBlock {
    branches: Vec<ConditionalBranch>,  // [{$IF expr, nodes}, {$ELSEIF expr, nodes}, ...]
    else_branch: Option<Vec<AstNode>>, // {$ELSE} nodes
    active_branch_index: Option<usize>, // which branch is active (None if Deferred)
}
```

### Q6: Error recovery strategy

The parser must be fault-tolerant — users are editing code that doesn't compile. Need to decide on recovery strategy:
- **Synchronization tokens**: on error, skip to next `;`, `end`, `begin`, `procedure`, `function`, etc.
- **Error nodes**: insert `ErrorNode` in the AST, continue parsing
- How does this interact with conditional blocks that contain syntax errors in the inactive branch?

---

## Appendix A: dproj-rs Capabilities

### API Surface

| Method | Returns | Purpose |
|---|---|---|
| `Dproj::from_file(path)` | `Result<Dproj>` | Load + parse from disk |
| `active_configuration()` | `Option<&str>` | Default config (e.g. `"Debug"`) |
| `active_platform()` | `Option<&str>` | Default platform (e.g. `"Win32"`) |
| `configurations()` | `Vec<Configuration>` | All build configs |
| `platforms()` | `Vec<Platform>` | All platforms + active flag |
| `active_property_group()` | `Result<PropertyGroup>` | **Merged** PG for default config/platform |
| `property_group(config, platform)` | `Result<PropertyGroup>` | Merged PG for explicit config/platform |
| `main_source()` | `Option<PathBuf>` | Resolves `<MainSource>` (.dpr/.dpk) |
| `exe_output()` / `exe_output_for()` | `Option<PathBuf>` | Resolves output executable path |
| `directory()` | `Option<&Path>` | Parent dir of the .dproj |

### `DccOptions` Fields Relevant to Parsing

| Field | Type | What It Is |
|---|---|---|
| `define` | `Option<String>` | Semicolon-separated `$DEFINE` list (e.g. `"DEBUG;MSWINDOWS"`) |
| `unit_search_path` | `Option<String>` | `DCC_UnitSearchPath` with `$(Var)` expanded |
| `include_path` | `Option<String>` | `DCC_IncludePath` for `{$I}` resolution |
| `namespace` | `Option<String>` | Default namespace prefixes |
| `unit_alias` | `Option<String>` | Unit aliasing rules |
| `legacy_ifend` | `Option<String>` | `{$LEGACYIFEND}` switch |

### `DprojBuilder` for Environment Resolution

```rust
DprojBuilder::new()
    .rsvars_file(r"C:\...\rsvars.bat")?  // seeds $(BDS), $(BDSCOMMONDIR), etc.
    .env_var("key", "value")             // manual overrides
    .from_file("MyProject.dproj")?;
```

`rsvars.bat` provides: `$(BDS)`, `$(BDSCOMMONDIR)`, `$(FrameworkDir)`, `$(FrameworkVersion)`, etc.

### What dproj-rs Does NOT Do

- Does NOT parse Pascal source code
- Does NOT provide built-in VERxxx symbols (those come from the compiler, not the .dproj)
- Does NOT resolve `{$INCLUDE}` files
- Does NOT provide a list of all source files (gives you search paths, you walk them)
- The `define` field gives project-level defines only — not compiler-implicit ones

---

## Appendix B: Delphi Compiler Directives Reference

### All Conditional Compilation Directives

| Directive | Purpose |
|---|---|
| `{$DEFINE symbol}` | Define a conditional symbol |
| `{$UNDEF symbol}` | Undefine a conditional symbol |
| `{$IFDEF symbol}` | Compile if symbol is defined |
| `{$IFNDEF symbol}` | Compile if symbol is NOT defined |
| `{$IF expression}` | Compile if expression is true |
| `{$ELSEIF expression}` | Alternative branch for `$IF` |
| `{$ELSE}` | Else branch |
| `{$ENDIF}` | End conditional block |
| `{$IFEND}` | End `{$IF}` block (legacy mode) |
| `{$IFOPT switch}` | Compile based on compiler switch state |

### Other Directives With Parsing Relevance

| Directive | Effect on Parsing |
|---|---|
| `{$I filename}` / `{$INCLUDE filename}` | Textual file inclusion |
| `{$R filename}` / `{$RESOURCE filename}` | Resource file reference (no parse effect) |
| `{$L filename}` / `{$LINK filename}` | Object file link (no parse effect) |
| `{$SCOPEDENUMS ON/OFF}` | Changes enum member scoping |
| `{$LEGACYIFEND ON/OFF}` | Controls `{$IFEND}` vs `{$ENDIF}` matching |
| `{$REGION 'name'}` / `{$ENDREGION}` | Code folding markers (no semantic effect) |
| `{$WARN id ON/OFF/ERROR}` | Warning control |

### Built-In Predefined Symbols (Always Defined by Compiler)

| Category | Symbols |
|---|---|
| Version | `VER190` through `VER370` (one per compiler version) |
| Platform | `MSWINDOWS`, `LINUX`, `MACOS`, `IOS`, `ANDROID` |
| Architecture | `WIN32`, `WIN64`, `CPUX86`, `CPUX64`, `CPUARM`, `CPUARM32`, `CPUARM64` |
| Features | `UNICODE`, `CONDITIONALEXPRESSIONS`, `NATIVECODE`, `ASSEMBLER` |
| Compiler mode | `CONSOLE`, `NEXTGEN` (deprecated 10.4+) |

---

## Appendix C: Existing Parsing in the Ecosystem

### tree-sitter-pascal

Both known repositories (`AquaBx/tree-sitter-pascal`, `nickg/tree-sitter-pascal`) return **404** — no maintained grammar exists. Not viable.

### oak-delphi (crates.io)

- v0.0.5, published Oct 2025
- Green/Red tree architecture (Roslyn-inspired)
- Claims: units, classes, generics, attributes, fault-tolerant recovery
- **BUT**: docs.rs build fails, unclear directive handling, 239 downloads, not recommended as dependency

### rust-analyzer's approach

- Hand-written recursive descent parser emitting rowan `GreenNode`s
- **Does NOT persist ASTs or indexes to disk** — rebuilds everything on startup
- Uses salsa for intra-session incremental computation
- Cold start: loads all files into VFS, parses everything, runs `cargo check` for proc macros
- Gets away with no disk cache because Rust parsing is fast and cargo handles the heavy lifting

**Implication for DDK**: DDK has no equivalent of `cargo check` — all analysis is done by the LSP itself. Disk caching has higher ROI for DDK than for rust-analyzer.

---

*End of discussion notes. Next step: formalize into an implementation plan with task breakdown and milestones.*
