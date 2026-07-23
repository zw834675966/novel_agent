// CLI REPL 循环 + run_cli 入口
// ==============================
// `run_cli` 接管 clap 解析后的 `Cli`，负责 bootstrap + 分发到 REPL 或单次命令。
// REPL 循环从 stdin 读取行，用 clap 重新解析，通过 `execute` 执行。

use std::io::{self, BufRead, Write};

use clap::Parser;

use crate::bootstrap::{self, AppRuntime, BootstrapOptions};
use crate::cli::args::{Cli, Commands};
use crate::cli::commands::{self, CommandContext, CommandOutput};
use crate::cli::observation::Status;
use crate::cli::prose_cache::ProseCache;
use crate::cli::session::Session;

/// CLI 入口：bootstrap + 分发。
///
/// 返回进程退出码（0 = 成功/警告，1 = 错误）。
pub async fn run_cli(cli: Cli) -> anyhow::Result<i32> {
    let db_path = cli
        .db
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("novels.db"));

    let rt = bootstrap::bootstrap(BootstrapOptions { db_path }).await?;
    eprintln!("{}", rt.vocab_report_line);

    if rt.using_mock {
        println!(
            "status: warning\nsummary: DEEPSEEK_API_KEY 未设置，使用 Mock 生成器（非生产质量）\n"
        );
    }

    let json = cli.json;

    match cli.command {
        Some(Commands::Repl) => run_repl(rt, json).await,
        Some(cmd) => {
            let code = run_one_shot(rt, cmd, json).await?;
            Ok(code)
        }
        None => {
            // 无子命令时不进入 REPL（bare run 由 main.rs 处理 legacy 路径）
            // 如果走到这里说明 clap 配置允许无子命令但 main.rs 未拦截
            anyhow::bail!("no subcommand; use `novels repl` or bare `novels` for legacy mode")
        }
    }
}

/// 单次命令执行。
async fn run_one_shot(rt: AppRuntime, cmd: Commands, json: bool) -> anyhow::Result<i32> {
    let service = &rt.service;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();
    let mut ctx = CommandContext {
        service,
        session: &mut session,
        prose_cache: &mut prose_cache,
        using_mock: rt.using_mock,
        json,
        one_shot: true,
    };

    let output = commands::execute(cmd, &mut ctx).await;
    print_output(&output, json);
    Ok(status_to_exit(output.observation.status))
}

/// REPL 交互循环。
async fn run_repl(rt: AppRuntime, json: bool) -> anyhow::Result<i32> {
    let service = &rt.service;
    let mut session = Session::default();
    let mut prose_cache = ProseCache::default();

    eprintln!("novels REPL - 输入命令，quit/exit 退出");
    eprintln!("  示例: character create 宝玉 --tags 痴情");
    eprintln!("        scene create 事件 --with 宝玉");
    eprintln!("        derive --character 宝玉");
    eprintln!("        narrate");
    eprintln!("        show derivation");
    eprintln!("        show prose");

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        // prompt -> stderr（保持 stdout 干净以支持正文管道）
        eprint!("> ");
        if io::stderr().flush().is_err() {
            break;
        }

        let line = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(_)) | None => break, // EOF or error
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "quit" || trimmed == "exit" {
            break;
        }

        let args = split_repl_line(trimmed);
        let argv: Vec<String> = std::iter::once("novels".to_string())
            .chain(args)
            .collect();

        let parsed_cli = match Cli::try_parse_from(argv) {
            Ok(c) => c,
            Err(e) => {
                // clap 错误（解析失败）直接输出到 stderr
                eprintln!("{e}");
                continue;
            }
        };

        let cmd = match parsed_cli.command {
            Some(Commands::Repl) => {
                eprintln!("已在 REPL 中，无需嵌套");
                continue;
            }
            Some(c) => c,
            None => {
                eprintln!("请输入命令（输入 quit 退出）");
                continue;
            }
        };

        let mut ctx = CommandContext {
            service,
            session: &mut session,
            prose_cache: &mut prose_cache,
            using_mock: rt.using_mock,
            json,
            one_shot: false,
        };

        let output = commands::execute(cmd, &mut ctx).await;
        print_output(&output, json);
    }

    Ok(0)
}

/// 打印命令输出：observation（render）+ 可选正文。
fn print_output(output: &CommandOutput, json: bool) {
    print!("{}", output.observation.render(json));
    if let Some(body) = &output.body {
        println!("{body}");
    }
    // 确保输出刷新
    let _ = io::stdout().flush();
}

/// 状态 -> 进程退出码。
fn status_to_exit(status: Status) -> i32 {
    match status {
        Status::Error => 1,
        Status::Success | Status::Warning => 0,
    }
}

/// REPL 行分割：按空白拆分，双引号内保留空格。
///
/// ```text
/// character create 宝玉 --tags 痴情
///   -> ["character", "create", "宝玉", "--tags", "痴情"]
///
/// scene create "宝玉挨打" --with "宝玉,黛玉"
///   -> ["scene", "create", "宝玉挨打", "--with", "宝玉,黛玉"]
/// ```
fn split_repl_line(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_simple() {
        assert_eq!(
            split_repl_line("character create 宝玉 --tags 痴情"),
            vec!["character", "create", "宝玉", "--tags", "痴情"]
        );
    }

    #[test]
    fn split_quoted() {
        assert_eq!(
            split_repl_line("scene create \"宝玉挨打\" --with \"宝玉,黛玉\""),
            vec!["scene", "create", "宝玉挨打", "--with", "宝玉,黛玉"]
        );
    }

    #[test]
    fn split_empty() {
        assert_eq!(split_repl_line(""), Vec::<String>::new());
        assert_eq!(split_repl_line("   "), Vec::<String>::new());
    }

    #[test]
    fn split_unclosed_quote_keeps_rest() {
        // 未闭合引号：剩余部分作为一个 token
        assert_eq!(
            split_repl_line("scene create \"宝玉挨打"),
            vec!["scene", "create", "宝玉挨打"]
        );
    }

    #[test]
    fn status_to_exit_mapping() {
        assert_eq!(status_to_exit(Status::Success), 0);
        assert_eq!(status_to_exit(Status::Warning), 0);
        assert_eq!(status_to_exit(Status::Error), 1);
    }
}
