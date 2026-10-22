//! Human-facing presentation only. Machine clients use the versioned JSON envelope.
use pdx_core::Diagnostic;
use serde_json::Value;
use std::io::{self, Write};

fn field<'a>(data: &'a Value, name: &str) -> &'a str {
    data[name].as_str().unwrap_or("")
}

fn names(data: &Value, name: &str) -> String {
    data[name]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn write_result(
    stdout: &mut impl Write,
    stderr: &mut impl Write,
    command: &str,
    result: &Result<Value, Diagnostic>,
) -> io::Result<()> {
    let data = match result {
        Ok(data) => data,
        Err(error) => {
            writeln!(
                stderr,
                "エラー [{}] (exit {}): {}",
                error.code, error.exit_code, error.message
            )?;
            if let Some(path) = &error.instance_path {
                writeln!(stderr, "metadata位置 (JSON Pointer): {path:?}")?;
            }
            if let Some(hint) = recovery_hint(error.code) {
                writeln!(stderr, "次: {hint}")?;
            }
            return Ok(());
        }
    };
    match command {
        "init" => {
            writeln!(
                stdout,
                "Projectを作成しました: {}",
                field(data, "project_root")
            )?;
            writeln!(
                stdout,
                "次: 作成先で pdx check、続いて pdx plan を実行してください。"
            )?;
        }
        "check" => {
            writeln!(
                stdout,
                "静的検証に成功しました: {}",
                field(data, "project_root")
            )?;
            writeln!(stdout, "Target: {}", names(data, "targets"))?;
            writeln!(
                stdout,
                "次: pdx plan [path] --target <id> で選択結果を確認してください。"
            )?;
        }
        "lock" => {
            writeln!(
                stdout,
                "ローカルtemplate依存を固定しました: {}",
                field(data, "project_root")
            )?;
            writeln!(
                stdout,
                "次: pdx check [path]、続いて pdx plan [path] を実行してください。"
            )?;
        }
        "plan" => {
            writeln!(
                stdout,
                "Target: {} / Document: {}",
                field(data, "target"),
                field(data, "document")
            )?;
            writeln!(
                stdout,
                "Template: {} @ {}",
                field(&data["template"], "id"),
                field(&data["template"], "version")
            )?;
            let source = &data["template_source"];
            if let Some(path) = source["path"].as_str() {
                writeln!(stdout, "Template path: {path}")?;
            } else {
                writeln!(stdout, "内蔵template: {}", field(source, "id"))?;
            }
            writeln!(stdout, "Digest: {}", field(&data["template"], "digest"))?;
            writeln!(stdout, "入力 (宣言順): {}", names(data, "ordered_inputs"))?;
            writeln!(stdout, "Bibliography: {}", names(data, "bibliography"))?;
            writeln!(stdout, "Resources: {}", names(data, "resources"))?;
            writeln!(stdout, "Metadata: {}", names(data, "metadata_inputs"))?;
            writeln!(stdout, "出力先: {}", field(data, "output_dir"))?;
            writeln!(
                stdout,
                "必要tool (未確認): {}",
                names(data, "required_tools")
            )?;
            writeln!(
                stdout,
                "全field・結合metadataは pdx plan [path] --json で確認できます。"
            )?;
        }
        _ => unreachable!("only implemented operations produce success"),
    }
    writeln!(
        stdout,
        "静的操作のみ。PDF生成は未実装 (R2)。runtime未確認、build_ready=false。"
    )
}

fn recovery_hint(code: &str) -> Option<&'static str> {
    match code {
        "invalid_arguments" => {
            Some("pdx --help または pdx <command> --help で引数を確認してください。")
        }
        "init_collision" => {
            Some("既存原稿を保持し、新規または空のdirectoryを初期化先に指定してください。")
        }
        "project_not_found" | "missing_input" => {
            Some("pdx.toml と宣言した入力の存在を確認し、Projectのpathを明示してください。")
        }
        "unknown_target" => {
            Some("pdx check [path] のTarget一覧と pdx.toml の default_target を確認してください。")
        }
        "stale_lock" => Some(
            "templateの変更内容を確認し、意図した変更なら pdx lock [path] で明示更新してください。",
        ),
        "missing_dependency" => Some(
            "pdx.lock と選択したtemplate資産を確認してください。ローカル依存の準備後、pdx lock [path] で固定できます。",
        ),
        "invalid_template_ref" => {
            Some("pdx.toml の template には内蔵idまたはProject内pathを一つだけ指定してください。")
        }
        "invalid_metadata" | "metadata_schema" => Some(
            "宣言したmetadataとtemplateのmetadata.schema.jsonを確認し、表示された位置・型・必須項目を修正してください。",
        ),
        "unsafe_path" => {
            Some("pdx.toml の相対path、出力と入力の重なり、リンク、大小文字を確認してください。")
        }
        _ => None,
    }
}
