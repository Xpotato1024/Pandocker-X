---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/contracts/project.md
---

# Project・設定・依存固定契約

これは未実装の規範です。構文schemaとfixtureはR1で具体化します。

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

Project設定内pathはProjectRoot相対です。初期版では領域外参照・出力、絶対path、traversal、symlink/junction等による危険リンクを拒否します。OSの正規化・大小文字・別名も考慮し、検証後の入力copyや成果物確定時に境界を再確認します。ProjectRoot自体をrepository配下やASCII pathへ制限しません。ローカルtemplate pathもProject内に配置するか、明示addで検証されたData内のID参照に変換し、外部pathをbuild時に直接渡しません。

## metadata

結合順はtemplate defaults → Document → Targetです。mapは再帰的にキー結合、arrayは全体置換、scalarは後段が置換します。各入力内の重複キーは結合前に拒否し、結合結果をtemplateのmetadata schemaで検証します。未知キー・不正型・schema違反・path用途の危険値を拒否します。YAMLの解釈と境界例はR1のfixtureで固定し、独自parserは作りません。

user全体設定でフォント・余白など文書の見た目を変更しません。現在日付を暗黙補完せず、必要な日付は文書側に明示します。

## pdx.lock

`pdx.lock` はtemplate/toolchain依存だけを固定します。templateの同一性と内容digest、toolchainのimage digest等を記録し、詳細schemaはR1で定めます。本文・図・文書metadataの変更ではlock更新不要です。実際に用いたこれらの入力digestはBuildRecordへ記録します。

template編集は依存変更であり、明示的なlock更新を要します。lock不一致・未固定・不足依存をbuildが自動修復してはいけません。lockは依存を解決・固定する操作、fetchは固定依存を取得・検証する操作です。取得を伴う操作はCLIで明示し、init/check/planはローカルで準備済みの宣言だけを使います。
