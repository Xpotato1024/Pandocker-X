---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/contracts/README.md
---

# 実装契約の入口

このディレクトリは実装が守る振る舞いの正本です。ADRは採用理由、requirementsは受入要求、reportsは実測を扱います。R0は以下の文章契約のみで、CLI/Rust/schemaを提供しません。

| 契約 | 内容 | 次の具体化 |
| --- | --- | --- |
| [Project・設定](project.md) | domain model、metadata、path、lock | R1: pdx.toml/lock/BuildPlanのversion付きschemaと正常/異常fixture |
| [CLI](cli.md) | コマンド、副作用、終了コード、JSON | R1: parserとJSON schema・エラーfixture |
| [テンプレート](template.md) | package宣言、tool、許諾、実証経路 | R1: template.tomlとmetadata schema検証、R3: 実template |
| [build](build.md) | snapshot、隔離、確定、復旧 | R2: BuildRecord schema、Docker実行・実PDF・故障注入 |

R1以降の実装前に、各version付きschemaのfield名・必須性・互換規則とテストを同じIssue内で確定します。未定義fieldを既に公開した安定APIとは扱いません。文章契約との矛盾は設計変更として明示し、実装都合で黙って緩めません。
