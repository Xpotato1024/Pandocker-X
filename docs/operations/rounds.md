---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/operations/rounds.md
---

# 実装段階

2026-10-22の明示承認により、PR21上の独立stackで最小R2の初回PDF経路を実装します。先行PR19/20/21は統合せず、fetch/doctor/clean・公開は範囲外です。[最小R2契約](../contracts/runtime-r2.md)に今回の範囲を定義します。以下のR1限定は当初Issue #18の境界です。

Phaseは大きな到達点、Roundは一回の実行単位、Issueは一つのPRへ収める変更単位です。1 Issue = 1 PRを維持し、各Roundの受入条件を確認してから次を別途開始します。Issue #18の範囲はR1のみです。R0の履歴は[R0報告](../reports/r0-foundation.md)、R1の観測は[R1報告](../reports/r1-static-cli.md)に分けます。

| Round | 対象 | 受入証拠 |
| --- | --- | --- |
| R0 基盤 | 旧版保全の照合、旧tree撤去、仕様・文書・運用基盤 | archive SHA、LICENSE、inventory、文書検証、自己review、親の独立確認 |
| R1 Issue #18 | Rust workspace、init/check/plan、ローカルlock、設定/lock/CLI/template contracts、横断tests | Docker/Git/network不要の正常/異常系、機械schema、FR/NFR追跡、親の独立review |
| R2 | runtime、fetch/doctor/build、clean、準備後offline | 固定imageの実probe、実PDF、隔離・中断・保存失敗・古いPDF等の異常系 |
| R3 | 実卒論と公式学会templateの適合検証 | 3入力経路、実原稿による検証（別途許可）、許諾・出典・tool証拠 |
| R4 | user導入・更新・削除と配布整合 | 対象OSでのlifecycle、原稿保全、checksum/許諾 |
| R5 | 公開判定 | 全必須要件の証拠、残risk、独立reviewと公開判断。公開操作は別許可 |

R1ではDockerfile、組版image、公開workflow、fetch/build/doctor/cleanのstubを追加しません。GUI、旧版互換、未知ZIPの自動適合、旧Issue closeは対象外です。watch/source exportは後続で別設計します。R5という段階名だけでmerge/tag/Release/image公開を許可しません。
