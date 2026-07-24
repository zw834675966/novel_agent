# Task 3 Report: Assemble preserves sensation_refs order (kill sense sort)

## Status: DONE

## What was implemented

Removed the fixed 8-category sense sort from `assemble_beat_descriptions` in
`src/prose/assembly.rs`. Legal quotes are now joined in **input `refs`
encounter order** (camera-beat order) instead of being re-collaged as
atmosphere -> visual -> auditory -> olfactory -> tactile -> gustatory ->
emotion -> gesture. This is the core Phase 1 fix for coherent shot ordering.

### Core change

In `assemble_beat_descriptions`, the `by_cat` HashMap + fixed `order` iteration
was replaced with a single direct iteration over `resolved`:

```rust
// Preserve ref input order (camera-beat order); do NOT sort by sense category.
// `resolved` is already in ref-encounter order, and the source-isolation
// `retain` above is stable, so relative order survives.
let parts: Vec<String> = resolved.iter().map(|q| q.text.clone()).collect();
let quotes = parts.clone();
let desc = join_quotes_with_rhythm(&parts);
(desc, stripped, quotes)
```

`resolved` is already in ref-encounter order (the resolution loop pushes in
`refs` order), and the book-source isolation uses `Vec::retain`, which is
stable, so relative order survives the isolation step. Nothing else in the
function changed: the ref-resolution loop, the allowed-set filter, the
`VocabularyId` parse, and `dominant_book_source` + `retain` isolation are all
unchanged.

### Supporting change: removed now-write-only `sense` field

Removing the sense sort left `ResolvedQuote.sense` written but never read,
which trips clippy's `dead_code` lint (a hard failure under `-D warnings`).
The field was removed from the `ResolvedQuote` struct and from the resolve
push. `dominant_book_source` / `book_source` only read `source`, so behavior is
unchanged.

### Doc comment updates

- `assemble()` rule 4: `按固定 8 类顺序拼装` -> `按 sensation_refs 出现顺序拼装(Hard 原句;镜头序由 beat 序列表达)`; rule 5 reworded to note ref order is naturally preserved.
- `assemble_beat_descriptions` doc: `按类别有序拼装` -> `按 sensation_refs 出现顺序拼装`, with an explicit "顺序：保持 refs 遇到顺序（镜头序），不按感官类别重排" bullet.

## Files changed

- `src/prose/assembly.rs` (+48 / -34)

No other files were staged, reverted, reformatted, or deleted. The wider
worktree has many pre-existing unrelated modified/untracked files (user work
from other tasks); these were left untouched.

## TDD evidence

### RED (Step 1-2): new failing test

Wrote `assemble_preserves_ref_order_not_sense_order` first. It feeds refs in
`[visual.bloodstain, atmosphere.coldnight]` order (visual before atmosphere)
and asserts `血迹` appears before `夜凉如水`. Ran it against the old code:

```
$ cargo test --lib prose::assembly::tests::assemble_preserves_ref_order_not_sense_order -- --nocapture
thread '...' panicked at src\prose\assembly.rs:565:9:
ref order must win over sense order: 夜凉如水，血迹。她看向地面
test result: FAILED. 0 passed; 1 failed
```

The old sense-sort produced `夜凉如水，血迹` (atmosphere first) - exactly the
behavior the test is designed to catch.

### GREEN (Step 3-5): implement fix + update existing test

Implemented the fix (removed sense sort). Renamed
`orders_all_eight_categories` -> `preserves_ref_input_order_all_categories`
and updated its assertion to expect **input order**
(gesture -> emotion -> gustatory -> tactile -> olfactory -> auditory -> visual
-> atmosphere). The input was already in reverse sense order, so the old
assertion (`夜凉如水，血迹，…，她伸手把帕子绞了又绞。她进入房间`) had to flip to
(`她伸手把帕子绞了又绞，心中未免悔恨，苦涩，冰凉的手，血腥味，脚步声，血迹，夜凉如水。她进入房间`).

All other assembly tests use single-sense refs or `contains` checks, so they
passed unchanged.

```
$ cargo test --lib prose::assembly -- --nocapture
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 56 filtered out
```

Full lib suite also green:

```
$ cargo test --lib
test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured
```

### Test coverage

New / updated tests in `assembly.rs`:

- `assemble_preserves_ref_order_not_sense_order` (NEW) - visual-before-atmosphere
  ref order is preserved (would have failed under the old sense sort).
- `preserves_ref_input_order_all_categories` (RENAMED from
  `orders_all_eight_categories`) - all 8 categories joined in ref-input order
  with exact-string assertion.

The other 18 assembly tests (beat order, stripping, source isolation, rhythm,
provenance, KPIs, density) pass unchanged.

## Quality gates

- `cargo test --lib prose::assembly -- --nocapture` -> **PASS** (20 passed).
- `cargo test --lib` -> **PASS** (76 passed).
- `cargo fmt --all -- --check` -> **PASS** (clean).
- `cargo clippy --all-targets --all-features -- -D warnings` -> **PASS** (no
  warnings; the `sense` field removal was specifically required to keep this
  green).

## Commit

```
fb8c86b fix(prose): join quotes in ref order instead of sense order
 src/prose/assembly.rs | 82 ++++++++++++++++++++++++++++++---------------------
 1 file changed, 48 insertions(+), 34 deletions(-)
```

Staged with an explicit path (`git add src/prose/assembly.rs`); no
`git add -A` / `git add .`. `git show --stat` confirms only
`src/prose/assembly.rs` is in the commit. Base commit was `943a16a`.

## Constraints honored

- Only `src/prose/assembly.rs` modified and staged.
- `join_quotes_with_rhythm`, `book_source_isolation`, `dominant_book_source`,
  and the `assemble()` signature are unchanged.
- No unrelated worktree files touched.

## Self-review findings

1. **Order correctness** - `resolved` preserves ref-encounter order by
   construction (the loop `for raw in refs` pushes in order); `retain` is
   documented stable; the new `resolved.iter().map(...)` cannot reorder. The
   new test and the renamed all-categories test both pin exact output strings.
2. **No behavior drift** - source isolation still runs before the join and
   still increments `stripped`; `dominant_book_source` still reads `.source`
   only, so removing `sense` is behavior-neutral.
3. **Clippy cleanliness** - the only non-mechanical edit beyond the brief was
   dropping the write-only `sense` field; this is necessary (not optional) to
   satisfy `clippy -D warnings` and keeps the struct honest.
4. **No concerns.** Build, fmt, clippy, and focused tests are green.
