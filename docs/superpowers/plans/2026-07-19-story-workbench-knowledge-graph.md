# Story Workbench Knowledge Graph Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Chinese-first local authoring workbench that derives character perceptions, captures author-confirmed temporal relationship changes, and visualizes the resulting story graph.

**Architecture:** Keep SQLite as the only persistent store and project graph nodes from existing character, scene, memory, and derivation records. Add typed relationship facts, revisions, and pending LLM candidates; preserve all candidate evidence until the author accepts or rejects it. Add an Axum JSON boundary over `StoryService`, then a Vite/React client with Cytoscape.js for the focused graph route; compile frontend assets into the Rust binary.

**Tech Stack:** Rust 2024, Axum 0.8, SQLx SQLite, Tokio, Serde, RustEmbed, React, TypeScript, Vite, Cytoscape.js, Playwright.

## Global Constraints

- Preserve current SQLite-first architecture; do not add Neo4j, FalkorDB, Graphiti, GraphRAG, LightRAG, vector storage, or embedding dependencies.
- Existing character, scene, memory, sensation, and plot-development records remain canonical; no generic graph-node table.
- Only author acceptance can create or revise a relationship fact. LLM output remains a pending candidate.
- Persist evidence, scene provenance, valid-from scene, and superseded history for every confirmed revision.
- Use Chinese author-facing copy. Never render UUIDs, database details, vocabulary IDs, secrets, or raw internal errors in the normal UI.
- Bind local server to `127.0.0.1`; serve frontend and API from same origin; do not add permissive CORS.
- Keep existing distilled-vocabulary worktree changes untouched.
- Add Rust tests before corresponding implementation, run `cargo fmt --all`, `cargo test --all-targets`, and `cargo clippy --all-targets --all-features -- -D warnings` for each completed Rust slice.
- Do not commit generated `web/dist`, `web/node_modules`, `.env`, database files, token files, or browser artifacts.

---

## File Structure

| Path | Responsibility |
| --- | --- |
| `src/models/relationship.rs` | Typed IDs, relationship enums, facts, revisions, candidates, graph projection DTOs. |
| `src/db/relationship_repo.rs` | Relationship fact/candidate persistence, atomic candidate resolution, temporal queries. |
| `src/db/schema.rs` | SQLite DDL for relationship tables and indexes. |
| `src/db/mod.rs` | Exposes `RelationshipRepo` through `Db::relationships()`. |
| `src/llm/contract.rs` | Structured relationship-candidate portion of LLM output. |
| `src/llm/generator.rs` | Supplies named scene participants to LLM generation. |
| `src/llm/rig_impl.rs` | Prompts the model to propose only allowlisted relationship candidates. |
| `src/models/derivation.rs` | Returns persisted memory and relationship candidates from service derivation. |
| `src/db/derivation_repo.rs` | Returns created memory identity from the existing sensation-plus-memory transaction. |
| `src/scene/service.rs` | Validates and persists candidates after derivation, lists author-facing data, resolves candidates. |
| `src/api/mod.rs` | Axum router, application state, error mapping, static-asset fallback. |
| `src/api/dto.rs` | JSON request/response types and boundary validation. |
| `src/api/handlers.rs` | Thin handlers that call `StoryService`; no SQL. |
| `src/main.rs` | Creates service and starts the local Axum listener instead of demo data. |
| `tests/relationship_test.rs` | Repository and service-level temporal-relationship tests. |
| `tests/api_test.rs` | Router-level HTTP status, body, and partial-success tests. |
| `web/` | React/Vite application, type-safe API client, workbench, graph route, and frontend tests. |
| `web/e2e/workbench.spec.ts` | Playwright desktop and mobile primary-flow coverage. |

## Task 1: Establish Relationship Domain Types

**Files:**
- Create: `src/models/relationship.rs`
- Modify: `src/models/ids.rs`
- Modify: `src/models/mod.rs`
- Modify: `src/models/error.rs`
- Test: `tests/relationship_test.rs`

**Interfaces:**
- Consumes: `CharacterId`, `SceneId`, `MemoryId`, `StoryError` from `crate::models`.
- Produces: `RelationshipFactId`, `RelationshipCandidateId`, `RelationshipType`, `CandidateStatus`, `RelationshipFact`, `RelationshipRevision`, `RelationshipCandidate`, and `GraphSnapshot` for database, service, and API tasks.

- [ ] **Step 1: Write failing relationship-type and candidate-state tests**

```rust
use novels::models::{CandidateStatus, RelationshipType};

#[test]
fn relationship_type_uses_stable_snake_case_json_values() {
    assert_eq!(serde_json::to_string(&RelationshipType::Distrusts).unwrap(), "\"distrusts\"");
    assert_eq!(serde_json::from_str::<RelationshipType>("\"family_of\"").unwrap(), RelationshipType::FamilyOf);
}

#[test]
fn pending_candidate_cannot_be_resolved_twice() {
    assert!(CandidateStatus::Pending.can_transition_to(CandidateStatus::Accepted));
    assert!(!CandidateStatus::Accepted.can_transition_to(CandidateStatus::Rejected));
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test relationship_type_uses_stable_snake_case_json_values`

Expected: FAIL because `novels::models::RelationshipType` does not exist.

- [ ] **Step 3: Add minimal typed relationship model**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Knows,
    AlliedWith,
    Distrusts,
    Owes,
    FamilyOf,
    Mentors,
    ConflictsWith,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus { Pending, Accepted, Rejected }

impl CandidateStatus {
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!((self, next), (Self::Pending, Self::Accepted | Self::Rejected))
    }
}
```

Add UUID newtypes for fact and candidate IDs using the same derives as `CharacterId`. Define revision score fields as `Option<u8>` and add `validate_score(score: Option<u8>) -> Result<(), StoryError>` that rejects values above `100`. Add `StoryError::RelationshipCandidateNotFound`, `StoryError::RelationshipCandidateResolved`, and `StoryError::InvalidRelationshipCandidate(String)`. Re-export all public relationship types from `src/models/mod.rs`.

- [ ] **Step 4: Run domain tests and formatting**

Run: `cargo fmt --all && cargo test relationship_ --test relationship_test`

Expected: PASS with both domain tests green.

- [ ] **Step 5: Commit the focused domain model**

```bash
git add src/models/relationship.rs src/models/ids.rs src/models/mod.rs src/models/error.rs tests/relationship_test.rs
git commit -m "feat: add temporal relationship domain types"
```

## Task 2: Persist Facts, Revisions, and Candidates Atomically

**Files:**
- Create: `src/db/relationship_repo.rs`
- Modify: `src/db/schema.rs`
- Modify: `src/db/mod.rs`
- Modify: `tests/relationship_test.rs`

**Interfaces:**
- Consumes: all relationship types from Task 1 and `Db::pool()` transaction pattern.
- Produces: `Db::relationships() -> RelationshipRepo`, `insert_candidates`, `resolve_candidate`, `list_candidates_for_scene`, `graph_snapshot_through`, and `history`.

- [ ] **Step 1: Write failing repository tests for temporal replacement and rejection**

```rust
#[tokio::test]
async fn accepting_new_revision_supersedes_previous_revision() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, first_scene, second_scene) = seed_two_characters_and_two_scenes(&db).await;
    let first = db.relationships().insert_pending(candidate(a, b, first_scene, RelationshipType::AlliedWith)).await.unwrap();
    db.relationships().resolve_candidate(first.id, CandidateResolution::accept("共同目标使他们结盟", None)).await.unwrap();
    let second = db.relationships().insert_pending(candidate(a, b, second_scene, RelationshipType::AlliedWith)).await.unwrap();
    db.relationships().resolve_candidate(second.id, CandidateResolution::accept("证据引发不信任", None)).await.unwrap();

    let graph = db.relationships().graph_snapshot_through(second_scene).await.unwrap();
    assert_eq!(graph.edges.iter().filter(|edge| edge.active).count(), 1);
    assert_eq!(db.relationships().history(graph.edges[0].fact_id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn rejecting_candidate_creates_no_fact_or_revision() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, scene, _) = seed_two_characters_and_two_scenes(&db).await;
    let candidate = db.relationships().insert_pending(candidate(a, b, scene, RelationshipType::Distrusts)).await.unwrap();
    db.relationships().resolve_candidate(candidate.id, CandidateResolution::reject()).await.unwrap();
    assert!(db.relationships().graph_snapshot_through(scene).await.unwrap().edges.is_empty());
}
```

- [ ] **Step 2: Run targeted tests to verify they fail**

Run: `cargo test --test relationship_test accepting_new_revision_supersedes_previous_revision`

Expected: FAIL because `Db::relationships` and `RelationshipRepo` do not exist.

- [ ] **Step 3: Add schema and repository implementation**

Add these tables and indexes to `SCHEMA_SQL`; all UUIDs and timestamps remain `TEXT`, matching current schema conventions.

```sql
CREATE TABLE IF NOT EXISTS relationship_facts (
    id TEXT PRIMARY KEY,
    from_character_id TEXT NOT NULL,
    to_character_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL CHECK (created_by = 'author'),
    UNIQUE(from_character_id, to_character_id, relationship_type),
    FOREIGN KEY (from_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (to_character_id) REFERENCES characters(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS relationship_revisions (
    id TEXT PRIMARY KEY,
    relationship_fact_id TEXT NOT NULL,
    scene_id TEXT NOT NULL,
    valid_from_scene_id TEXT NOT NULL,
    valid_until_scene_id TEXT,
    status TEXT NOT NULL CHECK (status IN ('active', 'superseded')),
    summary TEXT NOT NULL,
    tension_score INTEGER,
    trust_score INTEGER,
    affection_score INTEGER,
    power_score INTEGER,
    evidence_memory_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (relationship_fact_id) REFERENCES relationship_facts(id) ON DELETE CASCADE,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE,
    FOREIGN KEY (evidence_memory_id) REFERENCES character_memories(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_relationship_revisions_fact ON relationship_revisions(relationship_fact_id, created_at DESC);
CREATE TABLE IF NOT EXISTS relationship_candidates (
    id TEXT PRIMARY KEY,
    scene_id TEXT NOT NULL,
    from_character_id TEXT NOT NULL,
    to_character_id TEXT NOT NULL,
    relationship_type TEXT NOT NULL,
    summary TEXT NOT NULL,
    tension_score INTEGER,
    trust_score INTEGER,
    affection_score INTEGER,
    power_score INTEGER,
    evidence_memory_id TEXT,
    confidence REAL NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'accepted', 'rejected')),
    created_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (scene_id) REFERENCES scenes(id) ON DELETE CASCADE,
    FOREIGN KEY (from_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (to_character_id) REFERENCES characters(id) ON DELETE CASCADE,
    FOREIGN KEY (evidence_memory_id) REFERENCES character_memories(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_relationship_candidates_scene ON relationship_candidates(scene_id, status);
```

Implement `resolve_candidate` in one SQLx transaction: fetch pending candidate; reject by updating only its status and timestamp; accept by `INSERT ... ON CONFLICT` the fact, update its existing active revision to `superseded` with `valid_until_scene_id = candidate.scene_id`, insert the new active revision, then update the candidate to `accepted`. Reject a non-pending row with `StoryError::RelationshipCandidateResolved`.

`graph_snapshot_through(scene_id)` must determine the selected scene's `occurred_at`, include every revision whose `valid_from_scene` occurs at or before it and whose `valid_until_scene` is null or later, and project character names as graph nodes. Do not use row insertion order as story time.

- [ ] **Step 4: Run relationship integration tests**

Run: `cargo test --test relationship_test && cargo test --test db_test`

Expected: PASS; relationship acceptance is atomic and rejection creates no fact.

- [ ] **Step 5: Commit repository and schema slice**

```bash
git add src/db/relationship_repo.rs src/db/schema.rs src/db/mod.rs tests/relationship_test.rs
git commit -m "feat: persist temporal relationship revisions"
```

## Task 3: Produce and Validate LLM Relationship Candidates

**Files:**
- Modify: `src/llm/contract.rs`
- Modify: `src/llm/generator.rs`
- Modify: `src/llm/rig_impl.rs`
- Modify: `src/llm/mock.rs`
- Modify: `src/models/derivation.rs`
- Modify: `src/db/derivation_repo.rs`
- Modify: `src/scene/service.rs`
- Modify: `tests/scene_test.rs`
- Modify: `tests/relationship_test.rs`

**Interfaces:**
- Consumes: Task 1 types and Task 2 `RelationshipRepo::insert_candidates`.
- Produces: `LlmRelationshipCandidate`, `CharacterDerivation { new_memory: CharacterMemory, relationship_candidates: Vec<RelationshipCandidate> }`, and `StoryService::resolve_relationship_candidate`.

- [ ] **Step 1: Write failing tests for valid and invalid candidates**

```rust
#[tokio::test]
async fn derive_character_persists_valid_candidate_with_created_memory_evidence() {
    let result = service_with_two_participants_and_candidate().derive_character(scene_id, character_a).await.unwrap();
    assert_eq!(result.relationship_candidates.len(), 1);
    assert_eq!(result.relationship_candidates[0].evidence_memory_id, Some(result.new_memory.id));
    assert_eq!(result.relationship_candidates[0].status, CandidateStatus::Pending);
}

#[tokio::test]
async fn derive_character_discards_candidate_that_targets_non_participant() {
    let result = service_with_invalid_target_candidate().derive_character(scene_id, character_a).await.unwrap();
    assert!(result.relationship_candidates.is_empty());
    assert_eq!(db.memories().list(character_a, 50).await.unwrap().len(), 1);
}
```

- [ ] **Step 2: Run targeted tests to verify they fail**

Run: `cargo test --test scene_test derive_character_persists_valid_candidate_with_created_memory_evidence`

Expected: FAIL because the LLM contract and `CharacterDerivation` lack relationship candidates and persisted memory ID.

- [ ] **Step 3: Extend structured contract and service path**

Use this explicit LLM shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmRelationshipCandidate {
    pub target_character_id: CharacterId,
    pub relationship_type: RelationshipType,
    pub summary: String,
    pub tension_score: Option<u8>,
    pub trust_score: Option<u8>,
    pub affection_score: Option<u8>,
    pub power_score: Option<u8>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LlmCharacterDerivation {
    pub sensations: SensorySelection,
    pub new_memory: CharacterMemoryDraft,
    pub plot_development: Vec<PlotDevelopment>,
    #[serde(default)]
    pub relationship_candidates: Vec<LlmRelationshipCandidate>,
}
```

Extend `DerivationRequest` with `scene_participants: Vec<Character>`. Before generator invocation, load participant characters and make the prompt include only their names and UUIDs. Prompt rules: candidates must target another listed participant, must use one listed relationship type, must have Chinese summary under 200 characters, scores from 0 through 100, and confidence from `0.0` through `1.0`.

Change `DerivationRepo::insert_derivation` to return a `CharacterMemory` with the generated `MemoryId` from its transaction. Update `CharacterDerivation` to hold this persisted memory. Validate candidates after vocabulary validation: target differs from source and is a scene participant, summary is nonblank and at most 200 Unicode scalar values, scores are valid, confidence is finite and inside inclusive bounds. Ignore an invalid candidate only; do not discard a valid derivation. Persist valid candidates with `evidence_memory_id: Some(new_memory.id)`.

Add `StoryService::resolve_relationship_candidate(candidate_id, resolution)` delegating to the relationship repository.

- [ ] **Step 4: Run service and relationship test suites**

Run: `cargo test --test scene_test && cargo test --test relationship_test`

Expected: PASS; mocks compile with `relationship_candidates: vec![]`, and valid candidates retain the generated memory as evidence.

- [ ] **Step 5: Commit LLM candidate slice**

```bash
git add src/llm src/models/derivation.rs src/db/derivation_repo.rs src/scene/service.rs tests/scene_test.rs tests/relationship_test.rs
git commit -m "feat: derive pending relationship candidates"
```

## Task 4: Add Read APIs Needed by Workbench

**Files:**
- Modify: `src/db/character_repo.rs`
- Modify: `src/db/scene_repo.rs`
- Modify: `src/db/memory_repo.rs`
- Modify: `src/db/sensation_repo.rs`
- Modify: `src/scene/service.rs`
- Modify: `tests/db_test.rs`
- Modify: `tests/scene_test.rs`

**Interfaces:**
- Consumes: existing repositories and `RelationshipRepo` from Task 2.
- Produces: `list` and scene-scoped read methods used only through `StoryService` and later API handlers.

- [ ] **Step 1: Write failing ordered-list and scene-read tests**

```rust
#[tokio::test]
async fn list_scenes_orders_by_occurrence_then_id() {
    let scenes = db.scenes().list().await.unwrap();
    assert_eq!(scenes.iter().map(|scene| &scene.objective_event).collect::<Vec<_>>(), vec!["earlier", "later"]);
}

#[tokio::test]
async fn scene_derivations_include_only_that_scene_memory_and_sensation() {
    let details = service.scene_derivations(scene_id).await.unwrap();
    assert!(details.iter().all(|detail| detail.memory.scene_id == scene_id));
}
```

- [ ] **Step 2: Run targeted tests to verify they fail**

Run: `cargo test --test db_test list_scenes_orders_by_occurrence_then_id`

Expected: FAIL because list and scene-scoped repository methods do not exist.

- [ ] **Step 3: Add bounded read methods**

Add `CharacterRepo::list() -> Result<Vec<Character>, StoryError>` ordered by `name, id`; `SceneRepo::list() -> Result<Vec<Scene>, StoryError>` ordered by `occurred_at, id`; `MemoryRepo::list_for_scene(scene_id)`; and `SensationRepo::list_for_scene(scene_id)`. Parse all stored IDs and enum JSON with existing `StoryError::Database` behavior.

Define an explicit `SceneDerivationDetail` in `src/models/derivation.rs` containing character, persisted memory, sensation, latest matching plot developments when persisted, and pending candidates. Because plot developments are currently not persisted, add a `character_plot_developments` table and repository in this task rather than returning ephemeral derivation output. Persist plot developments alongside memory and sensation in the existing derivation transaction.

Expose `StoryService::list_characters`, `list_scenes`, `get_scene`, `scene_derivations`, `story_graph_through`, and `relationship_history` as the application-level reads consumed by HTTP handlers.

- [ ] **Step 4: Run current and new database/service tests**

Run: `cargo test --test db_test && cargo test --test scene_test`

Expected: PASS; all workbench reads are deterministic and scoped to the selected scene.

- [ ] **Step 5: Commit application read-model slice**

```bash
git add src/db src/models/derivation.rs src/scene/service.rs tests/db_test.rs tests/scene_test.rs
git commit -m "feat: expose workbench story reads"
```

## Task 5: Add Axum API Boundary and Embedded Static Assets

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/dto.rs`
- Create: `src/api/handlers.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `Cargo.toml`
- Create: `tests/api_test.rs`

**Interfaces:**
- Consumes: Task 3 mutation methods and Task 4 service reads.
- Produces: `api::app(StoryService) -> Router`, same-origin `/api/*` routes, structured `ApiError`, and binary-static frontend fallback.

- [ ] **Step 1: Write failing router tests before handlers**

```rust
#[tokio::test]
async fn create_scene_rejects_blank_objective_event() {
    let response = app(test_service()).oneshot(
        Request::post("/api/scenes")
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"objectiveEvent":"   ","participantIds":[],"occurredAt":"2026-07-19T00:00:00Z"}"#))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn derive_scene_returns_success_and_failure_per_participant() {
    let response = app(service_with_one_failing_generator()).oneshot(
        Request::post(format!("/api/scenes/{scene_id}/derive")).body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeriveSceneResponse = decode_json(response).await;
    assert_eq!(payload.results.iter().filter(|result| result.ok).count(), 1);
    assert_eq!(payload.results.iter().filter(|result| !result.ok).count(), 1);
}
```

- [ ] **Step 2: Run API test to verify it fails**

Run: `cargo test --test api_test create_scene_rejects_blank_objective_event`

Expected: FAIL because module `novels::api` and function `app` do not exist.

- [ ] **Step 3: Add dependencies and routes**

Add `axum = "0.8"`, `tower = "0.5"`, `tower-http = { version = "0.6", features = ["set-header"] }`, and `http-body-util = "0.1"` as needed. Extend Tokio features with `net`.

Create these DTO contracts with `serde(rename_all = "camelCase")`:

```rust
pub struct CreateCharacterRequest { pub name: String, pub personality: Vec<String>, pub skills: Vec<String> }
pub struct CreateSceneRequest { pub objective_event: String, pub participant_ids: Vec<CharacterId>, pub occurred_at: DateTime<Utc> }
pub struct ResolveCandidateRequest { pub action: CandidateAction, pub summary: Option<String>, pub scores: Option<RelationshipScores> }
pub enum CandidateAction { Accept, Reject }
pub struct ApiError { pub code: &'static str, pub message: String, pub fields: Option<BTreeMap<String, String>> }
```

Validate trim-normalized character name and scene objective are 1 through 2,000 characters; each tag is 1 through 80 characters; require at least one unique scene participant. Map `SceneNotFound` and `CharacterNotFound` to `404`, candidate already resolved to `409`, request/domain validation to `422`, and database/LLM failures to a generic Chinese `500` message.

Implement exactly these routes:

```text
GET/POST /api/characters
GET/POST /api/scenes
GET      /api/scenes/{scene_id}
POST     /api/scenes/{scene_id}/derive
GET      /api/scenes/{scene_id}/derivations
GET      /api/story-graph?through_scene_id={scene_id}
PATCH    /api/relationship-candidates/{candidate_id}
GET      /api/relationships/{fact_id}/history
```

Use `AppState { service: StoryService }`, handler extractors, and `Json`. Configure a 1 MiB body limit. Add `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, and a CSP allowing only same-origin scripts, styles, images, and connections. This task serves only API routes; Task 9 adds static-asset embedding after Vite first produces `web/dist`.

Replace the demo in `main.rs` with dotenv loading, database/vocabulary/generator setup, `api::app(service)`, and `TcpListener::bind("127.0.0.1:3000")`.

- [ ] **Step 4: Run API, formatting, lint, and existing tests**

Run: `cargo fmt --all && cargo test --all-targets && cargo clippy --all-targets --all-features -- -D warnings`

Expected: PASS; API tests directly call `Router::oneshot` and no listener is required.

- [ ] **Step 5: Commit API boundary slice**

```bash
git add Cargo.toml Cargo.lock src/api src/lib.rs src/main.rs tests/api_test.rs
git commit -m "feat: expose story workbench API"
```

## Task 6: Scaffold Typed React Client and Scene Workbench

**Files:**
- Create: `web/package.json`
- Create: `web/vite.config.ts`
- Create: `web/tsconfig.json`
- Create: `web/src/main.tsx`
- Create: `web/src/app.tsx`
- Create: `web/src/api/client.ts`
- Create: `web/src/api/types.ts`
- Create: `web/src/features/scenes/SceneEditor.tsx`
- Create: `web/src/features/characters/CharacterSidebar.tsx`
- Create: `web/src/features/derivations/DerivationInspector.tsx`
- Create: `web/src/styles.css`
- Create: `web/src/app.test.tsx`

**Interfaces:**
- Consumes: Task 5 JSON route contracts.
- Produces: browser workbench that lists characters/scenes, creates scenes, derives participants, and renders loading, empty, success, partial-failure, and retry states.

- [ ] **Step 1: Scaffold Vite React TypeScript and install focused dependencies**

Run:

```bash
npm create vite@latest web -- --template react-ts
npm --prefix web install
npm --prefix web install react-router-dom cytoscape
npm --prefix web install --save-dev vitest @testing-library/react @testing-library/user-event jsdom playwright
```

Configure `vite.config.ts` with `/api` proxy target `http://127.0.0.1:3000` for development and `build.outDir: "dist"`.

- [ ] **Step 2: Write failing scene creation test**

```tsx
it("creates a scene from selected participants", async () => {
  server.use(http.get("/api/characters", () => HttpResponse.json([characterA])));
  render(<App />);
  await userEvent.click(await screen.findByRole("checkbox", { name: "沈砚" }));
  await userEvent.type(screen.getByLabelText("客观事件"), "书房发现被撕毁的信");
  await userEvent.click(screen.getByRole("button", { name: "创建场景" }));
  expect(await screen.findByText("场景已创建")).toBeVisible();
});
```

- [ ] **Step 3: Run frontend test to verify it fails**

Run: `npm --prefix web test -- --run src/app.test.tsx`

Expected: FAIL because `App`, API client, and workbench controls do not exist.

- [ ] **Step 4: Implement the minimum usable workbench**

Implement one typed `request<T>(path, init)` client that parses the `ApiError` body on non-2xx responses. Model the scene screen state explicitly:

```ts
type DeriveState =
  | { kind: "idle" }
  | { kind: "running" }
  | { kind: "complete"; result: DeriveSceneResponse }
  | { kind: "failed"; message: string };
```

Use a semantic form and native checkbox list for participant selection. Render a left character sidebar, central scene editor, and right inspector. Render `DerivationInspector` only after a selected scene exists; show each participant result independently so errors do not hide successful cards. Use Chinese labels: `人物`, `客观事件`, `参与人物`, `创建场景`, `开始推导`, `重试`, `暂无推导结果`.

Use responsive CSS grid: three columns at desktop, form-first single column at narrow widths. Avoid cards inside cards, decorative gradients, viewport-scaled text, and exposed IDs.

- [ ] **Step 5: Run frontend checks and production build**

Run: `npm --prefix web test -- --run && npm --prefix web run build`

Expected: PASS; `web/dist/index.html` exists for RustEmbed compilation.

- [ ] **Step 6: Commit workbench slice**

```bash
git add web/package.json web/package-lock.json web/vite.config.ts web/tsconfig.json web/src
git commit -m "feat: add scene derivation workbench"
```

## Task 7: Add Candidate Review and Temporal Timeline

**Files:**
- Create: `web/src/features/relationships/CandidateReview.tsx`
- Create: `web/src/features/timeline/SceneTimeline.tsx`
- Modify: `web/src/api/types.ts`
- Modify: `web/src/api/client.ts`
- Modify: `web/src/app.tsx`
- Modify: `web/src/features/derivations/DerivationInspector.tsx`
- Create: `web/src/features/relationships/CandidateReview.test.tsx`

**Interfaces:**
- Consumes: candidate response and `PATCH /api/relationship-candidates/{candidate_id}` from Task 5.
- Produces: review controls that update only after server confirmation and a selected `throughSceneId` shared with the graph route.

- [ ] **Step 1: Write failing candidate-resolution UI tests**

```tsx
it("accepts an edited candidate only after API success", async () => {
  render(<CandidateReview candidate={pendingCandidate} onResolved={onResolved} />);
  await userEvent.click(screen.getByRole("button", { name: "编辑后接受" }));
  await userEvent.clear(screen.getByLabelText("关系说明"));
  await userEvent.type(screen.getByLabelText("关系说明"), "两人因线索互相试探");
  await userEvent.click(screen.getByRole("button", { name: "确认接受" }));
  expect(await screen.findByText("已确认关系变更")).toBeVisible();
  expect(onResolved).toHaveBeenCalledWith(expect.objectContaining({ status: "accepted" }));
});

it("keeps candidate pending when resolution API fails", async () => {
  server.use(http.patch(/relationship-candidates/, () => HttpResponse.json({ code: "CONFLICT", message: "候选已处理" }, { status: 409 })));
  render(<CandidateReview candidate={pendingCandidate} onResolved={vi.fn()} />);
  await userEvent.click(screen.getByRole("button", { name: "接受" }));
  expect(await screen.findByText("候选已处理")).toBeVisible();
  expect(screen.getByText("待确认")).toBeVisible();
});
```

- [ ] **Step 2: Run candidate UI tests to verify they fail**

Run: `npm --prefix web test -- --run src/features/relationships/CandidateReview.test.tsx`

Expected: FAIL because candidate-review UI is absent.

- [ ] **Step 3: Implement candidate and time controls**

Render candidate relation type, source/target names, confidence, summary, score fields, and linked memory evidence. Provide only `接受`, `编辑后接受`, and `拒绝`; no automatic confirmation. Disable controls during PATCH and replace candidate state only with server response. Keep error text inline.

Implement `SceneTimeline` as a fixed-height ordered control with scene objective and formatted timestamp. Clicking a scene sets `throughSceneId`, refetches scene derivations, and becomes source of truth for graph temporal selection. On mobile, horizontal-scroll timeline sits below the form rather than overlapping controls.

- [ ] **Step 4: Run all frontend tests and build**

Run: `npm --prefix web test -- --run && npm --prefix web run build`

Expected: PASS; pending state is preserved after failed PATCH.

- [ ] **Step 5: Commit review workflow**

```bash
git add web/src/api web/src/app.tsx web/src/features/relationships web/src/features/timeline web/src/features/derivations
git commit -m "feat: review relationship candidates in workbench"
```

## Task 8: Add Cytoscape Temporal Relationship Graph

**Files:**
- Create: `web/src/routes/StoryGraphRoute.tsx`
- Create: `web/src/features/graph/StoryGraphCanvas.tsx`
- Create: `web/src/features/graph/GraphFilters.tsx`
- Create: `web/src/features/graph/RelationshipHistoryDrawer.tsx`
- Create: `web/src/features/graph/StoryGraphCanvas.test.tsx`
- Modify: `web/src/app.tsx`
- Modify: `web/src/styles.css`

**Interfaces:**
- Consumes: `GET /api/story-graph?through_scene_id=`, relationship history API, and shared `throughSceneId` from Task 7.
- Produces: dedicated, responsive graph route with selection, focus, filters, history, and reset behavior.

- [ ] **Step 1: Write failing graph mapping tests**

```tsx
it("maps confirmed facts to solid graph edges and candidates to dashed edges", () => {
  render(<StoryGraphCanvas graph={graphWithFactAndCandidate} />);
  expect(cytoscape).toHaveBeenCalledWith(expect.objectContaining({
    elements: expect.arrayContaining([
      expect.objectContaining({ data: expect.objectContaining({ id: "fact-1", kind: "confirmed" }) }),
      expect.objectContaining({ data: expect.objectContaining({ id: "candidate-1", kind: "candidate" }) }),
    ]),
  }));
});

it("focuses a clicked node and clears focus when canvas is clicked", () => {
  const { cy } = renderGraph(graphWithFactAndCandidate);
  cy.emit("tap", nodeA);
  expect(cy.elements().addClass).toHaveBeenCalledWith("faded");
  cy.emit("tap", cy);
  expect(cy.elements().removeClass).toHaveBeenCalledWith("faded");
});
```

- [ ] **Step 2: Run graph test to verify it fails**

Run: `npm --prefix web test -- --run src/features/graph/StoryGraphCanvas.test.tsx`

Expected: FAIL because graph canvas and Cytoscape mapping do not exist.

- [ ] **Step 3: Implement graph route and canvas**

Initialize Cytoscape once in an effect with `cose` layout, `minZoom`, `maxZoom`, and styles keyed by `data(kind)` and relationship type. Use a `ref` for canvas host. On nodes, label only `data(label)` with character name. Confirmed edges are solid; candidate edges are dashed and excluded unless `showCandidates` is true.

Add direct interactions: node tap highlights `node.neighborhood().add(node)` and opens its detail drawer; edge tap fetches `/api/relationships/{factId}/history`; blank-canvas tap removes faded classes; `重置视图` runs `cy.fit()` then layout. Filter controls must use checkboxes/selects for relationship types, candidate visibility, and focused character.

The desktop route has a graph canvas plus a non-nested side inspector. At mobile width, the graph is full-screen and inspector becomes a drawer. Ensure canvas has a stable `min-height` and does not collapse while loading.

- [ ] **Step 4: Run graph tests and production build**

Run: `npm --prefix web test -- --run && npm --prefix web run build`

Expected: PASS; graph unit tests validate elements and event handling.

- [ ] **Step 5: Commit graph visualisation**

```bash
git add web/src/routes web/src/features/graph web/src/app.tsx web/src/styles.css
git commit -m "feat: visualize temporal relationship graph"
```

## Task 9: Verify Browser Workflow and Embedded Production Delivery

**Files:**
- Create: `web/e2e/workbench.spec.ts`
- Modify: `Cargo.toml`
- Modify: `src/api/mod.rs`
- Modify: `web/package.json`
- Modify: `.gitignore`
- Modify: `README.md`

**Interfaces:**
- Consumes: complete Rust API, built `web/dist`, and all frontend workflows.
- Produces: repeatable browser validation and documented one-process local run.

- [ ] **Step 1: Add Playwright configuration and failing end-to-end test**

```ts
test("author creates a scene, derives it, accepts a relationship candidate, and inspects graph history", async ({ page }) => {
  await page.goto("http://127.0.0.1:3000");
  await page.getByLabel("客观事件").fill("书房发现被撕毁的信");
  await page.getByRole("checkbox", { name: "沈砚" }).check();
  await page.getByRole("button", { name: "创建场景" }).click();
  await page.getByRole("button", { name: "开始推导" }).click();
  await page.getByRole("button", { name: "接受" }).click();
  await page.getByRole("link", { name: "关系图谱" }).click();
  await expect(page.locator("[data-testid=story-graph-canvas] canvas, [data-testid=story-graph-canvas] svg").first()).toBeVisible();
});
```

- [ ] **Step 2: Run the end-to-end test to verify it fails before server wiring**

Run: `npm --prefix web run test:e2e -- --project=chromium`

Expected: FAIL until the test harness starts the Rust server with deterministic mock LLM data.

- [ ] **Step 3: Add deterministic test harness and production checks**

Add `rust-embed = "8"` to `Cargo.toml` and a `#[derive(RustEmbed)] #[folder = "web/dist/"]` asset type in `src/api/mod.rs`. For every non-`/api/` request, return the embedded exact asset when present; otherwise return embedded `index.html` so browser routes reload correctly. Do not apply SPA fallback to `/api/` paths.

Add a `--mock-llm` runtime flag or `NOVELS_USE_MOCK_LLM=1` environment switch in `main.rs`; it must only select `MockSenseGenerator`, never expose mock mode over HTTP. Configure Playwright `webServer` to run `cargo run -- --mock-llm` after `npm --prefix web run build`.

Add a mobile project with `width: 390, height: 844`. Assert no horizontal viewport overflow, form controls remain visible, graph canvas is nonblank, timeline works, and an LLM failure card shows `重试` without hiding successful cards.

Add `web/node_modules/`, `web/dist/`, `playwright-report/`, `test-results/`, and `novels.db` to `.gitignore`. Update README with exact prerequisites, development commands, production build command, and `http://127.0.0.1:3000` URL. Explain that `DEEPSEEK_API_KEY` remains server-only and absence selects mock mode.

- [ ] **Step 4: Run full validation sequence**

Run:

```bash
npm --prefix web run build
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
npm --prefix web test -- --run
npm --prefix web run test:e2e -- --project=chromium
```

Expected: all commands pass. Inspect desktop and mobile screenshots; graph canvas contains visible nodes/edges and controls do not overlap.

- [ ] **Step 5: Commit verification and documentation**

```bash
git add Cargo.toml Cargo.lock src/api/mod.rs src/main.rs web/e2e web/package.json web/package-lock.json .gitignore README.md
git commit -m "test: verify story workbench browser flow"
```

## Plan Self-Review

### Spec coverage

- Rust API, same-origin hosting, and embedded frontend: Task 5.
- SQLite-only temporal fact, revision, candidate data: Tasks 1 and 2.
- Structured LLM candidate generation and validation: Task 3.
- Existing sensory, memory, and plot data in author-facing scene reads: Task 4.
- Desktop/mobile workbench and partial derivation failures: Task 6.
- Author accept/edit/reject behavior and scene time selection: Task 7.
- Cytoscape graph, filters, focus, history, candidates, and reset: Task 8.
- Security headers, local binding, no secret exposure, browser checks, static production delivery, and documentation: Task 9.

### Placeholder scan

The scan found no unfinished markers, deferred work, or implicit validation steps. Every code-changing task has defined files, types, test command, expected result, and commit boundary.

### Type consistency

All relationship mutations use `RelationshipCandidateId`, all confirmed graph/history reads use `RelationshipFactId`, and graph time selection uses `SceneId`. Candidate status is only `pending`, `accepted`, or `rejected`; revision status is only `active` or `superseded`.
