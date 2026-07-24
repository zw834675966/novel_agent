# Structured Action + Plot Reason Slot (Platform reverse N3/N4)

> **Status:** N3 Implemented 2026-07-22 | N4 In Progress

**Goal:** Eliminate free-text AI narration patterns (P7晋江红线) and causal explanation prose (P1番茄签约话术).

## Platform Signals Addressed

| # | Platform Signal | Mitigation |
|---|-----------------|------------|
| P1 | Plot reason / memory free text (AI causal explanations) | N4: PlotReasonSlot enum - short fact slots |
| P7 | Action free text - AI formulaic narration | N3: StructuredAction enum - stage direction level |

---

## N3: StructuredAction - 结构化动作模板 (Done)

**Problem:** Free-text action strings allow LLM to generate formulaic narration patterns like "她缓缓起身" / "他低头思索" - these are the exact晋江 narrative-level AI red flags.

**Solution:** Replace `String` action with enum-based structured action templates that enforce stage-direction constraint:
- Subject + Verb + Target + Optional Dialogue
- No psychological description allowed
- No environmental description allowed
- No causal fillers stripped automatically at render

### Implementation

**File:** `src/models/action.rs`

```rust
pub enum ActionKind {
    // Movement
    MoveToward, MoveAway, Enter, Exit, StandUp, SitDown, Turn, StepForward, StepBack,
    // Observation
    LookAt, GlanceAt, StareAt, PeerAt,
    // Physical
    Reach, Touch, Hold, Drop, Bow, Nod, ShakeHead, Frown, Smile, Sigh,
    // Speech
    Say, Ask, Answer, Whisper, Shout, Exclaim,
    // Interaction
    Grab, Push, Pull, Help,
    // Other
    Pause, Hesitate, Search, Inspect,
}

pub struct StructuredAction {
    kind: ActionKind,
    target: Option<String>,  // ≤6 chars, auto-cleaned
    dialogue: Option<String>, // ≤30 chars, auto-cleaned
}
```

**Integrated into:** `src/prose/contract.rs` → `NarrativeBeat.action` changed from `String` to `StructuredAction`.

### Render Rules

```rust
// Pattern: Subject + Verb [+ Target] [+ dialogue]
"她" + "看向" + "尸体" + "道：" + "这是谁？" → "她看向尸体道：这是谁？"
"她" + "道：" + "此事蹊跷。" → "她道：此事蹊跷。"
```

### Tests Added

- `structured_action_render_simple` - basic render without dialogue
- `structured_action_render_with_dialogue` - non-speech action with dialogue
- `structured_action_speech_only` - pure speech action render
- `structured_action_cleanup_fillers` - causal fillers stripped at render time
- `all_action_kinds_have_verbs` - every variant has Chinese verb defined

### Integration Points Updated

1. `src/prose/assembly.rs` - `render_action` replaces `sanitize_action` for beats
2. `tests/prose_test.rs` - all fixture actions updated to StructuredAction
3. `src/models/mod.rs` - export new types

---

## N4: PlotReasonSlot - 情节原因短槽 (Next Priority)

**Problem:** `PlotDevelopment.reason` and `CharacterMemory.content` are still free strings, allowing LLM to generate AI-style causal explanations that trigger platform P1 signals.

**Solution:** Replace `String` reason with enum-based slot system that only allows short fact descriptions:

```rust
pub enum PlotReasonSlot {
    Observed(String),    // saw X → "见血迹"
    Heard(String),       // heard X → "闻异响"
    NoticedAnomaly(String), // found abnormal X
    BehaviorOdd(String),  // X acted strangely
    ContextChanged(String), // situation X changed
    CharacterState(String), // X's state changed
    DialogueContent(String), // mentioned X in dialogue
    PhysicalEvidence(String), // evidence X
    Other(String),       // fallback, X ≤20 chars
}
```

### Implementation Plan

**Phase 1 - Data model:**
- [x] Define `PlotReasonSlot` enum in `src/models/action.rs`
- [ ] Update `PlotDevelopment` in `src/models/plot.rs`: `reason: PlotReasonSlot` (break change)
- [ ] Update `CharacterMemory` in `src/models/memory.rs`: consider structured memory slots

**Phase 2 - LLM contract:**
- [ ] Update `LlmCharacterDerivation` in `src/llm/contract.rs` to use structured types
- [ ] Update `LlmNarrative` contract if needed
- [ ] Rig prompt schema changes will enforce LLM output structure

**Phase 3 - Persistence:**
- [ ] Serialization/deserialization for SQLite (current `reason: String` stored as JSON)
- [ ] DB schema may not need change since PlotDevelopment is serialized as JSON anyway

**Phase 4 - Render:**
- [x] `PlotReasonSlot.render()` returns short Chinese fact description
- [ ] Prose assembly: use rendered reasons when summarizing plot developments

---

## Test Results

```
cargo test --lib --test prose_test --test e2e --test scene_test --test vocab_test

Result: 34 + 5 + 6 + 9 + 1 + 19 = 74 tests passed
```

### N3 Tests Added (8)

- 7 unit tests in `models::action::tests`
- 1 integration test in prose_test.rs (emotion resolve)
- All backwards compatible within constraint

---

## Files Changed

| File | Change |
|------|--------|
| `src/models/action.rs` | NEW: StructuredAction + ActionKind + PlotReasonSlot |
| `src/models/mod.rs` | Export new types |
| `src/prose/contract.rs` | NarrativeBeat.action: String → StructuredAction |
| `src/prose/assembly.rs` | render_action + is_speech_kind helper |
| `tests/prose_test.rs` | All fixtures updated to StructuredAction |

---

## Next Steps (N4 Full)

1. Integrate `PlotReasonSlot` into `PlotDevelopment` model
2. Update LLM contract and prompts
3. Add memory structured slots if applicable
4. Run full verification and update reverse research doc status

## Verification Command

```bash
cargo test --lib --test prose_test --test e2e --test scene_test --test vocab_test --test db_test
```
