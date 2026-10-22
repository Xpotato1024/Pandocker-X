---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/contracts/cli.md
---

# CLI・結果契約

**全コマンドは計画でありR0では未実装です。** CLIと後続GUIは同じapplication APIを呼びます。未実装commandをexit 0で返すstubは作りません。

| 計画コマンド | 効果・境界 |
| --- | --- |
| `pdx init <path> --template <id-or-path>` | ローカルの検証済みtemplateからProject作成。既存ファイルを上書きしない |
| `pdx check [path]` | 設定・入力参照・metadata・template/lock整合を静的検証 |
| `pdx plan [path] --target <id> --json` | 検証済みBuildPlanを出力。runtime可用性や成功は保証しない |
| `pdx lock` | template/toolchain依存の明示解決・固定。原稿変更では更新不要 |
| `pdx fetch` | lockに固定した依存を明示取得・digest検証 |
| `pdx build --target <id>` / `pdx build --all-targets` | 準備済み依存からPDF化。download/lock更新/Git操作はしない |
| `pdx doctor` | OS・Docker・必要tool・固定依存の状態を診断。自動インストールしない |
| `pdx template list` / `add` / `check` | 登録一覧、明示取得/追加、package静的検証。安全境界は緩めない |
| `pdx clean` / `pdx clean --apply` | 既定はdry-run。--applyだけが確認済みアプリ所有物を削除 |

`[path]` 省略時はcurrent directoryから最も近い親の `pdx.toml` を探索します。明示pathがあればそのProjectを使い、InstallRootからは推測しません。target省略時はProjectのdefault_target、all-targetsとの併用は入力エラーです。初期のall-targetsは逐次実行で、一つ失敗すれば全体を非0にします。

init/check/planはDocker・Git・network不要です。未登録templateや宣言不足は明確な入力/依存エラーにし、取得を開始しません。buildは依存を勝手に取得せず、不足ならfetchが必要と診断します。

## 終了コード

| code | 意味 |
| --- | --- |
| 0 | その操作の契約を満たしたsuccess |
| 1 | 組版または今回のartifact検証失敗 |
| 2 | 入力・設定の不正 |
| 3 | runtime・依存の不足/不整合 |
| 4 | 安全境界の違反 |
| 5 | 成果物の保存/結果確定失敗（publish-result）。Release公開を意味しない |
| 130 | cancel |

JSON対応操作の `--json` はversion付きの一objectだけをstdoutに出し、生logと診断はstderrへ送ります。成功・失敗とも同じenvelopeでcommand、status、dataまたはerrorを識別できるようにし、version fieldを含む具体schemaはR1で固定します。JSON出力不能も成功にしません。途中のprogressをstdoutに混ぜたり、失敗を空objectとexit 0で表したりしません。

cleanはProjectの本文・図・metadataやユーザー所有PDFを消しません。削除候補は所有記録と正規化済みpathで確定し、危険リンク・所有不明・領域外は拒否します。更新/削除による原稿保全は[構成](../architecture/system.md)の領域境界に従います。
