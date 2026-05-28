# TFS: TUI Architecture Refactor — Split main.rs into explicit modules

## Overview

The Coherence TUI crate (`coherence-core-db-tui`) has crossed the "vibecoded prototype" line:
`src/main.rs` mixes project discovery, AppState, tree flattening, key handling, DB persistence,
external editor integration, and ratatui rendering in one file (~1100 lines). The next feature
(explicit entity editing lifecycle: draft → validate → save/cancel) and the subsequent Dolt-backed
branch/diff review will be painful if persistence and UI mutation remain tangled.

This refactoring preserves all current user-visible behavior while cutting the dependency
web so new features can be added without rewriting the TUI again.

## Target module layout

```
src/
  main.rs              — bootstrap only (terminal init, event loop, effects dispatch)
  app.rs               — AppState, Screen, Focus, selected entity state
  action.rs            — AppAction enum
  update.rs            — state transition logic (pure: AppState + Action → AppState + Vec<Effect>)
  effects.rs           — DB/editor/filesystem side effects
  project_discovery.rs — discover_projects via `find`
  tree.rs              — TreeItem, build_tree, toggle_expand, select logic
  edit.rs              — Draft model, validation, save/cancel session
  ui/
    mod.rs             — pub fn ui() and sub-module re-exports
    tree.rs            — render_tree
    detail.rs          — render_detail
    pickers.rs         — render_project_picker, render_env_picker
    theme.rs           — Theme struct, THEME constant
    helpers.rs         — title_line, textwrap_indent, general UI helpers
```

## Architecture rules

1. `main.rs` — initializes terminal, constructs dependencies, runs event loop,
   dispatches effects, restores terminal. No business logic.
2. `action.rs` — `AppAction` enum maps every key input / event to a named action.
3. `update.rs` — `fn update(app: &mut AppState, action: AppAction) -> Vec<Effect>`.
   Pure state transitions. No I/O. Returns effects for side-effecting work.
4. `effects.rs` — executes returned effects: DB reads/writes via SpecRepository,
   editor subprocess, filesystem ops.
5. `ui/` modules — read-only access to `&AppState`. Never mutate state.
6. Repository boundary — DB access wraps `spec_store` behind a trait so update.rs
   never calls `spec_store` directly. Tests use a fake in-memory repository.
7. `edit.rs` — Draft model (copied entity + pending changes), validation,
   save/cancel lifecycle, separate from loaded SpecGraph.

## Action/Effect protocol

```rust
enum AppAction {
    // Navigation
    SelectProject(usize),
    SelectEnv(usize),
    NavUp,
    NavDown,
    EnterPressed,
    Back,           // Esc
    Quit,
    // Tree
    FocusTree,
    FocusDetail,
    ToggleExpand,
    // Edit
    EnterEditMode,
    CycleField(FieldKind),  // status, level, review_mode, risk_level
    OpenEditor,
    SaveDraft,
    CancelEdit,
}

enum Effect {
    LoadGraph(String, String),   // project_path, env
    PersistSpec(Spec),
    PersistAc(AcceptanceCriterion),
    OpenExternalEditor(String, String),  // spec_id or ac_id, content
    LoadGraphAndRefresh(String, String),
    None,
}
```

## Draft model (edit.rs)

```rust
enum EditSession {
    Spec(Spec),
    Ac(AcceptanceCriterion),
}

impl EditSession {
    fn start(entity: &Spec) -> Self           // copy from graph
    fn start(entity: &Ac) -> Self
    fn validate(&self) -> Result<(), Vec<String>>
    fn apply(&mut self, field: FieldKind, value: String)
    fn into_spec(self) -> Option<Spec>         // consume on save
    fn into_ac(self) -> Option<AcceptanceCriterion>
}
```

- Draft starts as a copy of the loaded entity (Spec or AC).
- Field mutations go onto the draft, never the graph.
- Save validates → if valid, persists (via Effect) → on success, updates graph.
- Cancel drops the draft without touching DB or graph.

## Migration from current code

Each step is a separate commit with tests:

| Step | Module | What moves | Test strategy |
|------|--------|------------|---------------|
| 1 | `tree.rs` | TreeItem, build_tree, toggle_expand, update_preview | Unit test: tree flattening from known graph produces expected items |
| 2 | `action.rs` + `update.rs` | AppAction enum, update fn extracting key dispatch | Unit test: given state + action, verify state changes + effects emitted |
| 3 | `effects.rs` | spec_store calls, editor run, load_graph | Integrate with real DB or fake repo in test |
| 4 | `edit.rs` | Draft model, field apply, validate, save/cancel | Pure unit tests: validate, apply field, into_entity |
| 5 | `ui/` | All render_* functions, title_line, textwrap_indent | Snapshot tests via ratatui (future) |
| 6 | `project_discovery.rs` | discover_projects | Test with temp dirs and fake git roots |
| 7 | `app.rs` | AppState, Screen, TreeItem types moved from main | Compile-check, no behavior change |
| 8 | `main.rs` | Strip down to bootstrap + loop + dispatch | Smoke test: binary starts and quits |

## Non-goals

- No new user-visible features added during refactoring.
- No ratatui snapshot testing (ratatui 0.29 `assert_buffer` is available but out of scope).
- No Dolt branch/diff review in M0 (deferred to M1 after this refactor lands).
