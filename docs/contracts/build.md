---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/contracts/build.md
---

# build・隔離・成果物契約

R1の静的resolve/planへ最小R2 buildを追加しました。今回実装したコマンド・lock版・隔離・予算・世代確定と制約の正本は[最小R2の具体契約](runtime-r2.md)です。以下の工程表は当初の計画と全体要件を示し、fetch/doctor/clean等の未実装項目を成功扱いしません。

## 初回PDFまでの最小R2実装範囲

最小経路として `build` command、Docker adapter、local runtime lock、snapshot、qpdf検査、BuildRecord、世代確定を実装します。配布imageを取得するfetchと自動更新は含みません。開発用Linux compiler containerの起動と、組版runtimeの実PDF検証は別の証拠です。

初回の縦断実装は一つのDocument・Target、内蔵default-report、Markdown→standalone LaTeX→LuaLaTeX→PDFに絞ります。GUI、複数engine、template配布管理、watch、installer自動更新を先に増やしません。本文・metadata・図・bibliographyは既存の宣言モデルを使い、template名によるRust分岐は追加しません。

| 最小工程 | 実装する責務・受入条件 |
| --- | --- |
| 開発導入 | 現在のsource installを使い、ProjectをInstall外に作る。Rust不要の配布受入はR4へ分離 |
| runtimeの用意と固定 | R2で開発用Dockerfileを作り、Pandoc・LuaLaTeX・日本語フォント・PDF validator（qpdf）を一つのimageへ含める。まずlocal buildしたimageを使い、公開を前提にしない。probeで実在・tool/version・fontと隔離条件を確認するまで準備済みとしない |
| lock拡張 | 版付きlockへruntime種別・platform・固定image同一性を追加。local imageのconfig IDとregistry manifest digestを区別し、tagだけを許可しない。旧lockのunresolvedを勝手に解決しない。明示したruntime固定操作だけが更新する |
| Docker境界 | adaptersにargv起動・bounded log・timeout/cancel・専用containerの収束を一か所で実装。applicationがprobe/snapshot/run/確定の順序を所有し、CLIは引数と表示だけ。host TeX・独自TeX parser・Composeへの組版処理の複製は不要 |
| 入力snapshot | 宣言入力とtemplateの必要bytesだけをtask別BuildRootへcopyし、実際のbytesをhash。path/リンクとcopy前後の整合を再検査。組版中にProjectを読み直さず、Project全体をmountしない |
| 実行とPDF判定 | 下記の隔離条件とNFR上限で一つのTargetを実行。今回の専用出力に新規・非空PDFがあり、image内のqpdf検査まで成功した場合だけ次へ進む。commandのexit 0や古いPDFは成功条件にならない |
| 成果物確定 | build-id付き新規世代へPDFとBuildRecordを同一filesystemのstagingから確定。既存PDFを上書きしない。失敗時は今回所有物だけを収束させ、残存pathと非0結果を返す |

R2の最初の動作保証は、ローカルで用意して固定したimageによる一つの実PDFまでです。registry向けの `fetch` は固定manifest digestを明示取得する別操作として追加し、buildがpull・lock更新を暗黙実行しない境界を維持します。local imageのみの試験を、他端末への配布・取得の受入とは呼びません。

最低限の異常系はDocker不在、固定image/tool不足、snapshot中の入力変化、領域逸脱、組版異常/timeout/cancel、空/破損/古いPDF、保存失敗、確定途中失敗です。原稿と前回世代のdigest保全を各ケースで確認します。成功した実PDFとこれらの異常系がそろうまで、runtime/build-readyの保証へ進めません。image公開・製品release・daemon設定変更はこの実装計画によって許可されません。

## 処理順

1. validate: Project、Target、template、metadata、path、lock整合を検証する。
2. resolve/plan: ローカルで固定済み依存と入力順・出力を解決する。取得やlock更新はしない。
3. runtime probe: Docker、固定image、要求toolと隔離条件の実体を確認する。
4. declared input snapshot: 宣言された本文・図・bibliography・metadata・必要template資産だけをBuildRootへcopyし、実際に使うbytesのdigestを記録する。copy前後のpath/リンク境界と入力整合を確認する。
5. run: 制限したコンテナでsnapshotを組版する。
6. validate PDF: 今回のbuild-idの新規出力について、存在・非空・PDF形式/読取可能性・要求された成果物の充足を確認する。image内qpdfの具体実装・実probe・受入はR2で固定する。
7. 確定: PDF・BuildRecord等を世代build-id付き成果物セットとして確定する。中途半端な世代を成功扱いしない。
8. cleanup: 今回所有のprocess・コンテナ・一時物を収束させ、結果と残存物を記録する。

同一Project/Targetは排他し、競合build/cleanを拒否します。別Targetでも同一出力先を許しません。初期all-targetsは逐次で、個別結果を保持し一つ失敗すれば全体非0です。原稿や前回の正常PDFは常に保全し、古いPDFの存在やprocessのexit 0だけで成功としません。

## 成果物・復旧

BuildRecordはbuild-id、Project/Document/Target、template/toolchain固定情報、実入力digest、実行条件、検証結果、成果物digest、失敗/中断/確定状態を追跡します。本文等の実入力digestをlockへ混ぜません。原稿本文やcredentialsをlogへ複製しません。

新しい世代をstagingで揃え、確定できるまで前回成果物を置換しません。disk不足、保存失敗、中断、process異常、cleanup失敗を隠さず非成功とし、残存pathと回復方法を示します。確定とcleanupを含む成功状態・OS別の安全な切替手段はR2で故障注入して具体化します。確定済み世代を、前回PDFとは独立して識別できなければ成功を返しません。

## Docker安全境界

non-root、read-only-rootfs、cap-drop、no-new-privileges、network-none、resource limits、no-shell-escapeを要求します。[NFR予算](../requirements/non-functional.md)を適用し、満たせないruntimeは診断して停止します。

コンテナへhome、Docker socket、credentials、ProjectRoot全体を渡しません。必要入力copyと検証済みtemplateのみをread-onlyで渡し、書込み先は今回専用のbuild領域へ限定します。ホスト外部processはargvで起動し、原稿pathやmetadataからshell文字列を組み立てません。snapshot後もhost Projectへの書込み経路を与えません。

組版中はnetworkを使わず、download、Git操作、lock更新、host TeX fallbackはしません。信頼したtemplateでも隔離は解除しません。Docker daemon自体の信頼やOS差を含む残余リスクは実測と区別して報告します。
