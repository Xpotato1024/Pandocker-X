---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/architecture/system.md
---

# 構成と責務

## 製品境界

CLI-firstのローカルアプリとし、後続GUIは同じapplication APIを呼びます。常駐HTTP、DB、汎用DAG、Compose、独自TeX/Markdownパーサーは導入しません。文書形式の処理は既存toolへ委譲し、共通preambleや特定engineを全テンプレートへ強制しません。

## 配置（R1の静的workspace）

| path | 責務 |
| --- | --- |
| `apps/pdx` | CLI引数、表示、終了コード。製品ロジックを持たない |
| `crates/pdx-core` | 純粋な型・検証・plan。FS、環境変数、外部process、CLI出力を行わない |
| `crates/pdx-application` | use case、port、処理順、application API |
| `crates/pdx-adapters` | R1はFS/内蔵templateのport実装。Docker adapterはR2 |
| `runtime` | R2以降の計画。固定する組版環境の定義・検証（未作成） |
| `templates` | 検証済みTemplatePackageと資産・許諾 |
| `packaging` | R4以降の計画。OSごとの配布・user lifecycle（未作成） |
| `schemas` | version付き機械可読契約 |
| `tests/Cargo.toml` | 横断テスト用crate。root Cargo workspaceのmemberとする |

root `Cargo.toml` に `apps/pdx`、3つのcrate、`tests` をworkspace memberとして明示しています。coreのcrate-local unit、横断contract/FS integration、実CLI、architecture/docsのCargo targetを分離します。CLI packageのbinary登録testにより横断テストから本物のpdx executableを起動します。

applicationの `ProjectStore` はProjectRoot探索、bounded read、入力/出力検査、template解決、排他的init、明示lock保存を表し、`Application` がcheck/plan/init/lockの処理順を所有します。adapterは各portのFS差を実装し、coreはbytes/値からの構文・path・metadata/schema・digest・planだけを扱います。製品コードは外部processを起動しません。Windows junction作成は隔離されたテストharnessだけです。

依存方向はCLI → application → core。adaptersはapplicationのportを実装し、coreへ副作用を持ち込みません。組立てはCLIの起動層で行い、CLIと後続GUIは同一use caseを使います。applicationはadapterの具体型に依存しません。[開発用Justの判断](../decisions/0003-thin-just.md)もこの方向を守ります。

## 領域と所有

| 領域 | 内容・所有 |
| --- | --- |
| Install | 配布アプリ。更新/削除できる領域 |
| Config | user設定。原稿・文書metadataを置かない |
| Data | 取得済みtemplate/toolchainの永続依存 |
| Cache | 再取得・再生成可能なアプリ所有物 |
| State | 実行状態、排他、履歴・BuildRecord等 |
| Project | ユーザー所有の設定・本文・図・metadata。lifecycleで触らない |
| BuildRoot | buildごとの隔離snapshot・一時生成物 |

ProjectRootはrepository配置やInstallから独立した任意ローカルpath（日本語・空白を含む）です。設定中pathの基準と安全検証は[Project契約](../contracts/project.md)、入力copyと成果物の所有は[build契約](../contracts/build.md)に従います。OS別の具体的ディレクトリ名や契約schemaはR1/R4で定めます。領域の別名・リンクによる重なりを許して原稿削除の境界を崩してはいけません。

## runtimeとOS

初期対象はWindows 11 x64 + Docker Desktop Linux containers、Linux x86_64 + Docker Engineです。WSL内ではLinux版pdxを使います。独自WSL mirror、remote daemon、macOSは初期対象外です。Windows nativeからはDocker Desktopを利用し、host TeX fallbackはしません。

通常利用はdigest固定の配布imageを明示fetchして準備します。R0に公開済みvNext imageはなく、取得できるとは案内しません。開発用imageのbuildと配布imageの利用・公開を区別します。templateが要求するtoolと、実imageでprobe済みのtoolは別の証拠です。
