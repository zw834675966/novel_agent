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
