---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/README.md
---

# 文書正本マップ

各主題の規範は以下の一箇所に置きます。他文書はリンクで参照し、同じ仕様をコピーしません。R1の静的操作の具体契約と、R2以降の設計要求を区別します。実測・実行証拠はreportsで区別します。

| 主題 | 正本 | 内容 |
| --- | --- | --- |
| 製品入口・現在能力 | [README](../README.md) | R1の使い方、未実装範囲、旧版への参照 |
| 機能要件 | [functional](requirements/functional.md) | FRと受入シナリオ |
| 品質・安全・性能要件 | [non-functional](requirements/non-functional.md) | NFRと未実測予算 |
| 構成・依存・領域 | [system](architecture/system.md) | 配置、責務、OS方針 |
| 実装契約の案内 | [contracts](contracts/README.md) | 各契約の境界と具体化時期 |
| Project・設定・lock | [project](contracts/project.md) | モデル、path、metadata結合 |
| テンプレート | [template](contracts/template.md) | package、toolchain、実証順 |
| CLI・終了コード・JSON | [cli](contracts/cli.md) | コマンドの効果と失敗 |
| build・隔離・成果物 | [build](contracts/build.md) | snapshotから確定・cleanupまで |
| 採用判断 | [ADR一覧](decisions/README.md) | 背景、採用理由、帰結 |
| 開発・Git・review・検証 | [development](operations/development.md) | 作業gateと証拠 |
| 実装段階 | [rounds](operations/rounds.md) | R0〜R5、停止境界 |

実装pathは[構成](architecture/system.md)、要件からテストへの対応は要件表を正本とします。未作成の将来pathを実在リンクとして扱いません。reportsは観測結果であり、契約を上書きしません。

過去の実施証拠: [R0報告](reports/r0-foundation.md)。この報告は現在仕様の正本ではありません。

R1の実装・検証範囲: [R1報告](reports/r1-static-cli.md)。fieldの機械契約は各contractから `schemas/` へ参照します。

R1のtemplate検証修正・通常CLI表示・source install・静的時間観測: [R1追補](reports/r1-usability.md)。
