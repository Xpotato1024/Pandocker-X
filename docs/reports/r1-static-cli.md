# R1 静的CLI 実装・検証報告

日付: 2026-10-15。対象: Issue #18、branch `codex/18-static-cli`。基点はR0の `bb7d6a4b526786dfcc21f6b44abf372ed9380de2`。最終の検証対象HEADとCI結果はPRに記録します。

## 実装した範囲

Rustのcore / application / adapters / CLIと、横断テスト用workspace memberを作成しました。`init`、`check`、`plan`、ローカル依存を明示更新する`lock`を実装しています。原稿のパスはアプリの配置と独立し、内蔵テンプレートはバイナリ単体で利用できます。

静的結果は常に `static_only=true`、`runtime_verified=false`、`build_ready=false`、runtimeは `unresolved` です。Docker組版、依存取得、インストーラ、GUIは未実装です。正本は[Project](../contracts/project.md)、[Template](../contracts/template.md)、[CLI](../contracts/cli.md)、[構成](../architecture/system.md)です。

## Windowsでの検証

実装担当の検証とは別に、親の実行環境でも次を確認しました。観測compilerはRust 1.99.0、hostはx86_64-pc-windows-msvcです。

| 検証 | 結果 |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo test --workspace --all-targets --locked` | 29 passed / 0 failed / 0 ignored |
| 空のbuild出力先からの全test | 同じ29件がPASS。既存バイナリ不要 |
| checkout外へコピーしたCLIの独立試験 | 14項目PASS。日本語・空白path、原稿非変更、上書き拒否、lock維持、JSON失敗応答 |

テスト内訳はcore unit 3、FS/contract 17、実CLI 4、architecture/docs 4、実binary登録1です。Windowsのjunction試験も含みます。file symlinkの権限が必要な経路とLinux固有試験は、このローカル実行では確認していません。

## 発見して修正した問題

実CLI試験で、Windowsのcanonical pathに含まれるドライブprefixをルート区切り前に照会して失敗する問題を検出しました。RootDirまで組み立ててから検査するよう修正し、明示path・checkout外実行・junction拒否の回帰を確認しました。planの設定二重読込みも除き、同じ読込snapshotのdefault_targetを使います。

R0のLICENSEに対するテキスト正規化除外を修正しました。LICENSE本文・著作権表示と元のGit blobを維持し、改行だけの差分混入を防ぎます。CIのcheckoutとRust toolchain actionは取得元で確認したcommit SHAへ固定し、checkoutのcredential永続化を無効にしました。

## 独立レビューと再現情報

独立read-onlyレビューには固定snapshot `a5d09aba90415ac71f602b75cc22c18ec6f75e7f` を渡しています。最終レビュー結果、追加修正、最終HEADでのCIはPRへ記録します。自己検証を独立レビューの完了として扱いません。

Rustは `rust-toolchain.toml`、直接依存とfeatureは `Cargo.toml`、推移依存は `Cargo.lock` が正本です。YAMLとJSON schemaは標準parser / validatorを使い、外部取得featureを無効にしています。検証のraw log、失敗時と修正後の結果、実行バイナリhash、workerの元報告はローカルevidenceに保持しています。公開文書へ端末固有pathやログ全文を複製しません。

## 未実施と作業領域

Docker/Pandoc/TeX実行、実PDF、学会・卒論書式の適合、配布・更新削除、性能30回測定はNot Runです。LinuxはCIで別途確認します。tag、merge、Release/image公開、認証・ACL・sandbox設定変更は行っていません。

作業専用toolchain、親のbuild出力とレビュー用checkout、検証evidenceは再確認のため残しています。workerの一時cacheは自動審査が再帰削除を拒否したため残存しており、他の手段で削除を迂回していません。完全なcleanupは未完了です。これらは配布物やGit追跡ファイルに含めず、ユーザーの既存原稿・checkoutを変更していません。

## R2への引継ぎ

次は固定image解決、tool probe、宣言入力のsnapshot、実行隔離、PDF検証、BuildRecord・成果物確定、中断・失敗処理です。R1の静的path検査だけでcopy後の安全を証明したとは扱いません。初期の制限（Markdown、非再帰schema、reparse拒否、新規/空directoryへのinit）は各契約に明記しています。
