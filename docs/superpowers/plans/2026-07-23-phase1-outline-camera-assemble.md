# Phase 1: Outline + Camera-Beat Assemble Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before narrating a scene, produce a structured director outline and camera-beat sheet; assemble prose in camera-beat order with **Hard** verbatim quotes (no modern paraphrase), so multi-character scenes read as a coherent shot list instead of sense-sorted collage.

**Architecture:** Add a `ScenePlanner` (trait + mock + rig) that emits `LlmScenePlan` (`StoryOutline` + `SceneCard` with ordered `camera_beats`). `StoryService::narrate_scene` calls the planner first, injects the plan into `NarrateRequest`, and requires `ProseGenerator` to emit one `NarrativeBeat` per camera beat in order. `AssembledProse::assemble` stops reordering quotes by sense category and preserves **ref list order** (fill order). Hard mode: still resolve `VocabularyId` → `entry.text` substring paste + post-assemble provenance scan.

**Tech Stack:** Rust 2024, existing `rig` extractors + `schemars`/`serde`, `StoryService`, Mock/Rig generators, integration tests with mocks only.

**Spec / decisions:** `docs/superpowers/plans/2026-07-23-fill-blank-narrative-pipeline.md` §12  
- 出处：**A Hard**  
- 范围：**Phase 1 only**（无 Align 现代化、无 Critic loop）

## Global Constraints

- **Hard quote:** injected description must be continuous substrings of `VocabEntry.text`; never paraphrase distilled text into final body in this plan.
- Do not remove `validate_selection`, `text_guard`, book-source isolation, or `unverified_quotes` provenance.
- Planner + prose still use structured JSON extractors (`schemars::JsonSchema`); no free-form chapter dump.
- Tests use **Mock** generators only (no live DeepSeek).
- Do not revert or reformat unrelated dirty worktree files; stage only files touched by each task.
- `temperature`/determinism for rig extractors: follow existing prose/llm patterns (`retries(1)` is fine; do not invent new sampling unless already used).
- Chinese user-facing observation strings; English code identifiers.
- Phase 1 does **not** change derive/BM25 retrieval algorithm (optional later).

---

## File map

| Path | Responsibility |
|------|----------------|
| `src/prose/plan_contract.rs` | `LlmScenePlan`, `StoryOutline`, `OutlineAct`, `SceneCard`, `CameraBeat` (JsonSchema) |
| `src/prose/planner.rs` | `ScenePlanner` trait + `PlanRequest` |
| `src/prose/planner_mock.rs` | Deterministic mock plan from scene + participants |
| `src/prose/planner_rig.rs` | DeepSeek extractor for `LlmScenePlan` |
| `src/prose/generator.rs` | Extend `NarrateRequest` with `plan: LlmScenePlan` |
| `src/prose/assembly.rs` | Quote join order = ref order; optional beat alignment helper |
| `src/prose/contract.rs` | Optional `camera_beat_id` on `NarrativeBeat` (string, default empty) |
| `src/prose/rig_impl.rs` | Prompt includes plan; require 1:1 beats |
| `src/prose/mock.rs` | Fallback narrate: one beat per camera beat |
| `src/prose/mod.rs` | Export planner types |
| `src/scene/service.rs` | `scene_planner` field; `narrate_scene` plan→narrate→assemble |
| `src/bootstrap.rs` | Construct mock/rig planner pair with generators |
| `src/bin/run_childhood_rivalry.rs` | Wire planner if it constructs service manually |
| `src/cli/observation.rs` | Optional outline/camera summary fields on narrate success |
| `src/cli/commands.rs` | Pass plan summary into observation after narrate |
| `tests/plan_narrate_test.rs` | Mock closed-loop: plan order + ref order + hard quote |
| `AGENTS.md` / `Claude.md` | One short note on narrate planning Phase 1 |

---

### Task 1: Plan contracts (JsonSchema types)

**Files:**
- Create: `src/prose/plan_contract.rs`
- Modify: `src/prose/mod.rs`
- Test: unit tests in `src/prose/plan_contract.rs`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `pub struct OutlineAct { pub act_id: String, pub summary: String, pub emotional_beat: String, pub stakes: String }`
  - `pub struct StoryOutline { pub premise_one_liner: String, pub acts: Vec<OutlineAct> }`
  - `pub struct CameraBeat { pub beat_id: String, pub pov_name: String, pub intent: String, pub must_show: Vec<String>, pub location_hint: String }`
  - `pub struct SceneCard { pub when: String, pub where_place: String, pub on_stage: Vec<String>, pub camera_beats: Vec<CameraBeat> }`
  - `pub struct LlmScenePlan { pub outline: StoryOutline, pub scene_card: SceneCard }`
  - Note: use `where_place` not `where` (Rust keyword). Serde rename if prompt wants `where`: `#[serde(rename = "where")]` on field `where_place` **or** keep `where_place` in JSON for simplicity (prefer `where_place` everywhere to avoid rename bugs).

- [ ] **Step 1: Write failing compile-free shape test**

Create `src/prose/plan_contract.rs`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OutlineAct {
    pub act_id: String,
    pub summary: String,
    pub emotional_beat: String,
    pub stakes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StoryOutline {
    pub premise_one_liner: String,
    pub acts: Vec<OutlineAct>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CameraBeat {
    pub beat_id: String,
    /// Character display name (planner); service maps to UUID for prose.
    pub pov_name: String,
    pub intent: String,
    #[serde(default)]
    pub must_show: Vec<String>,
    #[serde(default)]
    pub location_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SceneCard {
    pub when: String,
    pub where_place: String,
    #[serde(default)]
    pub on_stage: Vec<String>,
    pub camera_beats: Vec<CameraBeat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LlmScenePlan {
    pub outline: StoryOutline,
    pub scene_card: SceneCard,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_plan_roundtrip_json() {
        let plan = LlmScenePlan {
            outline: StoryOutline {
                premise_one_liner: "破产夜撤离".into(),
                acts: vec![OutlineAct {
                    act_id: "a1".into(),
                    summary: "封条与倒地".into(),
                    emotional_beat: "惊惧".into(),
                    stakes: "家破".into(),
                }],
            },
            scene_card: SceneCard {
                when: "查封当日夜".into(),
                where_place: "工厂门口".into(),
                on_stage: vec!["苏念卿".into()],
                camera_beats: vec![CameraBeat {
                    beat_id: "b1".into(),
                    pov_name: "苏念卿".into(),
                    intent: "看见封条".into(),
                    must_show: vec!["封条".into()],
                    location_hint: "厂门".into(),
                }],
            },
        };
        let s = serde_json::to_string(&plan).unwrap();
        let back: LlmScenePlan = serde_json::from_str(&s).unwrap();
        assert_eq!(back.scene_card.camera_beats[0].beat_id, "b1");
        assert_eq!(back.outline.acts[0].summary, "封条与倒地");
    }
}
```

- [ ] **Step 2: Export from `src/prose/mod.rs`**

```rust
mod plan_contract;
pub use plan_contract::{
    CameraBeat, LlmScenePlan, OutlineAct, SceneCard, StoryOutline,
};
```

- [ ] **Step 3: Run test**

Run: `cargo test -p novels plan_contract -- --nocapture`  
(or `cargo test scene_plan_roundtrip_json -- --nocapture`)  
Expected: PASS (and crate compiles with new module).

- [ ] **Step 4: Commit**

```bash
git add src/prose/plan_contract.rs src/prose/mod.rs
git commit -m "feat(prose): add LlmScenePlan outline and camera-beat contracts"
```

---

### Task 2: ScenePlanner trait + mock

**Files:**
- Create: `src/prose/planner.rs`
- Create: `src/prose/planner_mock.rs`
- Modify: `src/prose/mod.rs`
- Test: `src/prose/planner_mock.rs` unit tests

**Interfaces:**
- Consumes: `LlmScenePlan` from Task 1; `crate::models::{Character, Scene}`
- Produces:
  - `pub struct PlanRequest { pub scene: Scene, pub characters: Vec<Character> }`
  - `#[async_trait] pub trait ScenePlanner: Send + Sync { async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError>; }`
  - `pub struct MockScenePlanner { pub response: Option<LlmScenePlan> }`
  - `MockScenePlanner::fallback()` builds one beat per character, ordered by character name, beat_id `b{i}`, intent from truncated `objective_event`
  - `MockScenePlanner::new(plan)` returns fixed plan

- [ ] **Step 1: Write failing test (mock fallback beat count)**

In `src/prose/planner_mock.rs` (after types exist):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Character, CharacterId, Scene, SceneId};
    use chrono::Utc;
    use uuid::Uuid;

    fn char(name: &str) -> Character {
        Character {
            id: CharacterId(Uuid::new_v4()),
            name: name.into(),
            personality: vec![],
            skills: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn fallback_one_beat_per_character_sorted_by_name() {
        let a = char("苏念卿");
        let b = char("顾承烨");
        let scene = Scene {
            id: SceneId(Uuid::new_v4()),
            objective_event: "破产清算：封条与撤离".into(),
            occurred_at: Utc::now(),
            participant_ids: vec![a.id, b.id],
        };
        let planner = MockScenePlanner::fallback();
        let plan = planner
            .plan_scene(&PlanRequest {
                scene,
                characters: vec![b.clone(), a.clone()],
            })
            .await
            .unwrap();
        assert_eq!(plan.scene_card.camera_beats.len(), 2);
        // sorted by name: 顾 before 苏 (Unicode/lexicographic — assert stable sort by name)
        assert_eq!(plan.scene_card.camera_beats[0].pov_name, "顾承烨");
        assert_eq!(plan.scene_card.camera_beats[1].pov_name, "苏念卿");
        assert!(!plan.outline.premise_one_liner.is_empty());
    }
}
```

**Note:** If `Character` fields differ in this repo, match `src/models/character.rs` exactly (copy from existing tests in `tests/e2e.rs` or `tests/scene_test.rs`).

- [ ] **Step 2: Implement `planner.rs` + `planner_mock.rs`**

```rust
// planner.rs
use crate::models::{Character, Scene, StoryError};
use super::plan_contract::LlmScenePlan;

#[derive(Debug, Clone)]
pub struct PlanRequest {
    pub scene: Scene,
    pub characters: Vec<Character>,
}

#[async_trait::async_trait]
pub trait ScenePlanner: Send + Sync {
    async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError>;
}
```

```rust
// planner_mock.rs — fallback builds:
// outline.premise_one_liner = first 40 chars of objective_event
// one OutlineAct { act_id: "a1", summary: premise, emotional_beat: "推进", stakes: "未定" }
// SceneCard.when = "场景当下", where_place = "未标注"
// on_stage = character names
// camera_beats: sort characters by name; for (i,c) in enumerate:
//   beat_id = format!("b{}", i+1)
//   pov_name = c.name
//   intent = format!("回应：{}", truncated event)
//   must_show = []
//   location_hint = ""
```

- [ ] **Step 3: Export in `mod.rs`**

```rust
mod planner;
mod planner_mock;
pub use planner::{PlanRequest, ScenePlanner};
pub use planner_mock::MockScenePlanner;
```

- [ ] **Step 4: Run test**

Run: `cargo test fallback_one_beat_per_character_sorted_by_name -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/prose/planner.rs src/prose/planner_mock.rs src/prose/mod.rs
git commit -m "feat(prose): add ScenePlanner trait and mock fallback"
```

---

### Task 3: Assemble preserves sensation_refs order (kill sense sort)

**Files:**
- Modify: `src/prose/assembly.rs` (`assemble_beat_descriptions` + doc comments + tests)
- Test: unit tests in `assembly.rs`

**Interfaces:**
- Consumes: existing `assemble` API
- Produces: same `AssembledProse`; **behavior change**: legal quotes joined in **input `refs` order**, not atmosphere→…→gesture. Keep: allowed-set filter, VocabularyId parse, book-source majority filter (apply majority filter **without reordering surviving refs relative to each other**).

- [ ] **Step 1: Write failing test for ref order**

```rust
#[test]
fn assemble_preserves_ref_order_not_sense_order() {
    // vocab: visual.bloodstain="血迹", atmosphere.coldnight="夜凉如水"
    // beat refs: [visual.bloodstain, atmosphere.coldnight]  // visual before atmosphere
    // OLD behavior: atmosphere first → "夜凉如水…血迹"
    // NEW: preserve ref order → starts with "血迹"
    let prose = AssembledProse::assemble(
        &narrative(vec![(CHARACTER_A, look_action(), &["visual.bloodstain", "atmosphere.coldnight"])]),
        &sample_vocab(),
        &[derivation(CHARACTER_A, &["visual.bloodstain", "atmosphere.coldnight"])],
        &participants(&[CHARACTER_A]),
    )
    .unwrap();
    let pos_blood = prose.text.find("血迹").unwrap();
    let pos_night = prose.text.find("夜凉如水").unwrap();
    assert!(pos_blood < pos_night, "ref order must win over sense order: {}", prose.text);
}
```

Adapt helpers already in `assembly.rs` tests (`narrative`, `sample_vocab`, `derivation`, `CHARACTER_A`).

- [ ] **Step 2: Run test — expect FAIL**

Run: `cargo test assemble_preserves_ref_order_not_sense_order -- --nocapture`  
Expected: FAIL (blood after night, or assertion message).

- [ ] **Step 3: Implement order-preserving assemble**

Replace sense-bucket sort in `assemble_beat_descriptions` with:

```rust
// After source isolation retain:
let mut parts: Vec<String> = Vec::new();
for q in &resolved {
    parts.push(q.text.clone());
}
// resolved must stay in original `refs` encounter order.
// Implementation detail: when iterating refs, push to `resolved` in that order;
// after retain for source, use stable retain (Vec::retain is stable).
```

Update the module doc bullet that says "按固定 8 类顺序拼装" → "按 sensation_refs 出现顺序拼装（Hard 原句；镜头序由 beat 序列表达）".

- [ ] **Step 4: Run unit tests in assembly**

Run: `cargo test --lib prose::assembly -- --nocapture`  
Expected: all PASS (fix any test that assumed atmosphere-first order — update those tests explicitly).

- [ ] **Step 5: Commit**

```bash
git add src/prose/assembly.rs
git commit -m "fix(prose): join quotes in ref order instead of sense order"
```

---

### Task 4: NarrateRequest carries plan; NarrativeBeat optional camera_beat_id

**Files:**
- Modify: `src/prose/generator.rs`
- Modify: `src/prose/contract.rs`
- Modify: all constructors of `NarrateRequest` / `NarrativeBeat` (search repo)
- Test: compile via `cargo check` + small unit test if needed

**Interfaces:**
- Produces:
  - `NarrateRequest { ..., pub plan: LlmScenePlan }`
  - `NarrativeBeat { pov, action, sensation_refs, #[serde(default)] pub camera_beat_id: String }`
- Consumes: `LlmScenePlan`

- [ ] **Step 1: Grep and list all `NarrateRequest {` and `NarrativeBeat {` construction sites**

Run: use workspace search for `NarrateRequest` and `NarrativeBeat`.

- [ ] **Step 2: Extend structs**

```rust
// generator.rs
pub struct NarrateRequest {
    pub scene: Scene,
    pub characters: Vec<Character>,
    pub derivations: Vec<CharacterDerivation>,
    pub candidates: Vec<CharacterProseCandidates>,
    pub plan: crate::prose::LlmScenePlan,
}
```

```rust
// contract.rs NarrativeBeat
#[serde(default)]
pub camera_beat_id: String, // empty allowed; prefer plan beat_id
```

- [ ] **Step 3: Fix compile errors at every construction site** with a minimal empty plan only if site is test-local; production path fills real plan in Task 5.

Helper for tests:

```rust
pub fn empty_plan_for_tests() -> LlmScenePlan { /* one act, one beat "b1" */ }
```

Put helper in `plan_contract.rs` under `#[cfg(test)]` **or** as `pub fn minimal_plan(characters: &[Character], event: &str) -> LlmScenePlan` used by mocks.

- [ ] **Step 4: `cargo check --all-targets`**

Expected: success (or only pre-existing baseline failures unrelated to this crate — if Lance etc. is already removed, full check green).

- [ ] **Step 5: Commit**

```bash
git add src/prose/generator.rs src/prose/contract.rs src/prose/mock.rs src/prose/rig_impl.rs tests/
git commit -m "feat(prose): thread LlmScenePlan through NarrateRequest"
```

---

### Task 5: Wire planner into StoryService::narrate_scene + bootstrap

**Files:**
- Modify: `src/scene/service.rs`
- Modify: `src/bootstrap.rs`
- Modify: `src/bin/run_childhood_rivalry.rs` (if it builds `StoryService::new` manually)
- Modify: any test that calls `StoryService::new`
- Test: `tests/plan_narrate_test.rs` (Task 6 fleshes behavior; here at least compile + one smoke)

**Interfaces:**
- Produces:
  - `StoryService { ..., scene_planner: Arc<dyn ScenePlanner> }`
  - `StoryService::new(..., scene_planner: Arc<dyn ScenePlanner>)`
  - `narrate_scene` flow:
    1. load scene + validate derivations (existing)
    2. load characters (existing)
    3. `plan = scene_planner.plan_scene(&PlanRequest { scene, characters }).await?`
    4. validate plan: `camera_beats` non-empty; every `pov_name` matches some character.name (else `StoryError::InvalidNarrationContext`)
    5. build candidates (existing)
    6. `narrative = prose_generator.narrate(&NarrateRequest { ..., plan }).await?`
    7. `narrative = align_beats_to_plan(narrative, &plan, &characters)?` (Task 5b helper)
    8. assemble (existing)

**Helper `align_beats_to_plan` (in `service.rs` or `assembly.rs`):**

```rust
/// Ensure output beats follow camera_beats order.
/// Strategy:
/// 1. If narrative.beats.len() == plan.camera_beats.len(), reorder/rename by index:
///    force beat[i].pov = uuid of plan.camera_beats[i].pov_name, set camera_beat_id.
/// 2. If lengths differ: pad/truncate is NOT allowed — return Err(InvalidNarrationContext)
///    OR (mock-friendly): if narrative has 1 beat and plan has N, expand is prose mock's job.
/// Prefer strict equality for production; mock prose (Task 6) always matches length.
fn resolve_pov_name_to_uuid(name: &str, characters: &[Character]) -> Result<CharacterId, StoryError>
```

- [ ] **Step 1: Add `scene_planner` field and update `new`**

- [ ] **Step 2: Implement plan validation + call order in `narrate_scene`**

- [ ] **Step 3: Bootstrap**

```rust
// with Rig:
let scene_planner: Arc<dyn ScenePlanner> = Arc::new(RigScenePlanner::new(client.clone()));
// mock pair:
Arc::new(MockScenePlanner::fallback())
```

Same mock/real coupling rule as sense+prose: **all mock or all real**.

- [ ] **Step 4: Fix all `StoryService::new` call sites**

- [ ] **Step 5: `cargo test --test e2e -- --nocapture` (or existing smoke)**

Expected: PASS after mock updates.

- [ ] **Step 6: Commit**

```bash
git add src/scene/service.rs src/bootstrap.rs src/bin/run_childhood_rivalry.rs tests/
git commit -m "feat(scene): plan scene before narrate with ScenePlanner"
```

---

### Task 6: Prose mock + rig prompt follow camera_beats 1:1

**Files:**
- Modify: `src/prose/mock.rs`
- Modify: `src/prose/rig_impl.rs` (`build_prompt`)
- Modify: `src/prose/planner_rig.rs` (create if not done in Task 5)
- Test: unit test prompt contains `camera_beats` / `beat_id`

**Interfaces:**
- Mock fallback:
  - For each `plan.scene_card.camera_beats[i]`:
    - resolve pov UUID by `pov_name` among `req.characters` (skip beat if missing)
    - `sensation_refs` = first candidate id for that character if any
    - `action` = `Look` or `Enter` with `target` = truncated `intent` (≤6 chars clean)
    - `camera_beat_id` = beat.beat_id
- Rig `build_prompt` append:

```text
【编导镜头表 — 必须严格按序各写 1 个 beat，数量必须相等】
for beat in plan.scene_card.camera_beats:
  - beat_id | pov_name | intent | must_show | location_hint

铁律补充:
5. beats.len() 必须等于镜头表长度，顺序一致。
6. 每个 beat.camera_beat_id 填对应 beat_id。
7. pov 填该镜头角色的 UUID（参与者列表中的 id）。
8. sensation_refs 只能选自该 pov 的候选片段；Hard 模式正文将粘贴原文。
9. action 只写客观动作/对话，禁止感官修饰。

【大纲】
premise + acts summaries
```

- [ ] **Step 1: Test mock emits N beats**

```rust
#[tokio::test]
async fn mock_narrate_emits_one_beat_per_camera_beat() {
    // build NarrateRequest with plan of 3 beats, 2 characters alternating names
    // MockProseGenerator::fallback().narrate(&req)
    // assert beats.len()==3
    // assert beats[i].camera_beat_id == plan.camera_beats[i].beat_id
}
```

- [ ] **Step 2: Implement mock + prompt**

- [ ] **Step 3: Implement `RigScenePlanner`**

```rust
pub struct RigScenePlanner {
    extractor: Extractor<deepseek::CompletionModel, LlmScenePlan>,
}
// prompt: 你是小说编导。根据客观事件与角色，输出大纲+镜头表。
// 镜头 3~8 个；pov_name 必须是角色名之一；intent 短句；must_show 2词内。
// 禁止输出正文。
```

- [ ] **Step 4: Run tests**

Run: `cargo test mock_narrate_emits_one_beat_per_camera_beat -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/prose/mock.rs src/prose/rig_impl.rs src/prose/planner_rig.rs src/prose/mod.rs
git commit -m "feat(prose): align narrate beats to camera plan in mock and prompt"
```

---

### Task 7: Integration test — coherent shot order + Hard quote

**Files:**
- Create: `tests/plan_narrate_test.rs`
- Modify: `AGENTS.md` (Runtime / Architecture short note)

**Interfaces:**
- End-to-end with `Db::open_in_memory`, real `assets/vocab.yaml` (base only: set `NOVELS_SKIP_DISTILLED=1` in test process if needed for speed), `MockSenseGenerator` canned sensations using **base** ids like `visual.bloodstain` if present in vocab, `MockScenePlanner::new(fixed_plan)`, `MockProseGenerator` or fallback.

**Scenario:**

```text
plan.camera_beats:
  b1 苏念卿 intent=看封条 must_show=[封条]
  b2 苏念卿 intent=看父亲
  b3 苏念卿 intent=搀扶母亲
```

Mock prose: three beats, refs only from derivation set; actions Look/Support.

Assert:

1. `assembled.text` has **three lines** (split `\n`) in beat order.
2. Line order matches action targets or camera_beat intents (substring).
3. Every injected quote from vocab still `assembled.text.contains(quote)`.
4. `unverified_quotes == 0`.
5. Optional: `stripped_refs` documented.

- [ ] **Step 1: Write `tests/plan_narrate_test.rs`**

Mirror structure of `tests/e2e.rs` for service construction — **must pass `scene_planner`**.

```rust
#[tokio::test]
async fn narrate_follows_camera_beat_order_hard_quotes() {
    // ... create chars, scene, derive with mock sense that picks known vocab ids
    // fixed MockScenePlanner plan with 3 beats same pov
    // fixed MockProseGenerator with 3 NarrativeBeats matching plan
    let prose = service.narrate_scene(scene_id, &derivations).await.unwrap();
    let lines: Vec<_> = prose.text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 3);
    // order checks on action render fragments
    assert!(lines[0].contains("封条") || lines[0].contains("看"));
    assert_eq!(prose.unverified_quotes, 0);
}
```

- [ ] **Step 2: Run**

Run: `cargo test --test plan_narrate_test -- --nocapture`  
Expected: PASS

- [ ] **Step 3: Document in AGENTS.md**

Under Scene Orchestration or prose:

```markdown
- `narrate_scene` Phase 1: `ScenePlanner` builds outline + camera_beats; prose emits 1 beat per camera beat; assemble joins Hard quotes in ref order (not sense order).
```

- [ ] **Step 4: Commit**

```bash
git add tests/plan_narrate_test.rs AGENTS.md
git commit -m "test(prose): camera-beat order narrate integration under Hard quotes"
```

---

### Task 8: CLI observation surfaces plan summary

**Files:**
- Modify: `src/cli/observation.rs` (if needed: extra optional fields)
- Modify: `src/cli/commands.rs` narrate branch
- Modify: `src/scene/service.rs` **or** return type

**Design choice (pick one, implement exactly):**

**Preferred (minimal API churn):** keep `narrate_scene -> AssembledProse` but store last plan is wrong for one-shot. Better:

```rust
pub struct NarrationResult {
    pub prose: AssembledProse,
    pub plan: LlmScenePlan,
}
```

Change `narrate_scene` to return `NarrationResult`. Update all callers (`cli`, `main`, bins, tests).

CLI human render on success:

```text
status: success
summary: 正文已生成
artifacts:
  outline: <premise_one_liner>
  camera_beats: b1:苏念卿/看封条; b2:...
quality:
  quote_density: ...
```

JSON: include `"outline"` and `"camera_beats"` arrays.

- [ ] **Step 1: Introduce `NarrationResult`, update callers**

- [ ] **Step 2: CLI formats plan**

- [ ] **Step 3: `cargo test --test cli_test plan` filters + `cargo test --test plan_narrate_test`**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/scene/service.rs src/cli/commands.rs src/cli/observation.rs src/main.rs src/bin/ tests/
git commit -m "feat(cli): show outline and camera beats after narrate"
```

---

### Task 9: Verification gate

**Files:** none (commands only) + fix only if gate fails within Phase 1 scope

- [ ] **Step 1: Format**

Run: `cargo fmt --all -- --check`  
Expected: clean

- [ ] **Step 2: Unit + integration tests for this feature**

Run:

```powershell
cargo test assemble_preserves_ref_order_not_sense_order
cargo test fallback_one_beat_per_character
cargo test mock_narrate_emits_one_beat
cargo test --test plan_narrate_test
cargo test --test e2e
cargo test --test cli_test
```

Expected: all PASS

- [ ] **Step 3: Clippy on touched surface**

Run: `cargo clippy --all-targets --all-features -- -D warnings`  
Expected: clean (or document pre-existing baseline only if identical to known Lance issue — if Lance already removed, must be clean)

- [ ] **Step 4: Manual checklist (no code)**

- [ ] Hard: no new code path rewrites `VocabEntry.text` before inject  
- [ ] Sense-order sort removed from description join  
- [ ] Plan required non-empty camera_beats  
- [ ] Mock path works without API key  

- [ ] **Step 5: Final commit if fixes landed**

```bash
git add -u
git commit -m "chore: Phase1 outline-camera verification fixes"
```

(Only if there are fixes; otherwise skip.)

---

## Out of scope (do not implement in this plan)

- Stage 5 Align / Soft modern paraphrase into body  
- Stage 7 Critic loop  
- Changing BM25 / tag shortlist  
- Persisting plans to SQLite  
- Multi-scene arc planner  
- New modern corpus bucket  

---

## Self-review checklist (author)

| Spec item (Phase 1 + Hard) | Task |
|----------------------------|------|
| StoryOutline | T1, T5, T6 |
| SceneCard + camera_beats | T1–T2, T5–T6 |
| Assemble camera/ref order not sense order | T3 |
| Hard verbatim quotes | T3, T7 (no Align) |
| Wire narrate path | T5–T6 |
| CLI visibility | T8 |
| Tests + gate | T7, T9 |

No Soft ground. No placeholder tasks. Types named consistently: `LlmScenePlan`, `ScenePlanner`, `PlanRequest`, `NarrationResult`.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-23-phase1-outline-camera-assemble.md`.

**Two execution options:**

1. **Subagent-Driven（推荐）** — 每任务新 subagent，任务间 review  
2. **Inline Execution** — 本会话按 executing-plans 连续做完  

**Which approach?**
