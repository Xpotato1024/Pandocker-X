---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/contracts/README.md
---

# 実装契約の入口

このディレクトリは実装が守る振る舞いの正本です。ADRは採用理由、requirementsは受入要求、reportsは実測を扱います。R1はProject/CLI/templateの静的契約とversion付き機械schemaを提供し、build実行はR2以降です。

| 契約 | 内容 | 次の具体化 |
| --- | --- | --- |
| [Project・設定](project.md) | domain model、metadata、path、lock | R1実装: pdx.toml/lock/BuildPlanのversion付きschemaと正常/異常テスト |
| [CLI](cli.md) | コマンド、副作用、終了コード、JSON | R1実装: parser、JSON schema、実CLIテスト |
| [テンプレート](template.md) | package宣言、tool、許諾、実証経路 | R1静的実装: manifestとmetadata schema検証。R3: 実template適合 |
| [build](build.md) | snapshot、隔離、確定、復旧 | R2: BuildRecord schema、Docker実行・実PDF・故障注入 |

R1のschema_versionは1に固定し、未知versionと未知fieldを拒否します。追加の公開field/recipe/形式は契約・validator・テストを同時更新します。未定義fieldを既に公開した安定APIとは扱いません。文章契約との矛盾を実装都合で黙って緩めません。
