---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/contracts/project.md
---

# Project・設定・依存固定契約

R1の静的契約です。構文は[Project schema 1](../../schemas/project-v1.schema.json)、[lock schema 1](../../schemas/lock-v1.schema.json)、[BuildPlan schema 1](../../schemas/plan-v1.schema.json)とRustの型に対応します。JSON schemaはTOMLをJSONの型へ写したfield契約であり、参照・path・所有権の意味検証はcore/adapterが行います。[横断テスト](../../tests/contracts.rs)で両者の整合を確認します。

`pdx.toml` は `schema_version=1`、`default_target`、`documents`、`targets` を必須とします。Documentは `format`（R1は `markdown` のみ）、非空の `sources`、`bibliography`、`resources` を必須とし、`metadata` は任意のYAML pathです。Targetは `document`、`template`（`id` / `path` のどちらか一方）、`output_dir` を必須とし、`metadata` は任意です。IDは1〜80文字のASCII英数字・`-`・`_`。IDを実pathへ暗黙変換しません。未知field、重複key、型不正はTOML parserで拒否します。

## モデル

| 型 | 意味 |
| --- | --- |
| Project | ProjectRoot、Documents、Targets、default_targetを持つ |
| Document | 順序付きsources、文書metadata、bibliography、resourcesを持つ論理文書 |
| Target | Document参照、TemplatePackage参照、metadata override、output_dirを持つ一つの書式出力 |
| TemplatePackage | 宣言・schema・資産・tool要求を持つ組版package |
| Toolchain | 固定image digestと必要tool/環境の依存 |
| BuildPlan | 検証済み入力・順序・依存・Target・出力・実行条件の静的計画 |
| BuildRecord | build-id、実入力digest、依存、実行条件、結果、成果物の観測記録 |

同じDocumentを卒論書式と提出用別書式のTargetへ割り当てることと、別Documentを持つことを区別します。ID重複、存在しないDocument/template/default_target参照、衝突する出力先は拒否します。target省略時はdefault_targetを解決し、明示指定やall-targetsとの曖昧さを残しません。

## 設定とpath

Project設定は `pdx.toml`、文書metadataはYAMLです。本文・図・bibliography・metadataはユーザー所有です。sourcesは宣言された順序を保持し、暗黙のdirectory走査順で組版しません。

Project設定内pathはProjectRoot相対、区切りは `/` のみです。空component、`.` / `..`、先頭slash、backslash、drive/UNC/ADS、制御文字、`<>:"|?*`、Windows予約名（拡張子付き・COM/LPTの上付き数字を含む）、末尾dot/space、8.3別名の曖昧さを持つ `~` を拒否します。Unicode uppercaseで比較した同一/親子pathを検査し、component単位の大小文字aliasも拒否します。実FSで要求と異なるcaseの既存entry、symlink、junction、全reparse point、通常file/directory以外の入力を拒否します。ProjectRootの祖先もリンク検査します。R1は安全な内部リンクも許可しません。

出力は全Targetで同一/親子を禁止し、全Documentのsource、metadata、bibliography、resourceと全Targetのmetadata、ローカルtemplate directory、`pdx.toml` / `pdx.lock` / `.git` / `.pdx` とも同一/親子を禁止します。既存出力の祖先はdirectoryである必要があります。resource directory・既存出力の子孫も危険path/リンクを検査します。走査は10000 entries、深さ64 component以内です。原稿内容のparser・外部参照解析・敵対的な別userとのTOCTOU完全防御はR1の保証外です。R2の入力copy・確定では境界を再確認する必要があります。

ProjectRoot自体をrepository配下やASCII pathへ制限しません。明示pathはそのdirectoryだけを使い、設定がなければ親へfallbackしません。省略時だけ最も近い `pdx.toml` の親を探索します。ローカルtemplateはProject内directoryに限定し、Dataへの登録/import/addは未実装です。

## templateを切り替える例

`--target` は既存Targetを選ぶ操作で、template自体は `pdx.toml` のTargetへ宣言します。内蔵は `template = { id = "default-report" }`、Project内packageは `template = { path = "templates/custom" }` です。`id` と `path` は同時指定できません。内蔵IDの一覧取得・外部package取得はR1にありません。

試す場合は新規Projectへ、開発checkoutの `templates/default-report/` 全体を `templates/custom/` としてコピーします。manifestのid/versionやmetadata defaults/schemaを編集し、既存の `[targets.report]` のtemplate行を上記path参照へ置き換えます。全資産が宣言と一致するpackageだけを使います。schemaとmetadataが適合することを確認してから次を実行します。

```text
pdx lock "原稿 path"
pdx check "原稿 path"
pdx plan "原稿 path" --target report
```

同じDocumentで二つの書式を残す場合は、既存Targetを保持したまま次を追加できます。出力先は別にします。

```toml
[targets.custom]
document = "report"
template = { path = "templates/custom" }
output_dir = "output/custom"
```

Targetを追加した後も明示 `lock` が必要です。`pdx plan "原稿 path" --target custom` で選択元・digest・入力と出力を比較します。既定を変える場合はrootの `default_target` を `"custom"` に変更します。依存変更はlock不一致として検出し、本文だけの編集ではlockを更新しません。`check` / `plan` は全Targetを検証するため、選択外Targetのエラーも先に修正してください。いずれもPDF生成・書式適合の実証ではありません。

## metadata

結合順はtemplate defaults → Document → Targetです。mapは再帰的にキー結合、arrayは全体置換、scalarは後段が置換します。各入力内の重複キーは結合前に拒否し、結合結果をtemplateのmetadata schemaで検証します。未知キー・不正型・schema違反・path用途の危険値を拒否します。YAMLの解釈と境界例はR1のfixtureで固定し、独自parserは作りません。

user全体設定でフォント・余白など文書の見た目を変更しません。現在日付を暗黙補完せず、必要な日付は文書側に明示します。

R1のYAMLはJSON互換のmappingのみです。serde-saphyrで重複key、複数document（末尾の空documentも含む）、非文字列map key、非有限数、未知タグ/`!include`、anchor/alias、merge keyを拒否します。include filesystem featureは組み込みません。strict booleanを使い、arrayは再帰結合せず全置換します。YAMLは各1MiB、深さ32、10000 nodes、40000 events、scalar合計1MiB、document数1、anchor/alias/merge key数0が上限です。TOMLとlock JSONも各1MiB以内。schemaの予算は[template契約](template.md)へ定義します。

## pdx.lock

最小R2でlocal runtimeを明示固定した場合はlock/plan schema 2を使います。旧schema 1は引き続き未解決runtimeとして受理します。具体的な形と保持・更新境界は[最小R2契約](runtime-r2.md)へ定義します。以下はR1 schema 1の契約です。

`pdx.lock` はJSONです。`schema_version: 1`、Target IDごとの `templates`（`id` / `version` / `digest`）、`runtime: "unresolved"` を必須とします。未知field・重複key・型不正は拒否します。R2のimageがまだ存在しないため、image名、ゼロdigest、架空registryを記録しません。Target集合・template identity/digestが現状と一致しなければ `stale_lock`、不足なら依存不足です。本文・図・文書metadataの変更ではlock更新不要です。これらの実入力digestを残すBuildRecordはR2です。

template編集は依存変更であり、明示的なlock更新を要します。lock不一致・未固定・不足依存をbuildが自動修復してはいけません。lockは依存を解決・固定する操作、fetchは固定依存を取得・検証する操作です。取得を伴う操作はCLIで明示し、init/check/planはローカルで準備済みの宣言だけを使います。

R1の `lock [path]` は全入力・metadata・template・出力所有権を静的検証した後、ローカル依存だけを固定します。旧lockの不一致は明示更新時だけ許可します。排他的に作成した同一directory内一時fileへ書いてrenameし、失敗時は今回の一時fileだけを削除します。runtimeは未解決のままです。`check` / `plan` は書込みゼロで、`lock` を暗黙実行しません。

## BuildPlan

選択Targetのrecipe、順序付き本文、bibliography/resource、metadata入力pathと結合結果、template source/identity/digest/宣言file、出力、必要tool、runtime resolutionを型で返します。全Targetのcheckを通してから選択し、未指定時は同じ設定snapshotのdefault_targetを使います。絶対path・mtime・現在日付を計画に混ぜず、同じ宣言から決定的なJSON dataを得ます。shell command文字列は作りません。

`schema_version=1`、`static_only=true`、`runtime_verified=false`、`build_ready=false` と未検証項目を持ちます。R1成功は静的整合であり、Docker、tool存在、PDF、書式適合の証拠ではありません。
