### Task 3: Assemble preserves sensation_refs order (kill sense sort)

**Files:**
- Modify: `src/prose/assembly.rs` (`assemble_beat_descriptions` + doc comments + tests)
- Test: unit tests in `assembly.rs`

**Base commit:** `943a16a`

**Goal:** Stop reordering quotes by fixed sense-category order (atmosphere→visual→…→gesture). Instead, join legal quotes in **input `refs` encounter order**. This is the core Phase 1 fix: assemble in camera-beat/ref order, not sense-sorted collage.

---

## Current code (to change)

### `assemble_beat_descriptions` (line ~322)

Currently after resolving refs and applying source isolation, it sorts quotes by a fixed 8-category order:

```rust
// Lines ~357-384: THIS IS THE SENSE SORT TO REMOVE
let order = [
    "atmosphere", "visual", "auditory", "olfactory",
    "tactile", "gustatory", "emotion", "gesture",
];
let mut by_cat: HashMap<String, Vec<String>> = HashMap::new();
for q in &resolved {
    by_cat.entry(q.sense.clone()).or_default().push(q.text.clone());
}
let mut parts: Vec<String> = Vec::new();
for cat in order {
    if let Some(texts) = by_cat.get(cat) {
        for t in texts { parts.push(t.clone()); }
    }
}
```

### What to change

Replace the above block with direct iteration over `resolved` (which is already in ref-encounter order; source isolation uses `Vec::retain` which is stable, so relative order is preserved):

```rust
// Preserve ref input order (camera-beat order); do NOT sort by sense category.
let parts: Vec<String> = resolved.iter().map(|q| q.text.clone()).collect();
let quotes = parts.clone();
let desc = join_quotes_with_rhythm(&parts);
```

Keep everything else in `assemble_beat_descriptions` unchanged: the ref resolution loop, the allowed-set filter, the VocabularyId parse, the book-source isolation (dominant_book_source + retain). Only the sort step is removed.

### Doc comment update (line ~78)

In the `assemble` method doc comment, change:
```
///   4. 合法 ref 按固定 8 类顺序拼装:atmosphere -> visual -> auditory -> olfactory -> tactile -> gustatory -> emotion -> gesture
```
to:
```
///   4. 合法 ref 按 sensation_refs 出现顺序拼装(Hard 原句;镜头序由 beat 序列表达)
```

---

## Tests

### Step 1: Write new failing test

Add this test to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn assemble_preserves_ref_order_not_sense_order() {
    // vocab: visual.bloodstain="血迹", atmosphere.coldnight="夜凉如水"
    // beat refs: [visual.bloodstain, atmosphere.coldnight]  // visual before atmosphere
    // OLD behavior: atmosphere first -> "夜凉如水…血迹"
    // NEW: preserve ref order -> starts with "血迹"
    let action = crate::models::StructuredAction {
        kind: crate::models::ActionKind::LookAt,
        target: Some("地面".into()),
        dialogue: None,
    };
    let prose = AssembledProse::assemble(
        &narrative(vec![(
            CHARACTER_A,
            action,
            &["visual.bloodstain", "atmosphere.coldnight"],
        )]),
        &sample_vocab(),
        &[derivation(
            CHARACTER_A,
            &["visual.bloodstain", "atmosphere.coldnight"],
        )],
        &participants(&[CHARACTER_A]),
    )
    .unwrap();
    let pos_blood = prose.text.find("血迹").unwrap();
    let pos_night = prose.text.find("夜凉如水").unwrap();
    assert!(
        pos_blood < pos_night,
        "ref order must win over sense order: {}",
        prose.text
    );
}
```

### Step 2: Run new test - expect FAIL

Run: `cargo test assemble_preserves_ref_order_not_sense_order -- --nocapture`
Expected: FAIL (atmosphere sorted before visual, so 夜凉如水 appears before 血迹).

### Step 3: Implement the fix (remove sense sort)

As described above: replace the by_cat sort with direct `resolved` iteration.

### Step 4: Update `orders_all_eight_categories` test

The existing test `orders_all_eight_categories` (line ~509) inputs refs in reverse sense order:
```rust
let ids = [
    "gesture.weep", "emotion.sorrow", "gustatory.bitter", "tactile.coldhand",
    "olfactory.bloodsmell", "auditory.footstep", "visual.bloodstain", "atmosphere.coldnight",
];
```
and asserts the OLD atmosphere-first output:
```rust
assert_eq!(
    prose.text,
    "夜凉如水，血迹，脚步声，血腥味，冰凉的手，苦涩，心中未免悔恨，她伸手把帕子绞了又绞。她进入房间"
);
```

This test must be updated to expect **input order** (gesture first, atmosphere last). Rename it to `preserves_ref_input_order_all_categories` and update the assertion to the new expected text. The implementer should compute the exact expected string by running the test after the fix and using the actual output, then verify it matches ref-input order: gesture → emotion → gustatory → tactile → olfactory → auditory → visual → atmosphere.

**Important:** The `join_quotes_with_rhythm` function inserts `，` between short (≤6 char) lemmas and `。` between longer ones. All sample_vocab texts here are ≤10 chars. Trace the exact separator placement for the new order and assert the precise string.

### Step 5: Check other tests

Review ALL other tests in `assembly.rs` for any that assume atmosphere-first ordering. Most tests use single-sense refs (e.g., only `emotion.sorrow`) so they should be unaffected. The `book_source_isolation_drops_minority_corpus` test uses `contains` checks (not exact position), so it should pass unchanged.

Run: `cargo test --lib prose::assembly -- --nocapture`
Expected: all PASS.

### Step 6: Commit

```bash
git add src/prose/assembly.rs
git commit -m "fix(prose): join quotes in ref order instead of sense order"
```

---

## What NOT to change

- Do NOT modify `join_quotes_with_rhythm` - the rhythm punctuation logic stays.
- Do NOT modify `book_source_isolation` / `dominant_book_source` / `retain` - source isolation stays, just without reordering.
- Do NOT modify `assemble()` main function signature or flow - only `assemble_beat_descriptions` internals change.
- Do NOT modify any other file.
- Do NOT touch unrelated dirty worktree files.
