# Story Workbench Knowledge Graph Design

## Status

Approved design. Implementation has not started.

## Goal

Deliver a Chinese-first local authoring workbench for the existing novel
character-perception engine. An author creates an objective scene, selects its
participants, runs character derivation, reviews character-private outcomes,
and confirms or rejects proposed relationship changes.

The workbench must make evolving character relationships inspectable without
allowing LLM output to silently become story canon.

## Scope

### Included

- An Axum JSON API over the existing `StoryService` and repositories.
- A React/Vite workbench, built into static assets and served by the Rust
  binary in production.
- Character creation and browsing.
- Scene creation, participant selection, and derivation execution.
- Rendering sensory selections, private memories, and plot developments.
- Relationship-change candidates proposed by the LLM.
- Author acceptance, editing, and rejection of candidates.
- A temporal relationship graph with Cytoscape.js.
- Desktop and mobile browser coverage for primary workflows and failure states.

### Excluded

- Authentication, multi-user sharing, and cloud deployment.
- Replacing SQLite with Neo4j, FalkorDB, or another graph database.
- Automatically importing an existing manuscript into a graph.
- Automatic LLM confirmation of relationship facts.
- Vector search, embeddings, GraphRAG, LightRAG, or Graphiti runtime
  integration.

## Product Principles

- Chinese author-facing copy; do not surface UUIDs, database table names, or
  vocabulary IDs in normal UI.
- Existing domain data remains canonical. The graph is a read model, not a
  second source of truth.
- A relationship candidate is advisory until author confirmation.
- Every confirmed relationship change is time-bounded and traceable to scene
  and memory evidence.
- One failed character derivation must not discard results for other scene
  participants.

## Research Decision

Graphiti, Microsoft GraphRAG, and LightRAG were evaluated as reference
projects. Their extraction, temporal provenance, and graph retrieval concepts
are useful, but their Python services, graph backends, embeddings, and LLM
indexing pipelines do not fit this local SQLite-first product slice.

Borrowed concepts:

- Graphiti: validity windows, episode-style evidence, retained historical
  facts.
- GraphRAG and LightRAG: LLM-extracted facts must retain source provenance and
  must not be treated as ground truth without product controls.
- Tension Map: relationship graph, perspective focus, temporal navigation, and
  analytical relation lenses.
- Cytoscape.js: graph rendering, `cose` layout, click selection, neighbour
  highlighting, panning, zooming, and filtering.

Use Cytoscape.js under its MIT license for the frontend graph canvas. Do not
copy UI code or data from reference projects.

## System Architecture

```text
React/Vite SPA
  -> typed JSON client
  -> Axum routes and DTO validation
  -> StoryService and repositories
  -> SQLite (novels.db)
  -> RigSenseGenerator or MockSenseGenerator
```

Development uses the Vite dev server and Axum API separately. Production runs
one Rust binary that serves the compiled Vite assets and API from the same
origin.

HTTP handlers orchestrate requests and map errors. They do not execute SQL.
`StoryService` remains the application boundary for scene creation and
character derivation. Relationship-specific application services may use
repositories but must remain separate from route handlers.

## Data Model

Existing tables remain the canonical source for character, scene, memory,
sensation, and plot-development data. Do not add a generic graph-node table.
Graph nodes are projected on read from those existing records.

Add relationship persistence:

### `relationship_facts`

Represents a durable directed relationship between two characters.

- `id`
- `from_character_id`
- `to_character_id`
- `relationship_type`
- `created_at`
- `created_by` (`author` or `llm_confirmed` is disallowed; only `author` for
  this release)

The unique key is `(from_character_id, to_character_id, relationship_type)`.
Reciprocal relationships are represented explicitly if their semantics differ.

### `relationship_revisions`

Represents a time-bounded state of one relationship fact.

- `id`
- `relationship_fact_id`
- `scene_id`
- `valid_from_scene_id`
- `valid_until_scene_id` (nullable while current)
- `status` (`active` or `superseded`)
- `summary`
- `tension_score`, `trust_score`, `affection_score`, `power_score` (nullable
  integer range 0 through 100)
- `evidence_memory_id` (nullable)
- `created_at`

When an author accepts a new revision for a fact, its prior active revision is
closed in the same transaction. Graph snapshots include revisions active at or
before the selected scene.

### `relationship_candidates`

Stores LLM-proposed changes without making them facts.

- `id`
- `scene_id`
- `from_character_id`
- `to_character_id`
- `relationship_type`
- proposed summary and four optional scores
- `evidence_memory_id` (nullable)
- `confidence`
- `status` (`pending`, `accepted`, `rejected`)
- `created_at`, `resolved_at`

Candidate acceptance creates or updates a relationship fact and creates a
revision atomically. Editing happens before acceptance and records the
author-approved values in the revision. Rejection changes only candidate state.

Allowed relationship types in first release:

`knows`, `allied_with`, `distrusts`, `owes`, `family_of`, `mentors`,
`conflicts_with`.

Participation, memory, and causal plot links are projected from existing scene
and derivation data; they are not author-editable relationship facts in this
release.

## LLM Contract and Validation

Extend the structured LLM derivation contract with zero or more relationship
change candidates. Each candidate contains both character references, an
allowed relationship type, a short Chinese explanation, optional score values,
and evidence reference when available.

Validation at service boundary must:

1. Verify both characters exist and are distinct.
2. Verify relationship type is allowlisted.
3. Verify scores are integers from 0 through 100 when present.
4. Verify an evidence memory belongs to the derived character and scene when
   supplied.
5. Persist valid candidates only; invalid candidates are rejected individually
   without failing unrelated sensory or memory results.

LLM output is untrusted data. It cannot choose SQL, routes, HTML, storage
paths, or author-confirmed state.

## API Contract

All successful payloads are JSON. Error payloads use:

```json
{
  "code": "VALIDATION_ERROR",
  "message": "中文错误说明",
  "fields": { "objectiveEvent": "不能为空" }
}
```

`fields` is optional. Domain absence maps to `404`, conflicting state to
`409`, invalid input to `422`, and unexpected LLM or storage failures to
`500` without internal details.

| Method | Route | Purpose |
| --- | --- | --- |
| `GET` | `/api/characters` | List author-facing character summaries. |
| `POST` | `/api/characters` | Create a character. |
| `GET` | `/api/scenes` | List scenes in story order. |
| `POST` | `/api/scenes` | Create a scene with objective event, time, and participants. |
| `GET` | `/api/scenes/:scene_id` | Read scene and participants. |
| `POST` | `/api/scenes/:scene_id/derive` | Derive all participants; returns per-character outcomes. |
| `GET` | `/api/scenes/:scene_id/derivations` | Read sensory results, private memories, plot development, and candidates. |
| `GET` | `/api/story-graph?through_scene_id=:scene_id` | Read temporal node, edge, and timeline projection. |
| `PATCH` | `/api/relationship-candidates/:id` | Accept, edit-and-accept, or reject candidate. |
| `GET` | `/api/relationships/:id/history` | Read fact revisions and linked evidence. |

`POST /derive` returns a result object per participant. A failed derivation is
represented alongside successful results rather than failing the complete
batch.

## Workbench UX

### Scene Workbench

Default landing screen, optimized for the author workflow.

- Left sidebar: searchable character list and participant selection.
- Main area: scene editor for objective event, occurrence time, and selected
  participants. Submission has visible running, success, partial-failure, and
  retry states.
- Right inspector: tabs or compact sections for each participant's sensory
  impressions, private memories, plot development, and relationship candidates.
- Candidate controls: accept, edit then accept, and reject. Candidate state
  updates immediately only after server confirmation.
- Bottom timeline: scenes ordered by occurrence and creation tie-breaker;
  selecting a scene changes the inspector and graph time context.

### Relationship Graph

Dedicated route or focused full-screen view launched from the workbench.

- Cytoscape.js `cose` layout on desktop.
- Default subset: selected scene participants and one-hop confirmed
  relationships.
- Nodes: characters, labeled with author-facing names.
- Confirmed edges: colour and label by relationship type.
- Candidate edges: dashed and excluded from default graph layout unless the
  candidate filter is enabled.
- Clicking a node focuses its neighbourhood and opens character details.
- Clicking an edge opens relationship history, revision data, and evidence.
- Blank-canvas click clears focus. Graph supports pan, zoom, and a reset-view
  command.
- Filters: relationship type, confirmed/candidate state, selected character,
  and temporal snapshot.
- Mobile: workbench prioritizes the scene form. Graph opens as a dedicated
  screen; drawers replace persistent sidebars.

## Frontend Structure

```text
web/
  src/
    api/            Typed request and response client
    features/
      characters/
      scenes/
      derivations/
      relationships/
      graph/
    routes/
      WorkbenchRoute
      StoryGraphRoute
    components/     Shared controls and status primitives
```

Use React state local to features. Introduce a shared store only when scene
selection and graph time selection require it. Do not introduce a query cache
or client-side graph database in first release unless concrete repeated fetch
or synchronization needs appear.

## Security and Operational Boundaries

- Bind local development server to loopback by default.
- Use same-origin static assets and API in production; no permissive CORS.
- Validate JSON bodies and request lengths at Axum boundaries.
- Return generic internal-error messages; log contextual errors without API
keys, tokens, or raw secrets.
- Preserve current `.env` handling for `DEEPSEEK_API_KEY`; never expose it to
  frontend assets or API responses.
- Set restrictive security headers for static assets and API responses.

## Verification

### Rust Unit and Integration Tests

- Candidate validation rejects invalid characters, types, scores, and evidence.
- Candidate acceptance atomically creates or revises a fact and closes prior
  active revision.
- Candidate rejection preserves relationship facts.
- Graph projection returns only relations active at requested scene.
- Existing derivation transaction behavior remains intact.

### API Tests

- Request validation and Chinese error responses.
- Correct `404`, `409`, `422`, and `500` mappings.
- Batch derivation preserves successful participant results during partial
  failure.

### Frontend Tests

- Scene creation and participant selection.
- Candidate accept, edit-and-accept, and reject states.
- Graph filter and temporal selection state.

### Browser Tests

- Desktop and mobile workbench layouts.
- Empty, loading, partial-failure, and retry states.
- Cytoscape graph renders nonblank nodes and edges.
- Node, edge, timeline, and filter interactions update visible detail.

## Acceptance Criteria

1. An author can create a character and scene, select participants, and derive
   the scene from a browser.
2. Each successful participant result displays sensory selections and private
   memory without exposing internal IDs.
3. Relationship candidates never appear as confirmed facts until an author
   explicitly accepts them.
4. An accepted candidate produces a dated relationship revision with scene and
   optional memory evidence.
5. A graph snapshot at a selected scene shows only confirmed relations valid
   at that time; pending candidates appear only when enabled.
6. An author can inspect a relationship's prior revisions and evidence.
7. The built production frontend is served by the Rust application as one local
   process.
8. Existing Rust tests and newly added tests pass; browser tests cover the
   primary desktop and mobile paths.
