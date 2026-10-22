---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/contracts/cli.md
---

# CLI・結果契約

R1で実装するのは `init` / `check` / `plan` / ローカル `lock` です。CLIと後続GUIは同じapplication APIを呼びます。以下の他コマンドは将来計画であり、CLIへ成功するstubとして登録しません。

| 計画コマンド | 効果・境界 |
| --- | --- |
| `pdx init <path> --template default-report` | R1実装。内蔵雛形から新規/空directoryへ作成。template省略時もdefault-report |
| `pdx check [path]` | 設定・入力参照・metadata・template/lock整合を静的検証 |
| `pdx plan [path] --target <id> --json` | 検証済みBuildPlanを出力。runtime可用性や成功は保証しない |
| `pdx lock [path]` | R1実装。ローカルtemplate依存の明示固定。取得せずruntimeはunresolved |
| `pdx fetch` | lockに固定した依存を明示取得・digest検証 |
| `pdx build --target <id>` / `pdx build --all-targets` | 準備済み依存からPDF化。download/lock更新/Git操作はしない |
| `pdx doctor` | OS・Docker・必要tool・固定依存の状態を診断。自動インストールしない |
| `pdx template list` / `add` / `check` | 登録一覧、明示取得/追加、package静的検証。安全境界は緩めない |
| `pdx clean` / `pdx clean --apply` | 既定はdry-run。--applyだけが確認済みアプリ所有物を削除 |

`[path]` 省略時はcurrent directoryから最も近い親の `pdx.toml` を探索します。明示pathがあればそのProjectを使い、InstallRootからは推測しません。target省略時はProjectのdefault_target、all-targetsとの併用は入力エラーです。初期のall-targetsは逐次実行で、一つ失敗すれば全体を非0にします。

init/check/planはDocker・Git・network不要です。未登録templateや宣言不足は明確な入力/依存エラーにし、取得を開始しません。buildは依存を勝手に取得せず、不足ならfetchが必要と診断します。

## 終了コード

| code | 意味 |
| --- | --- |
| 0 | その操作の契約を満たしたsuccess |
| 1 | 組版または今回のartifact検証失敗 |
| 2 | 入力・設定の不正 |
| 3 | runtime・依存の不足/不整合 |
| 4 | 安全境界の違反 |
| 5 | 成果物の保存/結果確定失敗（publish-result）。Release公開を意味しない |
| 130 | cancel |

JSON対応操作の `--json` はversion付きの一objectだけをstdoutに出し、生logと診断はstderrへ送ります。成功・失敗とも同じenvelopeでcommand、status、dataまたはerrorを識別できるようにし、version fieldを含む具体schemaはR1で固定します。JSON出力不能も成功にしません。途中のprogressをstdoutに混ぜたり、失敗を空objectとexit 0で表したりしません。

## R1の具体契約

`--json` はglobalでsubcommandの前後に置けます。`--` 以後は位置引数です。引数解析失敗・未知option・未知command・不正 `--json=...` でもJSON要求を先に検出し、[result schema 1](../../schemas/result-v1.schema.json)の一objectだけをstdoutへ返します。`--help` / `--version` はexit 0の通常テキスト表示を許す例外です。

`--json` なしでは人間向けの日本語要約を返します。成功はstdoutへ、失敗はコード・exit code・診断本文・metadata位置（ある場合）をstderrへ出します。主要な入力・template/lock不整合には確認先と明示的な復旧操作を添え、自動修復はしません。通常表示の文言は機械契約ではなく、parseする利用者は `--json` を指定してください。両表示で終了コードと検証処理は共通です。出力先への書込み失敗はexit 5です。

通常のplan要約はTarget/Document、template ID/version・内蔵IDまたはProject内path・digest、順序付き本文、bibliography/resource/metadata参照、出力先、未確認の必要toolを表示します。全fieldと結合metadataは `plan --json` で返します。静的操作だけではruntime未確認・PDF生成未実装・`build_ready=false` であることを成功表示にも明示します。

envelopeは `schema_version: 1`、`command`、`status: success|error`、`message`、`data`、`error`、`unverified` を常に持ちます。成功時はdata objectとerror=null、失敗時はdata=nullとerror objectです。errorは安定 `code`、`message`、`exit_code`、`instance_path`（metadata schema違反以外はnull）を持ちます。引数解析時commandは `arguments`。stdoutへの保存失敗はexit 5で、出力先自体が壊れている場合の完全JSONは保証できません。

| 主なcode | exit | 意味 |
| --- | --- | --- |
| invalid_arguments / invalid_toml / invalid_metadata / invalid_json / invalid_lock | 2 | 引数または構文不正 |
| unsupported_schema / unsupported_format / unsupported_recipe / tool_mismatch | 2 | R1対応外または宣言矛盾 |
| unknown_target / unknown_document / invalid_template_ref / invalid_id | 2 | 参照・識別子不正 |
| empty_sources / missing_input / invalid_input / project_not_found / filesystem | 2 | 入力欠損・種別・読込み失敗 |
| metadata_schema / invalid_schema / external_schema_ref / unsupported_schema_keyword | 2 | metadata/schema違反 |
| parse_budget / schema_budget / filesystem_budget / invalid_utf8 / invalid_template | 2 | 予算・表現・package不正 |
| missing_dependency / stale_lock | 3 | template/lock不足・不一致 |
| unsafe_path / init_collision | 4 | path・所有権・既存directoryの安全拒否 |
| save_failed / cleanup_failed / serialization | 5 | 保存・作成物cleanup・出力失敗 |

initは親directoryが存在する新規directory、または空directoryだけを受理します。`content/report.md`、`metadata/common.yaml`、`references.bib`、`figures/`、`pdx.toml`、`pdx.lock`、`.gitignore` を作成します。既存entryが一つでもあれば拒否します。create-newのguardと各fileの排他作成で通常の同時initによる上書きを防ぎ、途中失敗では作成済みfileと空directoryだけを戻します。既存directory自体を消しません。cleanup失敗は残存pathとexit 5を返します。別userによる敵対的path差替え、電源断の完全transaction保証は含みません。

`check` / `plan` は書込みゼロ。`lock` は明示したlock更新だけを行います。全操作が未解決runtimeと未検証のPDF/書式を結果へ示し、静的successをbuild-readyとは表示しません。

cleanはProjectの本文・図・metadataやユーザー所有PDFを消しません。削除候補は所有記録と正規化済みpathで確定し、危険リンク・所有不明・領域外は拒否します。更新/削除による原稿保全は[構成](../architecture/system.md)の領域境界に従います。
