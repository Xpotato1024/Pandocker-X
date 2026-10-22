use clap::{Parser, Subcommand, error::ErrorKind};
use pdx_adapters::{DockerRuntime, LocalStore, default_state_root};
use pdx_application::{Application, BuildSettings, Cancellation};
use pdx_core::{Diagnostic, StaticStatus};
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};
mod display;

#[derive(Parser)]
#[command(
    version,
    about = "Pandocker-X: 静的検証と固定Docker runtimeによるPDF生成"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "version付きJSON objectをstdoutに1個出力。既定は人間向け表示"
    )]
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
    Lock {
        path: Option<PathBuf>,
        #[arg(long)]
        runtime_image: Option<String>,
    },
    /// 固定local runtimeでPDFを新しい世代へ生成
    Build {
        path: Option<PathBuf>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u64).range(1..=600))]
        timeout_seconds: u64,
    },
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
        Command::Lock {path,runtime_image:None} => ("lock",app.lock(path.as_deref()).map(|p|json!({"project_root":p.root,"static_only":true,"runtime_verified":false,"build_ready":false}))),
        Command::Lock {path,runtime_image:Some(image)} => ("pin-runtime", cancellation().and_then(|cancel|app.pin_runtime(path.as_deref(), &image, &DockerRuntime::default(), &cancel)).map(|v|json!({"driver":v.driver,"runtime_verified":v.runtime_verified,"tools":v.tools,"static_only":false,"build_ready":false,"unverified":["PDF generation, typesetting conformance and cross-host reproducibility"]}))),
        Command::Build {path,target,timeout_seconds} => ("build", cancellation().and_then(|cancel| app.build(path.as_deref(),target.as_deref(),&BuildSettings {state_root:default_state_root()?,timeout:std::time::Duration::from_secs(timeout_seconds)},&DockerRuntime::default(),&cancel)).and_then(|v|serde_json::to_value(v).map_err(|e|Diagnostic::new("serialization",5,e.to_string())))),
    };
    emit(command, cli.json, result)
}
fn cancellation() -> Result<Cancellation, Diagnostic> {
    let cancel = Cancellation::default();
    let handler = cancel.clone();
    ctrlc::set_handler(move || handler.cancel())
        .map_err(|e| Diagnostic::new("signal_handler", 3, e.to_string()))?;
    Ok(cancel)
}
fn emit(command: &str, json_mode: bool, result: Result<Value, Diagnostic>) -> ExitCode {
    if !json_mode {
        let code = result.as_ref().err().map_or(0, |error| error.exit_code);
        return if display::write_result(
            &mut io::stdout().lock(),
            &mut io::stderr().lock(),
            command,
            &result,
        )
        .is_ok()
        {
            ExitCode::from(code)
        } else {
            ExitCode::from(5)
        };
    }
    let (code, envelope) = match result {
        Ok(data) => (
            0,
            json!({"schema_version":1,"command":command,"status":"success","message":"operation completed","unverified":data.get("unverified").cloned().unwrap_or_else(||json!(StaticStatus::default().unverified)),"data":data,"error":null}),
        ),
        Err(error) => (
            error.exit_code,
            json!({"schema_version":1,"command":command,"status":"error","message":error.message,"data":null,"error":error,"unverified":StaticStatus::default().unverified}),
        ),
    };
    let output = serde_json::to_string(&envelope);
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
