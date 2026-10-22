---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/contracts/template.md
---

# TemplatePackage契約

R0には検証済みvNext templateはありません。旧templateを移植せず、以下の契約に従ってR1以降に作成します。

packageは `template.toml` と `metadata.schema.json` を入口とし、ID/version、入力方式、必要toolとtoolchain条件、資産、metadata defaults、出力条件、許諾、出典、sample、検証方法を宣言します。未知キー・不正型・重複・危険path・不足資産を拒否します。含有フォント・wrapper・class・画像など個別資産の許諾と出典も追跡します。

実証は次の順です。

1. Markdown → standalone LaTeX → PDF。
2. Markdown fragment → 外部wrapper → PDF。
3. direct LaTeX → PDF。

これらの経路を同じProject/Document/Targetとapplication APIで扱います。共通preamble、citeproc、lualatexは強制しません。templateが要求する `lualatex`、`platex`、`uplatex`、`pbibtex`、`dvipdfmx`、`latexmk` 等は採用したimage内でprobeし、宣言だけで利用可能とみなしません。

外部wrapperも検証されたpackage資産としてsnapshotに含めます。未信頼ZIPを自動的に既知templateへ適合させたり、任意host shell commandを実行させたりしません。信頼済みtemplateでも[buildの隔離境界](build.md)を解除しません。template編集後は[lockの明示更新](project.md)が必要です。
