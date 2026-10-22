---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/operations/development.md
---

# 開発・検証・Git運用

## 開始と作業範囲

対象repository、remote、default branch、branch、HEAD、working treeを確認する。GitHub CLIが使えない場合は接続済みGitHub連携で同じ事実を確認し、認証情報を探したり設定を変更したりしない。

1 Issue = 原則1 PR。承認されたRoundだけを担当し、scope外の契約変更、無関係な変更、必要な検証環境の不足は報告する。default branchへ直接commitしない。通常のPR baseは確認したdefault branch。依存する未merge PRをbaseにする場合は、依存関係、レビュー差分、merge順、retarget条件を明記する。

merge、tag、Release/image公開、credentials、権限設定、破壊的cleanupは明示許可なしに行わない。環境が保護する操作は権限を変えず報告する。task内の役割分担は個別指示に従い、特定sessionの制約を恒常ルールへ転記しない。

## 検証と証拠

変更した契約に必要なunit、architecture、contract、integration、実PDF、配布受入を選ぶ。compile成功をPDF検証と呼ばない。同じ条件の既存証拠は再利用可能だが、SHA・依存・環境・実行範囲を記録する。テストを削除・skip・弱体化して合格させない。

差分はgit diff --check、working treeと未追跡ファイルを含めて確認する。新規文書のUTF-8と内部リンク、現在能力と未実装の区別も確認する。一時物はtask所有の明示領域に隔離し、既存原稿やuntracked/ignoredをcleanupしない。

## 文書とレビュー

R1の必須gateは次の通りです。Rustは `rust-toolchain.toml`、依存は `Cargo.lock` を使用します。実行前にtask専用の絶対temp rootを一つ用意し、`CARGO_TARGET_DIR`、必要なら `CARGO_HOME`、OS temp変数、`PDX_TEST_TEMP` をその配下へ向けます。各環境のcompiler/linker bootstrapは呼出し元が担当し、製品へscriptロジックを複製しません。

```text
cargo fmt --all --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
git diff --check
```

`tests` はworkspace memberです。`architecture` / `contracts` / `cli` は独立Cargo test target、coreのunitはcrate内へ置きます。workspace全体のall-targets testではCLI packageのintegration targetが実binaryを用意し、横断CLI testがそのbinaryをcheckout外へcopyして起動します。checkだけでCLI実行を証明しません。Windowsは権限不要のjunction、Linuxはsymlinkの試験を行い、OSごとの未実施を報告します。Justはcheck/testへの薄いCargo呼出しだけです。

1主題1正本。現在要件・契約は正本マップから辿る。Accepted ADRは理由を保存し、判断変更は新ADRでsupersedeする。reportsは観測証拠であって現在仕様ではない。

modelは実行環境とユーザー指定に従い、旧model名を固定しない。原稿保全、path、runtime隔離、成果物確定、更新削除など高リスク境界は独立read-onlyレビューを行う。レビューは対象snapshotを固定し、根拠・影響・severityを記録する。自己検証を独立レビューと呼ばない。

## PRと完了報告

日本語で目的、Closes、変更範囲、意図的破壊変更、検証commandと結果、Not Run理由、runtime/公開への影響、残るrisk、次工程を記載する。head/base/実差分と報告を一致させる。stacked PRは先行PRのmerge後にbaseと差分を再確認する。公開・mergeは実装完了と別の権限である。
