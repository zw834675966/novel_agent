// novels 应用入口
// ==================
// 三种运行模式：
//   1. 裸 `cargo run`（无子命令）-> 演示流程 + API :3000（legacy 模式）
//   2. `cargo run -- repl` -> 交互式 REPL
//   3. `cargo run -- <command>` -> 单次命令执行

use std::path::PathBuf;

use clap::Parser;
use novels::bootstrap::{self, BootstrapOptions};
use novels::cli::Cli;
use novels::models::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    match &cli.command {
        None => {
            // Legacy: 演示 + API
            let db_path = cli.db.clone().unwrap_or_else(|| PathBuf::from("novels.db"));
            let rt = bootstrap::bootstrap(BootstrapOptions { db_path }).await?;
            eprintln!("{}", rt.vocab_report_line);
            if rt.using_mock {
                eprintln!("DEEPSEEK_API_KEY not set, using mock generators");
            }
            run_legacy(rt).await
        }
        Some(_) => {
            let code = novels::cli::run_cli(cli).await?;
            std::process::exit(code);
        }
    }
}

/// Legacy 演示流程 + API 服务。
async fn run_legacy(rt: novels::bootstrap::AppRuntime) -> anyhow::Result<()> {
    let svc = rt.service;

    // 绑定端口并启动 API Web 服务
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("API listening on http://127.0.0.1:3000");

    let server_svc = svc.clone();
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, novels::api::app(server_svc)).await {
            eprintln!("Web server error: {e}");
        }
    });

    // ---- 演示流程 ----
    // 1. 创建一个角色
    let cid = CharacterId(uuid::Uuid::new_v4());
    svc.db()
        .characters()
        .create(cid, "侦探", &["谨慎".to_string()], &["推理".to_string()])
        .await?;

    // 2. 创建一个场景
    let sid = svc
        .create_scene(CreateScene {
            objective_event: "古宅发现一具尸体".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await?;

    // 3. 对场景中的角色进行推导
    let results = svc.derive_scene(sid).await;
    let derivations: Vec<_> = results
        .iter()
        .filter_map(|r| r.as_ref().ok().cloned())
        .collect();
    for d in &derivations {
        println!("{:?}", d);
    }

    // 4. 把结构化推导编排成小说正文
    match svc.narrate_scene(sid, &derivations).await {
        Ok(prose) => {
            println!(
                "
=== 正文 ===
{}
=== stripped refs: {} | rejected beats: {} | action-only beats: {} | quote density: {:.2} | unverified quotes: {} | low_quote_density: {} ===",
                prose.text,
                prose.stripped_refs,
                prose.rejected_beats,
                prose.action_only_beats,
                prose.quote_density(),
                prose.unverified_quotes,
                prose.quality.low_quote_density
            );
        }
        Err(e) => eprintln!("narrate err: {e}"),
    }

    // 保持主线程持续运行以维持 Web 服务
    futures::future::pending::<()>().await;
    Ok(())
}
