# Pandocker-X vNext

ユーザー所有の原稿を、選択したテンプレートと固定した組版環境でPDF化するローカルアプリを新規開発します。CLI-firstで、利用者の入口は `pdx` とします。

## 現在の状態

**R1はRust共通層とDocker不要の `init` / `check` / `plan`、ローカル依存の明示 `lock` を実装しています。PDF組版はR2です。** 組版image、fetch/build/doctor/clean、インストーラ、Windows配布受入は未実装です。静的成功でも `static_only=true`、`runtime_verified=false`、`build_ready=false` を返します。検証結果と未実施範囲は[R1報告](docs/reports/r1-static-cli.md)を参照してください。

2026-10-15承認の非互換な再開発です。旧実装・仕様・CLI互換性は継承しません。旧版は `archive/pre-rebuild-2026-10-15`、SHA `4fac1495e5418c57bb23e74509091ae333a5e7f4` に保全し、Git履歴と[LICENSE](LICENSE)を保持します。旧コードを新版treeへ移植せず、`legacy/` も残しません。保全の確認結果は[R0報告](docs/reports/r0-foundation.md)に記録します。

## R1の使い方

Rust 1.99.0とMSVCまたはLinuxのlinkerが必要です。`Cargo.lock` を保持し、開発時の依存取得と製品のoffline実行を区別します。

```text
cargo build -p pdx --locked
pdx init "原稿 path" --template default-report
pdx check "原稿 path" --json
pdx plan "原稿 path" --target report --json
```

`pdx` は生成された `target/debug/pdx`（Windowsは `pdx.exe`）を指します。バイナリ単体をcheckout外へ移しても内蔵templateを使えます。initは親directoryが存在する新規/空directoryに限定します。原稿directory内ではpathを省略でき、最も近い祖先の `pdx.toml` を使います。既存原稿へ導入する場合は新規directoryで雛形を確認してから手動で設定してください。

`check` / `plan` は全Targetを検証し、ファイルを変更しません。ローカルtemplate編集・Target依存変更後だけ `pdx lock "原稿 path" --json` で依存を明示更新します。lockは取得を行わず、未解決runtimeを解決済みにしません。[CLI契約](docs/contracts/cli.md)、[Project契約](docs/contracts/project.md)、[template契約](docs/contracts/template.md)に制限と失敗分類を定義します。

開発版をユーザー所有の専用directoryへ配置する場合は、checkoutで次を実行できます。Rustとlinkerが必要なsource installであり、R4の配布installer・更新機構ではありません。

```text
cargo install --path apps/pdx --locked --root "<ユーザー所有の専用app directory>"
```

配置先はそのdirectoryの `bin/pdx`（Windowsは `bin/pdx.exe`）です。PATHを変更する前でも実行ファイルの絶対pathで起動できます。原稿は配置先の外に作成し、更新時も `pdx.toml` と `pdx.lock` を原稿とともに保持してください。template依存はlockで固定しますが、runtimeは未解決で、PDF再現性はまだ検証していません。

通常表示は日本語の要約と次の操作です。`plan` で選択Target・templateの由来/digest・入力順・出力先を確認できます。失敗はstderrへコード・metadata位置・復旧案を出し、非0で終了します。自動処理には `--json` を使ってください。全fieldと結合metadataを含む版付きJSON一objectをstdoutへ返します。templateの切替手順は[Project契約の例](docs/contracts/project.md#templateを切り替える例)を参照してください。`build` はまだ利用できません。

## 読み始める場所

- [文書正本マップ](docs/README.md): 主題ごとの唯一の正本
- [機能要件](docs/requirements/functional.md)と[非機能要件](docs/requirements/non-functional.md): 受入条件と検証への追跡
- [構成と責務](docs/architecture/system.md): Rust共通コア、application API、Dockerと領域分離
- [実装契約の入口](docs/contracts/README.md): 設定・CLI・テンプレート・build
- [実装段階](docs/operations/rounds.md): R0からR5への進め方
- [貢献方法](CONTRIBUTING.md)と[agent入口](AGENTS.md)

初期対象はWindows 11 x64 + Docker DesktopのLinux containers、およびLinux x86_64 + Docker Engineです。WSLではLinux版pdxを使う設計です。Dockerによる組版を正とし、host TeXへfallbackしません。GUI、旧版互換、未知ZIPの自動適合は今回の対象外です。
