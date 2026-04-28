# `on_entity_property_changed` Branch Notes

This branch carries a Rust-side implementation of a Clarity-like entity property change hook.

It is intended to be easy to rebase onto upstream parser updates, so this document focuses on:
- what changed
- where conflicts are most likely during upstream sync
- what can be safely adjusted later

Important semantic note:
- the branch keeps Clarity-style `class_pattern` / `property_pattern` syntax
- named patterns use real regex matching again
- exact-class single-string form still uses exact string comparison

## Goal

Add an observer callback that fires once per relevant property change:

- on entity creation: once for each populated property currently present on the entity
- on entity update: once for each changed property in that packet
- on entity deletion: no property-change callback

Supported observer macro forms:

```rust
#[on_entity_property_changed("CDOTA_PlayerResource")]
```

```rust
#[on_entity_property_changed(
    class_pattern = "CDOTA_Unit_Hero_.*",
    property_pattern = "m_iHealth|m_iMaxHealth"
)]
```

## Public API Added

Observer trait:

```rust
fn on_entity_property_changed(
    &mut self,
    ctx: &Context,
    entity: &Entity,
    field_path: &FieldPath,
) -> ObserverResult
```

New interest flag:

```rust
Interests::TRACK_ENTITY_PROPERTY
```

Additional public helpers:

- `FieldPath` is now public
- `Entity::get_property_by_field_path(&FieldPath)`
- `Entity::field_paths() -> Vec<FieldPath>`
- `Class::field_name_for_path(&FieldPath) -> String`

## Implementation Map

### 1. Observer surface

Files:
- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/lib.rs`

Changes:
- added `TRACK_ENTITY_PROPERTY`
- added `Observer::on_entity_property_changed(...)`
- added hidden helper exports under `source2_demo::__private` for macro-generated filtering

### 2. Macro support

File:
- `source2-demo-macros/src/lib.rs`

Changes:
- added `#[on_entity_property_changed(...)]`
- parser only accepts:
  - `("ExactClassName")`
  - `(class_pattern = "...", property_pattern = "...")`
- macro generates a per-handler `OnceLock<EntityPropertyPatternFilter>`
- exact class-name form uses exact matching
- named-argument form keeps clarity-style `class_pattern` / `property_pattern` syntax
- named-argument form uses regex matching and caching

### 3. Runtime filter and caches

File:
- `source2-demo/src/parser/observer.rs`

Added:
- `PatternKind`
- `EntityPropertyPatternFilter`
- class match cache: `class -> bool`
- property match cache: `class -> (FieldPath -> bool)`

Notes:
- cache key is class name string plus decoded `FieldPath`
- `FieldPath` was made `Eq + Hash + PartialEq` for this
- property pattern checks use `Class::field_name_for_path(...)`
- pattern matching uses compiled regexes with caching

### 4. Field-path propagation from decoder

File:
- `source2-demo/src/reader/field/mod.rs`

Changes:
- `FieldReader::read_fields(...)` now returns the number of changed field paths
- added `FieldReader::field_paths(count)` to read back the decoded path buffer

This is the key low-level change that makes property-level callbacks possible.

### 5. Entity event dispatch

File:
- `source2-demo/src/parser/demo/svc.rs`

Changes:
- `entity_created(...)`
  - still performs normal entity setup
  - still fires `on_entity(Created, ...)`
  - then enumerates all populated field paths from entity state
  - fires `on_entity_property_changed(...)` once per field
- `entity_updated(...)`
  - collects changed field paths from `FieldReader`
  - still fires `on_entity(Updated, ...)`
  - then fires `on_entity_property_changed(...)` once per changed field
- `entity_deleted(...)`
  - unchanged for property callback purposes

## Files Changed in This Branch

- `source2-demo/Cargo.toml`
- `source2-demo/src/lib.rs`
- `source2-demo/src/entity/class.rs`
- `source2-demo/src/entity/field/mod.rs`
- `source2-demo/src/entity/field/path.rs`
- `source2-demo/src/entity/mod.rs`
- `source2-demo/src/parser/demo/svc.rs`
- `source2-demo/src/parser/observer.rs`
- `source2-demo/src/reader/field/mod.rs`
- `source2-demo-macros/src/lib.rs`

## Upstream Sync Risk Areas

When rebasing or merging upstream parser changes, check these areas first:

### High-risk

- `source2-demo/src/parser/demo/svc.rs`
  - entity create/update flow is commonly touched by parser changes
  - if upstream changes entity decode order, preserve this invariant:
    - decode entity state first
    - fire `on_entity(...)`
    - then fire `on_entity_property_changed(...)`

- `source2-demo/src/reader/field/mod.rs`
  - upstream changes to field decoding may alter how changed paths are buffered
  - preserve `read_fields(...) -> usize` or re-expose equivalent changed-path info

- `source2-demo-macros/src/lib.rs`
  - macro parser is large and central
  - conflicts are likely if upstream adds more event macros or refactors argument parsing

### Medium-risk

- `source2-demo/src/parser/observer.rs`
  - conflicts if new interests or observer methods are added upstream

- `source2-demo/src/entity/*`
  - conflicts if public entity API is reorganized

## Invariants To Preserve

- `on_entity_property_changed(...)` must only see post-update entity state
- created entities must emit populated properties, not raw serializer schema
- deleted entities must not emit property-change callbacks
- exact-class form and regex form must remain distinct
- named-argument filtering must stay cached, not recompute property-name matches every callback
- named-argument matching semantics must remain full regex semantics

## Pattern Semantics

The named-argument form:

```rust
#[on_entity_property_changed(
    class_pattern = "...",
    property_pattern = "..."
)]
```

currently supports:

- full regex syntax supported by the Rust `regex` crate

Examples that are supported:

- `CDOTA_Unit_Hero_.*`
- `CDOTA_NPC_Observer_Ward.*`
- `m_hItems.*`
- `m_hAbilities.*`
- `CBodyComponent.m_vec.*`
- `m_vecDataTeam.*.m_iNetWorth`

Because matching is regex-based again, alternation like `foo|bar` and other standard regex constructs are available.

## Validation Performed

Verified on this branch:

- `cargo check`
- `cargo test --lib`

Full `cargo test` still has unrelated pre-existing doctest failures in the repository.

## Entity Snapshotting Notes

The current `source2-demo` side now exposes:

```rust
Entity::field_paths() -> Vec<FieldPath>
```

This is intentionally narrow. It gives downstream consumers access to the
entity's currently populated field paths without forcing a particular snapshot
format into the parser itself.

Primary intended uses:

- full entity snapshot export
- whitelist/blacklist-based snapshot export
- debugging which populated fields exist on a live entity

Recommended usage pattern in downstream code:

1. call `entity.field_paths()`
2. optionally filter the returned paths
3. resolve display names with `entity.class().field_name_for_path(...)`
4. read values with `entity.get_property_by_field_path(...)`

## Performance Notes From Downstream Payload Experiments

The downstream `onNetWorthChanged` payload experiment showed a clear boundary:

- enumerating all populated fields is feasible
- materializing large object payloads for every event is not
- the dominant cost is total exported field count, not individual scalar conversion

Measured downstream outcomes:

- no-op baseline around `553ms` for the sampled run
- full entity snapshot logic around `99954ms` before field-name caching
- around `76703ms` after caching `FieldPath -> field name`
- full-field export and broad blacklist variants remained far too slow for practical use

What this means for `source2-demo`:

- keeping `Entity::field_paths()` is still useful
- caching field-name lookup is worthwhile
- but parser-side support for full-path enumeration should be treated as a building block, not as a recommendation to emit huge JSON objects per event

Recommended downstream strategy:

- prefer a small whitelist over a broad blacklist
- attach entity snapshots only to a narrow set of events
- if many fields must be exported, prefer a compact array/binary representation over large JS objects

## Recommended Workflow For Future Upstream Updates

If upstream parser changes are frequent:

1. Keep this work on its own branch: `on_entity_property_changed`
2. Commit property-change work as 1-3 small commits instead of one large mixed commit
3. Rebase this branch onto upstream `master`
4. Resolve conflicts in the high-risk files above first
5. Re-run:
   - `cargo check`
   - `cargo test --lib`

### Helper Script

This branch now includes a small helper script:

```bash
scripts/sync-upstream.sh [remote] [base-branch] [feature-branch]
```

Defaults:

- `remote = upstream`
- `base-branch = master`
- `feature-branch = on_entity_property_changed`

What it does:

1. verifies you are on the expected feature branch
2. verifies the working tree is clean
3. fetches the upstream remote
4. rebases the feature branch onto the selected upstream base branch
5. runs:
   - `cargo check`
   - `cargo test --lib`

Example:

```bash
scripts/sync-upstream.sh upstream master on_entity_property_changed
```

If you have not configured an `upstream` remote yet:

```bash
git remote add upstream <UPSTREAM_URL>
```

Then verify:

```bash
git remote -v
```

## About Rust Equivalents To `patch-package`

There is no exact standard Rust equivalent to JavaScript's `patch-package` for an application repo like this one.

Practical options are:

### Option 1. Maintain this as a branch or fork

Best when you directly own the parser repo.

- track upstream with `git remote`
- keep your feature as a rebaseable branch
- merge or rebase when parser updates land

### Option 2. Use git patches

Best when you want a replayable patch series.

- generate with `git format-patch`
- re-apply with `git am` or `git apply`

This is the closest workflow to `patch-package`.

## Performance Note

Current implementation preference:

- exact-class handlers use exact string comparison
- named-pattern handlers use cached compiled regex matching

Why:

- closer compatibility with Clarity semantics
- supports the full pattern space exposed by `class_pattern` / `property_pattern`

Because class and property results are cached, repeated matches on the same class and field path are already cheap. The main extra cost versus wildcard-only matching is the first regex evaluation for a new class or `(class, FieldPath)` pair.

### Option 3. Use Cargo dependency overrides

Best when this parser is consumed from another Rust project.

In the consumer project:

- use a git dependency pointing at your branch/fork, or
- use `[patch.crates-io]` / `[patch."..."]` to override the source

This does not patch the repo in place; it swaps which crate source Cargo resolves.

## Recommendation

For this parser, the cleanest long-term approach is:

- keep upstream remote intact
- keep `on_entity_property_changed` as an isolated branch
- periodically rebase it
- if another project depends on this crate, point that project to this branch or fork via Cargo

If you later want a patch-series workflow, generate `git format-patch` files from this branch rather than trying to mimic npm-style patch injection.
