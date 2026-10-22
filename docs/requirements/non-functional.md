---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/requirements/non-functional.md
---

# 非機能要件

以下は設計要求です。R0時点で製品実装による保証・性能実測はありません。R1以降、各シナリオを実テストpath・対象revision・OS・runtimeと結び付け、設計上の制約と検証済み範囲を分けます。

| ID | 要求 | 検証シナリオ・導入Round |
| --- | --- | --- |
| NFR-01 | 原稿と前回の正常成果物を失敗・中断・更新削除から保全する | preservation: bytes比較、故障注入、R1/R2/R4 |
| NFR-02 | Install/Config/Data/Cache/State/Project/BuildRootを分離する | roots: 所有・相互干渉・削除範囲、R1/R4 |
| NFR-03 | 今回生成・検証・確定した成果物だけで成功を判定する | freshness: 古いPDFを置く失敗系、R2 |
| NFR-04 | 依存固定、実入力digestと実行条件の記録で再現性を評価できる | reproducibility: 同一入力の反復と差分理由、R2/R3 |
| NFR-05 | 組版を最小入力・権限・network・資源で隔離する | isolation: mount/権限/network/制限の実測、R2 |
| NFR-06 | 原稿を送信せずtelemetryを持たない | privacy: network監視、ログ/配布物検査、R2/R4 |
| NFR-07 | 対象OSで日本語・空白を含む任意ローカルProjectRootを扱う | os-path: OS別path matrix、R1/R2/R4 |
| NFR-08 | 静的操作の応答性を測り予算内を目指す | latency: 下記条件で30回、R1以降 |
| NFR-09 | CPU/memory/時間/process/logが有界である | limits: 制限超過・巨大log・逐次Target、R2 |
| NFR-10 | 失敗・disk不足・中断から回復し、未確定世代を成功扱いしない | recovery: snapshot/run/確定の故障注入、R2 |
| NFR-11 | 製品ロジックをRust共通層へ一元化し、CLI/GUI/Justへ複製しない | dependency: crate依存とadapter境界、R1以降 |
| NFR-12 | 要件を正常/異常系テストへ追跡し、未実施を隠さない | coverage: FR/NFR対応表と検証結果、全Round |
| NFR-13 | 配布物・checksum・依存資産・許諾の整合を確認する | distribution: clean install/upgrade/uninstallと許諾、R3/R4 |
| NFR-14 | 1主題1正本、ADR/契約/実行証拠を分離する | docs: 重複・内部リンク・現行能力の確認、R0以降 |

## 初期予算（すべて未実測）

- warm help: p95 300ms。
- 合計5MiB・20filesのcheck/plan: p95 1s。
- 各条件30回で評価し、初回はwarm集計と分けて記録する。OS、CPU、memory、storage、version、入力、測定方法を報告する。
- 組版はTargetごとにmemory 2GiB、2CPU、10min、256process、log 32MiBを初期上限とする。初期はTarget逐次実行。超過を成功として扱わない。

依存固定はPDF bytesの完全一致を既に保証するものではありません。フォント・toolchain・metadata・日時等の影響を実証し、非決定性と未検証項目をBuildRecord/報告に残します。現在日付の暗黙補完は禁止です。詳細な隔離・失敗処理は[build契約](../contracts/build.md)を参照します。
