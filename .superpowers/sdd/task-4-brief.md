### Task 4: NarrateRequest carries plan; NarrativeBeat optional camera_beat_id

**Files:**
- Modify: `src/prose/generator.rs` (add `plan` field to `NarrateRequest`)
- Modify: `src/prose/contract.rs` (add `camera_beat_id` field to `NarrativeBeat`)
- Modify: `src/prose/plan_contract.rs` (add `minimal()` helper)
- Modify: `src/prose/mock.rs` (add `camera_beat_id` to NarrativeBeat literal)
- Modify: `src/prose/assembly.rs` (add `camera_beat_id` to test helper)
- Modify: `src/prose/rig_impl.rs` (add `plan` to 2 test helpers + `camera_beat_id` if any)
- Modify: `src/scene/service.rs` (add `plan` to NarrateRequest construction - placeholder)
- Modify: `tests/prose_test.rs` (add `camera_beat_id` to 3 NarrativeBeat literals)
- Modify: `tests/bm25_provenance_real_test.rs` (add `camera_beat_id` to 2 NarrativeBeat literals)

**Base commit:** `fb8c86b`

**Goal:** Thread `LlmScenePlan` through `NarrateRequest` so the prose generator can see the camera-beat plan. Add `camera_beat_id` to `NarrativeBeat` so each beat can reference its plan beat. This is a plumbing task - no behavioral changes yet (Task 5/6 wire the real flow).

---

## Changes

### 1. `src/prose/contract.rs` - Add `camera_beat_id` to `NarrativeBeat`

Add after `sensation_refs`:
```rust
/// 对应 `LlmScenePlan` 中 `CameraBeat::beat_id`（空字符串 = 未关联镜头）。
#[serde(default)]
pub camera_beat_id: String,
```

The `#[serde(default)]` means existing JSON without this field still deserializes correctly.

### 2. `src/prose/generator.rs` - Add `plan` to `NarrateRequest`

Add `use super::plan_contract::LlmScenePlan;` (or `use super::LlmScenePlan;` if already re-exported).

Add field:
```rust
pub struct NarrateRequest {
    pub scene: Scene,
    pub characters: Vec<Character>,
    pub derivations: Vec<CharacterDerivation>,
    pub candidates: Vec<CharacterProseCandidates>,
    pub plan: LlmScenePlan,
}
```

### 3. `src/prose/plan_contract.rs` - Add `minimal()` helper

Add an `impl LlmScenePlan` block with a minimal plan constructor for test/placeholder use:

```rust
impl LlmScenePlan {
    /// 构造最小计划：一个 act + 每个角色名一个镜头。
    /// Task 4 中作为 service.rs 占位；Task 5 替换为真实 ScenePlanner 输出。
    /// 测试中用于构造 NarrateRequest。
    pub fn minimal(event: &str, character_names: &[&str]) -> Self {
        let premise: String = event.chars().take(40).collect();
        let camera_beats: Vec<CameraBeat> = character_names
            .iter()
            .enumerate()
            .map(|(i, name)| CameraBeat {
                beat_id: format!("b{}", i + 1),
                pov_name: (*name).to_string(),
                intent: format!("回应：{}", premise),
                must_show: vec![],
                location_hint: String::new(),
            })
            .collect();
        let on_stage: Vec<String> = character_names.iter().map(|n| (*n).to_string()).collect();
        LlmScenePlan {
            outline: StoryOutline {
                premise_one_liner: premise.clone(),
                acts: vec![OutlineAct {
                    act_id: "a1".into(),
                    summary: premise,
                    emotional_beat: "推进".into(),
                    stakes: "未定".into(),
                }],
            },
            scene_card: SceneCard {
                when: "场景当下".into(),
                where_place: "未标注".into(),
                on_stage,
                camera_beats,
            },
        }
    }
}
```

### 4. Update all `NarrateRequest { ... }` construction sites

**`src/scene/service.rs` (~line 444):** Add placeholder plan:
```rust
let req = NarrateRequest {
    scene: scene.clone(),
    characters: characters.clone(),  // may need clone for plan construction
    derivations: sorted_derivations,
    candidates: sorted_candidates,
    plan: LlmScenePlan::minimal(
        &scene.objective_event,
        &characters.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
    ),
};
```
Note: `characters` is already available before this point. Check if it's moved or borrowed - if moved, clone for the plan. The plan is a **placeholder** - Task 5 replaces this with a real `ScenePlanner::plan_scene()` call.

**`src/prose/rig_impl.rs` (~line 141, `minimal_request()`):** Add:
```rust
plan: LlmScenePlan::minimal("事件", &["甲"]),
```

**`src/prose/rig_impl.rs` (~line 162, `request_with_unsorted_candidates()`):** Add:
```rust
plan: LlmScenePlan::minimal("深夜来访", &["甲", "乙"]),
```

### 5. Update all `NarrativeBeat { ... }` construction sites

Add `camera_beat_id: String::new(),` (or `camera_beat_id: "".into()`) to each:

**`src/prose/mock.rs` (~line 49):** MockProseGenerator fallback beat.
**`src/prose/assembly.rs` (~line 473):** Test helper `narrative()` in the `map` closure.
**`tests/prose_test.rs` (~lines 139, 148, 281):** Three test fixture beats.
**`tests/bm25_provenance_real_test.rs` (~lines 160, 249):** Two test fixture beats.

### 6. Verify compilation

Run: `cargo check --all-targets`
Expected: success (all construction sites updated).

Run: `cargo test --all-targets`
Expected: all existing tests still pass (no behavioral change - `camera_beat_id` defaults to empty, `plan` is a placeholder).

### 7. Commit

```bash
git add src/prose/generator.rs src/prose/contract.rs src/prose/plan_contract.rs src/prose/mock.rs src/prose/assembly.rs src/prose/rig_impl.rs src/scene/service.rs tests/prose_test.rs tests/bm25_provenance_real_test.rs
git commit -m "feat(prose): thread LlmScenePlan through NarrateRequest"
```

---

## What NOT to change

- Do NOT modify `NarrateRequest` consumers to actually USE the `plan` field yet (Task 5/6 does that).
- Do NOT change `MockProseGenerator::narrate` behavior - it still returns one beat per character (Task 6 changes this).
- Do NOT change `RigProseGenerator::build_prompt` to include plan content (Task 6 does that).
- Do NOT add a `ScenePlanner` field to `StoryService` (Task 5 does that).
- Do NOT touch unrelated worktree files.
- Do NOT remove `#[serde(default)]` from `camera_beat_id` - it must stay for JSON backward compat.

## Quality gates

- `cargo check --all-targets` - clean compile
- `cargo test --all-targets` - all existing tests pass
- `cargo fmt --all -- --check` - clean
- `cargo clippy --all-targets --all-features -- -D warnings` - clean
