use clap::{Parser, Subcommand, error::ErrorKind};
use pdx_adapters::LocalStore;
use pdx_application::Application;
use pdx_core::{Diagnostic, StaticStatus};
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    version,
    about = "Pandocker-X: Docker不要の静的検証・計画 (PDF生成はR2)"
)]
struct Cli {
    #[arg(long, global = true, help = "version付きJSON objectを1個出力")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// 新規/空directoryを内蔵templateで初期化
    Init {
        path: PathBuf,
        #[arg(long, default_value = "default-report")]
        template: String,
    },
    /// 全Targetの静的検証。書込みなし
    Check { path: Option<PathBuf> },
    /// 選択Targetの静的BuildPlan。実行しない
    Plan {
        path: Option<PathBuf>,
        #[arg(long)]
        target: Option<String>,
    },
    /// ローカルtemplate依存のlockを明示更新。取得は行わない
    Lock { path: Option<PathBuf> },
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();
    // Detect JSON even when clap cannot construct Cli. After --, --json is a path.
    let json_mode = args
        .iter()
        .skip(1)
        .take_while(|s| *s != "--")
        .any(|s| s == "--json" || s.to_str().is_some_and(|s| s.starts_with("--json=")));
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            return if error.print().is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(5)
            };
        }
        Err(error) => {
            return emit(
                "arguments",
                json_mode,
                Err(Diagnostic::input("invalid_arguments", error.to_string())),
            );
        }
    };
    let app = Application::new(LocalStore);
    let (command,result)=match cli.command {
        Command::Init {path,template} => ("init",app.init(&path,&template).map(|root|json!({"project_root":root,"static_only":true,"runtime_verified":false,"build_ready":false}))),
        Command::Check {path} => ("check",app.check(path.as_deref()).map(|p|json!({"project_root":p.root,"targets":p.plans.keys().collect::<Vec<_>>(),"static_only":true,"runtime_verified":false,"build_ready":false}))),
        Command::Plan {path,target} => ("plan",app.plan(path.as_deref(),target.as_deref()).and_then(|p|serde_json::to_value(p).map_err(|e|Diagnostic::new("serialization",5,e.to_string())))),
        Command::Lock {path} => ("lock",app.lock(path.as_deref()).map(|p|json!({"project_root":p.root,"static_only":true,"runtime_verified":false,"build_ready":false}))),
    };
    emit(command, cli.json, result)
}
fn emit(command: &str, json_mode: bool, result: Result<Value, Diagnostic>) -> ExitCode {
    let (code, envelope) = match result {
        Ok(data) => (
            0,
            json!({"schema_version":1,"command":command,"status":"success","message":"static operation completed","data":data,"error":null,"unverified":StaticStatus::default().unverified}),
        ),
        Err(error) => (
            error.exit_code,
            json!({"schema_version":1,"command":command,"status":"error","message":error.message,"data":null,"error":error,"unverified":StaticStatus::default().unverified}),
        ),
    };
    let output = if json_mode {
        serde_json::to_string(&envelope)
    } else {
        serde_json::to_string_pretty(&envelope)
    };
    match output {
        Ok(output) => {
            if writeln!(io::stdout().lock(), "{output}").is_err() {
                return ExitCode::from(5);
            }
        }
        Err(_) => return ExitCode::from(5),
    }
    ExitCode::from(code)
}
