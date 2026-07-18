// rig 是一个 Rust 的 LLM 抽象库，支持多种 AI 提供商
// CompletionClient trait：定义补全/对话能力
// ProviderClient trait：定义 AI 提供商客户端的通用接口
use rig::client::{CompletionClient, ProviderClient};
// deepseek 模块：DeepSeek 原生客户端
use rig::providers::deepseek;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
struct ExtractorSpike {
    color: String,
    count: u8,
}

// #[tokio::main] 是一个宏，将 async fn main 转换为 tokio 运行时入口
// tokio 是 Rust 的异步运行时，rt-multi-thread 特性启用多线程调度
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // dotenv().ok() 从项目根目录加载 .env 文件
    // .ok() 表示如果文件不存在也静默忽略，不报错
    // 加载后，环境变量（如 OPENAI_API_KEY）就可以通过 std::env::var 读取到
    dotenv::dotenv().ok();

    // deepseek::Client::from_env() 从环境变量 DEEPSEEK_API_KEY 创建客户端
    // 也可以用 deepseek::Client::new("api-key")? 显式传入
    let client = deepseek::Client::from_env()?;

    let extractor = client
        .extractor::<ExtractorSpike>(deepseek::DEEPSEEK_V4_FLASH)
        .retries(1)
        .build();
    let response = extractor.extract("I saw 3 red apples.").await?;

    println!("{response:?}");

    // main 函数返回 Ok(())，表示程序正常结束
    // 如果前面任何 ? 捕获到错误，main 会返回 Err，Rust 运行时会打印该错误
    Ok(())
}
