# R1追補: template検証とCLI導線（2026-10-22）

## 結果と設計判断

既存レビューのschema誤拒否を再現して修正し、通常CLIに日本語の要約・復旧案を実装しました。Windowsでfmt/check/clippyと全33テストが成功し、専用ユーザー領域へのsource install、checkout外・PATH空でのinit/check/planを確認しました。PDF生成は未実装のままです。

確認時のdefault branchは `master`、SHA `4fac1495e5418c57bb23e74509091ae333a5e7f4` です。旧版archiveも同じSHAです。新設計の[PR #19](https://github.com/Xpotato1024/Pandocker-X/pull/19)（R0、`bb7d6a4b526786dfcc21f6b44abf372ed9380de2`）と[Draft PR #20](https://github.com/Xpotato1024/Pandocker-X/pull/20)（R1、`724b19d795bafbdd680452000551ea2d96a94f17`）は未mergeでした。本変更はPR #20のSHAから分岐した独立clone・`codex/18-template-diagnostics` で作成し、既存R1/review worktreeへ書き込んでいません。

| 導線 | 確認結果・今回の判断 |
| --- | --- |
| 導入 | 単一binaryに内蔵templateが含まれる。Cargo標準の `install --path --locked --root` で専用ユーザー領域へ配置できた。独自installerは増やさず、source installとR4配布受入を区別する |
| 初回文書 | init→check→planは成立。R1成功をPDF build成功と表示しない。最初のPDFにはR2の固定runtime・実tool probe・snapshot/隔離・成果物確定が必要 |
| template選択 | Targetのid/path参照を使う既存モデルを維持。選択元/digestを通常planへ表示し、Project契約に切替・複数書式・明示lockの操作例を追加 |
| エラー | 通常表示はstderrに安定code・exit・metadata位置・復旧案。JSON modeは従来のschema 1・stdout一objectを維持。通常表示のpretty JSONをparseしていた利用者は `--json` が必要 |
| 更新・再現性 | 本文変更とtemplate依存変更を区別する既存lockを維持。暗黙修復・取得を追加しない。runtimeはunresolvedで、PDF再現性・自動更新・rollbackは未検証 |
| 責務 | 純粋coreでschema制限を修正し、CLIの `display` moduleで表示だけを担当。Docker/TeX、汎用実行基盤、追加portの重複実装は不要 |

## 修正と回帰証拠

既存独立レビューはsnapshot `a5d09aba90415ac71f602b75cc22c18ec6f75e7f` に対するP2として、`properties.pattern` 等のユーザー名をschema keywordと誤認する問題を記録していました。最新R1にも同じ走査が残っていました。修正前に追加回帰試験を実行し、データ内 `$ref` を外部参照と誤認して `external_schema_ref` で失敗することを観測しました。

修正はdraft-07のschema位置・名前map・schema配列・dependencies・通常データを区別します。schema本体の禁止keyword、外部/再帰参照の拒否、参照先の再検証、すべてのデータを含む深さ/node予算を維持します。標準validatorを置き換えたり独自metadata parserを追加したりしていません。

- [core unit](../../crates/pdx-core/src/template.rs): 禁止keyword/外部refを18種のschema位置で拒否。深い/大きいliteral dataの予算超過も拒否。
- [contract](../../tests/contracts.rs): keywordと同名のproperty/定義/dependency、literalデータを持つローカルpackageでlock/check/plan成功、metadata制約の維持、静的操作の非変更を検証。
- [実CLI](../../tests/cli.rs): 通常表示のinit→check→plan、local template切替、stale lock→明示lock→再plan、unknown Target・metadata位置・引数失敗の復旧案、stdout/stderr・終了コード・原稿digestを検証。既存JSON試験も成功。

固定Rust 1.99.0 MSVC・既存registryのtask専用コピーを使用し、network不要の `--locked --offline` でcheck/clippy/testを実行しました。fmt、git diff --check、文書UTF-8/LF検査も成功しました。全33テストはbinary 1、architecture 4、CLI 5、contract 18、core unit 5です。skip・ignoreはありません。Cargo.lock、toolchain指定、LICENSE、組版資産は変更していません。

source installはrelease profile・task専用root・2 build jobsで成功しました。registry cacheあり、release targetは初回compileで、Cargo報告は2分26秒、呼出し全体は148.9秒でした。既存PATH・user設定は変更していません。source install成功を配布installer・更新/削除の受入とは扱いません。

## 静的操作の時間観測

Windows 11 Home 10.0.26200、i7-9700K（8 core）、約31.9GiB RAM、NTFS、Rust 1.99.0 MSVC release binaryで計測しました。本文18 files＋metadata 1＋bibliography 1、計5MiBの宣言入力を持つ日本語・空白pathのProjectです。空resource directory、設定、lock、内蔵templateはこの20入力の外です。子processのPATHは空で、生成後の全Project file SHA-256は計測前後で一致しました。

| 操作 | 最初の1回 ms | warm median ms | warm p95 ms | 初期予算 ms |
| --- | --- | --- | --- | --- |
| help | 18.9 | 16.9 | 58.6 | 300 |
| check --json | 32.7 | 32.5 | 39.0 | 1000 |
| plan --json | 32.8 | 31.9 | 68.4 | 1000 |

各条件は最初の1回を別記し、その後30回を逐次測定しました。process開始・stdout/stderr回収・終了までを含み、p95は昇順29番目です。R1は本文bytesのparse/hashを行わず、入力存在と静的契約を検査します。したがって組版時間・PDF再現性・大規模文書処理の測定ではありません。

自タスクのbuildと計測は並列化せず、build上限2 jobs、テスト1 threadでした。計測中2.91秒に並行processのCPU増分2.61秒・2.08秒・1.22秒等を観測しました。新規/終了processを含まない観測で、占有環境や厳密な速度比較ではありません。本試行では初期予算内ですが、以前の版との性能改善は主張しません。raw samples・binary SHA・競合記録・実行ログはtaskのローカルevidenceに保存しています。

## 残る範囲と共有条件

Linux、Docker/Pandoc/TeX、実PDF、公式書式適合、network監視、配布installer、更新/rollback/uninstall、PDF再現性はNot Runです。既存R1 headのGitHub connector照会ではworkflow run/status一覧が空であり、Linux CI成功も現在のCI障害も断定していません。本変更の独立レビューは未実施です。

R1 treeのworkflowは読取り権限のstatic validationだけで、deploy・Release/image公開はありません。共有はPR #20をbaseとするDraftとし、merge順は #19 → #20 → 本変更です。先行PRをsquash mergeした場合はrebaseして実差分を再確認します。merge、tag、Release/image公開、Docker daemon設定変更、ユーザーの既存原稿・資格情報・security設定の変更は行いません。製品公開にはR2〜R5の受入と別の明示許可が残ります。
