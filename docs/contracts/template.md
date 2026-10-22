---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/contracts/template.md
---

# TemplatePackage契約

R1は内蔵 `default-report` とProject内ローカルpackageの静的解決を実装します。旧templateは移植していません。内蔵fileはバイナリに組み込み、checkout、InstallRoot、current directoryを探索しません。両者を同じ `TemplatePackage` 型・validator・digestへ解決します。実PDF・組版適合は未検証です。

packageは `template.toml` と `metadata.schema.json` を入口とし、ID/version、入力方式、必要toolとtoolchain条件、資産、metadata defaults、出力条件、許諾、出典、sample、検証方法を宣言します。未知キー・不正型・重複・危険path・不足資産を拒否します。含有フォント・wrapper・class・画像など個別資産の許諾と出典も追跡します。

R1のfieldは[manifest schema 1](../../schemas/template-v1.schema.json)です。`schema_version=1`、`id`、`version`、`recipe`、`engine`、`required_tools`、`metadata_schema`、`defaults`、`resources`、`license`、`provenance`、`sample` を必須とします。pathはpackage root相対で[Projectの可搬path規則](project.md)に従い、各roleは異なるfileを参照します。packageはmanifest、schema、defaults YAML、sample、明示resourceの正確なfile集合から成り、未宣言file、危険リンク、欠損を拒否します。最大256 files、個別16MiB、合計32MiBです。許諾/出典の非空宣言と資産存在を確認し、法的適合や公式書式の保証とは区別します。

R1で対応するrecipeは `markdown-standalone` のみです。入力はMarkdown、将来の出力経路はstandalone LaTeX→PDF、engineは `lualatex`、必要toolsは重複なしの `pandoc` / `lualatex` と固定します。未知recipe、format、engine/tool矛盾を拒否します。任意argv・shell命令はmanifestに含めません。fragment/wrapper、direct LaTeX、別engineは後続Roundで契約と実証を追加します。特定学会名によるRust側の分岐は不要です。

## metadata schemaとdigest

標準 `jsonschema` validatorを `default-features=false` で使い、HTTP/file取得を組み込みません。schema自身をdraft-07 meta-schemaで検証してから結合metadataを検証します。`$schema` は `http://json-schema.org/draft-07/schema#` を明示します。schema JSONは重複keyを拒否し、最大64KiB、深さ24、参照展開を含め4096 nodesです。外部 `$ref`、`$id`、dynamic/recursive ref、再帰参照、regexの `pattern` / `patternProperties` はR1では拒否します。同一document内 `#/...` JSON pointerによる非再帰refは許可します。metadata違反はinstance pathとvalidator診断を返します。schemaのdefaults keywordで文書値を暗黙補完せず、defaults YAMLから明示結合します。

禁止keywordはschema位置だけで判定します。`properties` / `definitions` / `dependencies` のユーザー定義名、`const` / `enum` / `default` / `examples` のデータ内で同じ文字列を使うことは許可します。`$ref` の参照先はschemaとして再検証し、禁止keywordや外部・再帰参照を隠せません。データ部分も深さ・node予算へ含めます。

参照fragmentは標準resolverと同じくpercent-encodingをUTF-8へ復号してからJSON Pointerを解決し、復号後の参照で循環を検出します。各schema位置の `$schema` も上記draft-07宣言に限定し、入れ子で別draftへ切り替えて走査対象keywordを変えることを拒否します。literalデータ内の同名keyはschemaとして参照された場合だけこの制限を受けます。

digestは `sha256:` + 64桁hexです。危険path/リンクをhash前に拒否し、関連fileを相対pathのUTF-8 byte順でsortします。SHA-256入力は `pdx-template-v1\0`、各fileのpath byte長（u64 big-endian）、path bytes、内容byte長（同）、実bytesの順です。manifest/schema/defaults/sample/license等の宣言資産を含み、mtime・絶対path・本文・文書metadataには依存しません。変更時は[明示lock](project.md)が必要です。

実証は次の順です。

1. Markdown → standalone LaTeX → PDF。
2. Markdown fragment → 外部wrapper → PDF。
3. direct LaTeX → PDF。

これらの経路を同じProject/Document/Targetとapplication APIで扱います。共通preamble、citeproc、lualatexは強制しません。templateが要求する `lualatex`、`platex`、`uplatex`、`pbibtex`、`dvipdfmx`、`latexmk` 等は採用したimage内でprobeし、宣言だけで利用可能とみなしません。

外部wrapperも検証されたpackage資産としてsnapshotに含めます。未信頼ZIPを自動的に既知templateへ適合させたり、任意host shell commandを実行させたりしません。信頼済みtemplateでも[buildの隔離境界](build.md)を解除しません。template編集後は[lockの明示更新](project.md)が必要です。
