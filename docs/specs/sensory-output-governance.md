# Sensory Output Governance Remediation — Eng Spec

## Goal
治理感知输出降级为纯动作/情绪、弱势感官被挤压、Prompt 含 Rust Debug `{:?}` 噪声、LLM 缺少 CoT 思考、装配门禁不足 5 大结构性问题。

## Non-goals
- 不更换 LLM provider / 模型（保持 `rig::providers::deepseek` + `deepseek::DEEPSEEK_V4_FLASH`）
- 不删除、移动、重写 `corpus/`、`assets/distilled/` 或任何 unrelated user WIP
- 不破坏 `validate_selection` 白名单过滤逻辑
- 不破坏 `DerivationRepo::replace_derivation` 事务原子性
- 不改变总候选硬顶 96

## Scope by task
| Task | In scope | Out of scope |
|------|----------|--------------|
| 1 扩充词库 + 蒸馏配额 | `assets/vocab.yaml` 扩充 50–100 条；`tools/distill_quality_report.py` 增加感官桶采样比例校验 | 重写 corpus extraction pipeline |
| 2 检索配额 + 场景焦点 | `src/vocab/loader.rs` 新增 `candidates_ranked_limited_with_quotas`；`src/scene/service.rs` 替换调用点 | 改变 BM25 核心算法 |
| 3 Prompt 渲染重构 | `src/models/action.rs` 新增 `MemoryContentSlot::display_narrative()`；`src/llm/rig_impl.rs` 清除 `{:?}` | 改变 LLM provider |
| 4 CoT Schema | `src/llm/contract.rs` 新增 `sensory_analysis`；`src/llm/mock.rs` + prompt 同步 | 替换 rig Extractor |
| 5 装配门禁 | `src/prose/assembly.rs` `ProseQualityReport` 新增 `sensory_diversity_score` / `missing_senses` / `DegradedSensoryDensity`；assemble 优先基础词库引用 | 改变 narration LLM contract |
| 6 文档 + 门禁 | `AGENTS.md` 更新；全套 `cargo fmt / check / test / clippy` | — |

## Eng ordered steps
1. **Task 3** — `display_narrative()` + prompt cleanup（纯渲染，零 DB 影响）
2. **Task 4** — `sensory_analysis` CoT field + mock + prompt（纯 contract）
3. **Task 2** — quota retrieval + scene focus weighting（纯 vocab + service wiring）
4. **Task 1** — `vocab.yaml` 扩充 + Python 工具感官桶采样（数据层，与代码解耦）
5. **Task 5** — `ProseQualityReport` 新字段 + assemble 基础词库优先（依赖 2/3 稳定）
6. **Task 6** — `AGENTS.md` + quality gates

## Acceptance criteria
- [ ] `cargo test --test vocab_test` 通过，包含 quota 新测试
- [ ] `cargo test --test scene_test` 通过，包含 prompt 渲染 + quality gate 新测试
- [ ] `cargo test --test e2e` 通过，mock fixture 能反序列化 `sensory_analysis`
- [ ] `cargo check --all-targets` 通过（Windows 已知 Lance 基线风险：若失败需比对 baseline，不可归因于本改动）
- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 通过
- [ ] `build_derivation_prompt` 输出中不再出现 `{:?}` Debug 语法
- [ ] 在倾斜语料下（gesture/emotion 占 61%），弱势感官（嗅/味/触/听）候选保底 ≥4 条/类
- [ ] `ProseQualityReport` 五感覆盖为 0 时标记 `DegradedSensoryDensity`
- [ ] `assets/vocab.yaml` 五感条目数 ≥50（不含 emotion/gesture/atmosphere）

## Risks
- **Windows Lance baseline**: `cargo check --all-targets` 可能因 `rig-lancedb` / lance 7.0.0 build script 失败；与 Plan 无关，需先比对 baseline。
- **Prompt stability**: Task 3/4 改变 prompt 文本，需确认 mock + scene_test 中 prompt 断言已同步。
- **Quota determinism**: quota 截断需保持稳定排序（sense 序 → id 序），否则 BM25 测试会抖动。
- **Out of scope**: 不扩展 `SensorySelection` 字段、不新增 DB 列、不修改 `DerivationRepo` 事务边界。

## Spec path
`docs/specs/sensory-output-governance.md`
