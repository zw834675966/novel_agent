### Task 5: Wire planner into StoryService::narrate_scene + bootstrap

**Files:**
- Create: `src/prose/planner_rig.rs` (RigScenePlanner)
- Modify: `src/prose/mod.rs` (export planner_rig)
- Modify: `src/scene/service.rs` (add scene_planner field, update new(), replace placeholder, add validation)
- Modify: `src/bootstrap.rs` (construct mock/real planner pair)
- Modify: `src/bin/run_childhood_rivalry.rs` (pass planner to StoryService::new)
- Modify: ALL 14 test call sites of `StoryService::new()` (add scene_planner argument)
- Test: existing tests still pass after wiring

**Base commit:** `d0db294`

**Goal:** Replace the `LlmScenePlan::minimal()` placeholder in `narrate_scene` with a real `ScenePlanner::plan_scene()` call. Add the planner to `StoryService`, bootstrap, and all call sites.

---

## Important: Pre-existing uncommitted changes

The working tree has pre-existing uncommitted changes in several files (service.rs, scene_test.rs, e2e.rs, models/*, etc.) from a separate N5/MemoryContentSlot refactor. These changes are needed for compilation. When you modify files that have pre-existing changes, your changes will be mixed with them. **This is expected** - make all changes needed for Task 5. The commit will be handled selectively after you complete.

**Do NOT revert or remove any pre-existing changes.** Only ADD your Task 5 changes on top.

---

## Changes

### 1. Create `src/prose/planner_rig.rs`

Mirror the `RigProseGenerator` / `RigSenseGenerator` pattern. Use a rig extractor with `LlmScenePlan` as the target type.

```rust
use std::sync::Arc;
use rig::client::ProviderClient;
use rig::providers::deepseek;
use rig::completion::CompletionModel;

use crate::models::StoryError;
use super::plan_contract::LlmScenePlan;
use super::planner::{PlanRequest, ScenePlanner};

pub struct RigScenePlanner {
    extractor: rig::extractor::Extractor<deepseek::CompletionModel, LlmScenePlan>,
}

impl RigScenePlanner {
    pub fn new(client: Arc<deepseek::Client>) -> Self {
        let model = client.completion_model(&deepseek::DEEPSEEK_V4_FLASH);
        Self {
            extractor: rig::extractor::Extractor::new(model),
        }
    }
}

#[async_trait::async_trait]
impl ScenePlanner for RigScenePlanner {
    async fn plan_scene(&self, req: &PlanRequest) -> Result<LlmScenePlan, StoryError> {
        let prompt = build_planner_prompt(req);
        self.extractor
            .extract(prompt)
            .await
            .map_err(|e| StoryError::Llm(e.to_string()))
    }
}

fn build_planner_prompt(req: &PlanRequest) -> String {
    // Basic prompt - Task 6 may enhance
    let names: Vec<&str> = req.characters.iter().map(|c| c.name.as_str()).collect();
    format!(
        "你是小说编导。根据客观事件与角色，输出大纲+镜头表。\n\
         客观事件：{}\n\
         角色：{}\n\
         要求：镜头3~8个；pov_name必须是角色名之一；intent短句；禁止输出正文。",
        req.scene.objective_event,
        names.join("、")
    )
}
```

**IMPORTANT:** Check how `RigProseGenerator` and `RigSenseGenerator` construct their extractors in `src/prose/rig_impl.rs` and `src/llm/rig_impl.rs`. Match that pattern exactly (same model constant, same extractor construction, same retry config if any). The above is a template - verify against the actual code.

### 2. Export in `src/prose/mod.rs`

```rust
mod planner_rig;
pub use planner_rig::RigScenePlanner;
```

### 3. Update `StoryService` in `src/scene/service.rs`

Add field:
```rust
pub struct StoryService {
    db: Db,
    vocab: Vocab,
    sense_generator: Arc<dyn SenseGenerator>,
    prose_generator: Arc<dyn ProseGenerator>,
    scene_planner: Arc<dyn ScenePlanner>,  // NEW
}
```

Update `new()`:
```rust
pub fn new(
    db: Db,
    vocab: Vocab,
    sense_generator: Arc<dyn SenseGenerator>,
    prose_generator: Arc<dyn ProseGenerator>,
    scene_planner: Arc<dyn ScenePlanner>,  // NEW
) -> Self {
    Self { db, vocab, sense_generator, prose_generator, scene_planner }
}
```

Add import: `use crate::prose::{..., ScenePlanner, PlanRequest};` (check existing imports).

### 4. Replace placeholder in `narrate_scene`

Find the Task 4 placeholder:
```rust
//    plan 为占位：Task 5 替换为真实 ScenePlanner::plan_scene() 输出。
let plan = LlmScenePlan::minimal(
    &scene.objective_event,
    &characters.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
);
```

Replace with:
```rust
// 5.5. 调用 ScenePlanner 生成编导大纲+镜头表
let plan = self
    .scene_planner
    .plan_scene(&PlanRequest {
        scene: scene.clone(),
        characters: characters.clone(),
    })
    .await?;

// 验证计划：camera_beats 非空；每个 pov_name 匹配某个角色名
if plan.scene_card.camera_beats.is_empty() {
    return Err(StoryError::Llm("scene planner returned no camera beats".into()));
}
let valid_names: HashSet<&str> = characters.iter().map(|c| c.name.as_str()).collect();
for beat in &plan.scene_card.camera_beats {
    if !valid_names.contains(beat.pov_name.as_str()) {
        return Err(StoryError::Llm(format!(
            "camera beat pov_name '{}' not found among scene participants",
            beat.pov_name
        )));
    }
}
```

Note: `characters` may need `.clone()` if it's moved later into `NarrateRequest`. Check the borrow flow.

### 5. Update `src/bootstrap.rs`

Add `MockScenePlanner` and `RigScenePlanner` to imports.

Update the generator construction to also build a planner:

```rust
let (sense_generator, prose_generator, scene_planner, using_mock): (
    Arc<dyn SenseGenerator>,
    Arc<dyn ProseGenerator>,
    Arc<dyn ScenePlanner>,
    bool,
) = match deepseek::Client::from_env() {
    Ok(client) => (
        Arc::new(RigSenseGenerator::new(client.clone(), vocab.clone())),
        Arc::new(RigProseGenerator::new(client.clone())),
        Arc::new(RigScenePlanner::new(client)),
        false,
    ),
    Err(_) => (
        Arc::new(MockSenseGenerator::new(...)),
        Arc::new(MockProseGenerator::fallback()),
        Arc::new(MockScenePlanner::fallback()),
        true,
    ),
};

let service = StoryService::new(db, vocab, sense_generator, prose_generator, scene_planner);
```

**IMPORTANT:** Check how `RigProseGenerator::new` takes the client - does it take `client` (consuming) or `client.clone()`? Match the pattern. The planner needs its own client reference.

### 6. Update `src/bin/run_childhood_rivalry.rs`

Add `MockScenePlanner::fallback()` (or appropriate planner) to the `StoryService::new()` call.

### 7. Update ALL 14 test call sites

Every `StoryService::new(db, vocab, sense, prose)` call needs a 5th argument: `Arc::new(MockScenePlanner::fallback())` (or `MockScenePlanner::new(plan)` for tests that need a specific plan).

**Call sites (from grep):**

Production:
- `src/bootstrap.rs:138`
- `src/bin/run_childhood_rivalry.rs:88`

Tests:
- `tests/prose_test.rs:73, 309`
- `tests/scene_test.rs:159, 195, 224, 288, 336, 383, 464`
- `tests/cli_test.rs:41`
- `tests/bm25_provenance_real_test.rs:167, 257`
- `tests/api_test.rs:33`
- `tests/e2e.rs:39`
- `tests/state_machine_test.rs:62`
- `tests/story_test_childhood_rivalry.rs:114`

For most tests, add `Arc::new(MockScenePlanner::fallback())` as the 5th argument. Import `use novels::prose::{MockScenePlanner, ScenePlanner};` (or `use novels::prose::MockScenePlanner;`) as needed.

For tests that import from `novels::` (check existing imports), the `MockScenePlanner` is already exported from `src/prose/mod.rs`.

### 8. Verify compilation and tests

```powershell
cargo check --all-targets
cargo test --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

All must pass. Existing tests should be unaffected (MockScenePlanner::fallback() produces a valid plan).

### 9. Commit

```bash
git add <all modified files>
git commit -m "feat(scene): plan scene before narrate with ScenePlanner"
```

---

## What NOT to change

- Do NOT change `MockProseGenerator::narrate` behavior (Task 6)
- Do NOT change `RigProseGenerator::build_prompt` (Task 6)
- Do NOT change the assemble logic (Task 3 already done)
- Do NOT remove `LlmScenePlan::minimal()` - tests may still use it
- Do NOT revert any pre-existing uncommitted changes in dirty files
- Do NOT modify the BM25/tag/candidate retrieval logic

## Key pattern to follow

Look at `src/llm/rig_impl.rs` (RigSenseGenerator) and `src/prose/rig_impl.rs` (RigProseGenerator) for the exact rig extractor pattern. Match it for RigScenePlanner. The key details:
- How the client is consumed/cloned
- How the completion model is created
- How the extractor is constructed
- How extraction errors are mapped to StoryError
- Whether retries are configured
