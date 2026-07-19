# Rust 代码质量优化设计

## 目标

在不改变主要公开数据结构、数据库表结构和核心 API 的前提下，修复当前 Rust 项目中的数据一致性和错误隐藏问题，并建立可执行的质量门禁。

## 已确认约束

- 数据库持久化数据解析失败时严格返回 `StoryError::Database`。
- LLM 重试使用第二次完整结果，感官、记忆和剧情必须来自同一次响应。
- 两次感官结果都无效时不落库，返回 `InvalidVocabularySelection`。
- 部分非法感官 ID 被剥离，合法 ID 继续保留。
- 感官字段必须匹配自身类别。
- 不新增依赖，不修改数据库表结构，不引入迁移。

## 业务流程设计

`StoryService::derive_character()` 将单次推导作为完整结果处理：

1. 加载场景、角色、私有记忆、最近感官和候选词汇。
2. 调用 LLM 获取完整 `LlmCharacterDerivation`。
3. 校验五感字段，保留合法 ID，收集并剥离非法 ID。
4. 若至少一个感官字段保留合法 ID，使用当前完整结果继续。
5. 若五感全空，再调用一次 LLM。
6. 第二次返回后，完整替换第一次结果，包括感官、记忆和剧情。
7. 第二次仍全空时，返回 `InvalidVocabularySelection`，不打开持久化事务。
8. 有效结果通过一次 SQLite 事务写入感官和新记忆。
9. 返回与数据库写入完全对应的完整推导结果。

## 词汇校验

校验同时检查全局候选集合和字段类别：

- `visual_ids` 只接受 `visual.*`。
- `auditory_ids` 只接受 `auditory.*`。
- `olfactory_ids` 只接受 `olfactory.*`。
- `tactile_ids` 只接受 `tactile.*`。
- `gustatory_ids` 只接受 `gustatory.*`。

`ValidationResult` 保留现有 `cleaned`、`stripped` 和 `all_empty` 语义。当前设计不改变部分合法结果的容错行为。

## 数据库错误边界

禁止数据库读取路径使用默认值掩盖损坏数据：

- UUID 解析失败返回 `StoryError::Database`。
- 角色、场景关联记录的字段读取失败返回 `StoryError::Database`。
- 感官 JSON 解析失败返回 `StoryError::Database`。
- 感官 JSON 中的非法 `VocabularyId` 返回 `StoryError::Database`。
- `MemorySource` 和 `Certainty` 反序列化失败返回 `StoryError::Database`。
- 内部枚举序列化失败传播为 `StoryError::Database`，不写入空字符串。

`CharacterRepo::update()` 检查主表更新影响行数。目标角色不存在时返回 `CharacterNotFound`，不替换标签和技能。

## 测试设计

新增或扩展以下测试：

- 重试使用第二次完整结果，验证感官、记忆、剧情均来自第二次响应。
- 两次感官均非法时，验证感官和记忆都没有新增记录。
- 合法但跨感官字段的 ID 被剥离。
- 损坏 UUID、JSON 和枚举读取返回 `StoryError::Database`。
- 更新不存在角色返回 `CharacterNotFound`。
- 多角色并发推导中单角色失败不影响其他角色。

## 质量门禁

实现完成后必须通过：

```text
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
```

现有严格 Clippy 报错的文档注释格式也纳入本次质量修复。实现不得修改用户已有的无关工作树变更。

## 非目标

- 不重构整个 Repository 架构。
- 不新增统一解析抽象层。
- 不修改 LLM Provider 或 Prompt 设计。
- 不添加日志、指标、HTTP API、认证或向量检索。
- 不自动修复已经存在的脏数据库数据。
