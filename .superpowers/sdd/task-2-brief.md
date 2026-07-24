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

**Character type (actual fields from `src/models/character.rs`):**
```rust
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub personality: Vec<String>,
    pub skills: Vec<String>,
}
```
Note: Character does NOT have `created_at`/`updated_at` fields. Only `id`, `name`, `personality`, `skills`.

**Scene type (verify from `src/models/scene.rs`):**
```rust
pub struct Scene {
    pub id: SceneId,
    pub objective_event: String,
    pub occurred_at: DateTime<Utc>,
    pub participant_ids: Vec<CharacterId>,
}
```

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
        // sorted by name: 顾 before 苏 (Unicode/lexicographic - assert stable sort by name)
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
// planner_mock.rs - fallback builds:
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
