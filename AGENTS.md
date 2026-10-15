# Agent作業の入口

対象は `Xpotato1024/Pandocker-X`。新版は旧版と非互換の新規開発です。

## 作業前に読むもの

`AGENTS.md`、対象Issue、[文書正本マップ](docs/README.md)、関係する契約・実装・テストを読みます。旧版のコード・CLI・Issue・過去報告を現在仕様として扱いません。

## 不変条件

- Rust coreは純粋なモデル・検証・計画を所有し、FS・環境変数・process・CLI出力を行いません。
- CLIと将来GUIは同じapplication APIを使い、Justへ製品ロジックを複製しません。
- アプリの配置場所と原稿のProjectRootを分離します。既存原稿・ユーザー設定・前回成果物を勝手に変更・削除しません。
- 通常組版はDocker内で完結し、host TeXや旧WSLミラーへ暗黙fallbackしません。
- 静的検証、compile、実PDF生成、組版適合、配布受入を区別します。未実装・未検証を成功と表示しません。

## 作業・検証・権限

- remote、default branch、作業branch、HEAD、working treeを確認します。default branchはGitHub metadataを正とし、現在は `master` です。
- 1 Issue = 原則1 PR。taskで承認されたRound／scopeを守り、無関係な変更・公開契約の無承認拡張では停止します。
- default branchへ直接commitせず、`codex/<issue>-<description>`で作業します。依存PRはbaseとmerge順を明示します。
- merge、tag、Release/image公開、credentials、権限設定、破壊的cleanupは別の明示許可が必要です。
- modelは実行環境とユーザーの明示指定に委ねます。原稿保全・path・隔離・更新削除など高リスク境界は独立レビューを行います。
- 要件・契約・挙動が変わるときは、同じ変更で正本文書と必要テストを同期します。検証を削除・skip・弱体化して合格させません。
- 人間向け文書は日本語UTF-8・LF。1主題1正本を守り、Accepted ADRの判断変更は新ADRでsupersedeします。
- 一時物はtask所有の明示された領域に隔離します。ユーザーの既存untracked/ignoredを削除せず、意図した残存物と未実施検証を報告します。

詳細は[開発運用](docs/operations/development.md)と[実装段階](docs/operations/rounds.md)を参照してください。
