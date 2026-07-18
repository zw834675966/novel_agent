# Repository Guide（仓库指南）

## Scope（项目范围）

- This is one Rust 2024 binary crate, not a workspace. The only application entrypoint is `src/main.rs`.
  （这是一个独立的 Rust 2024 二进制 crate，不是工作空间。唯一的应用入口是 `src/main.rs`。）
- `rig-lancedb` is declared but not used yet. Do not infer a vector-store architecture from the dependency alone.
  （`rig-lancedb` 已声明但尚未使用。不要仅从依赖推断出向量存储架构。）

## Commands（命令）

- Fast compile check（快速编译检查）: `cargo check --all-targets`
- Full test command（完整测试）: `cargo test --all-targets`
- Formatting check（格式检查）: `cargo fmt --all -- --check`
- Lint（代码检查）: `cargo clippy --all-targets --all-features -- -D warnings`
- Run the current agent（运行当前 agent）: Set `DEEPSEEK_API_KEY` in `.env` or shell, then run `cargo run`.
- No test targets exist yet; focused tests use Cargo's normal filter form: `cargo test <test-name>`.
  （暂无测试目标；针对性的测试使用 Cargo 的标准过滤方式。）

## Runtime Wiring（运行时依赖）

- Uses `rig::providers::deepseek` (not OpenAI compat layer).
  （使用 rig 内置的 DeepSeek provider，而非 OpenAI 兼容层。）
- `src/main.rs` loads `.env` file via `dotenv::dotenv().ok()`.
  （代码会通过 dotenv 自动加载 .env 文件。）
- Reads `DEEPSEEK_API_KEY` from environment.
  （从环境读取 DEEPSEEK_API_KEY。）
- Configured model: `deepseek::DEEPSEEK_CHAT` (deprecated alias for `deepseek-v4-flash`).
  （配置的模型：deepseek::DEEPSEEK_CHAT，是 deepseek-v4-flash 的弃用别名。）

## Windows Setup（Windows 特定设置）

- `protoc` is required for `lance-*` crates. Installed at `C:\Tools\protoc\bin\protoc.exe`.
  （lance 相关 crate 需要 protoc。已安装到 `C:\Tools\protoc\bin\protoc.exe`。）
- Set `PROTOC` env var in shell or user environment: `$env:PROTOC = "C:\Tools\protoc\bin\protoc.exe"`
  （在 shell 或用户环境中设置 PROTOC 环境变量。）
