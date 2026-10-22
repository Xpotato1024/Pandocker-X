---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/contracts/build.md
---

# build・隔離・成果物契約

R1はvalidateと静的resolve/planだけを実装します。実行・snapshot・成果物確定は未実装で、R2で実PDFと異常系を用いて確認します。R1のBuildPlanは[Project契約](project.md)に従い、runtime未解決・build_ready=falseを維持します。

## 処理順

1. validate: Project、Target、template、metadata、path、lock整合を検証する。
2. resolve/plan: ローカルで固定済み依存と入力順・出力を解決する。取得やlock更新はしない。
3. runtime probe: Docker、固定image、要求toolと隔離条件の実体を確認する。
4. declared input snapshot: 宣言された本文・図・bibliography・metadata・必要template資産だけをBuildRootへcopyし、実際に使うbytesのdigestを記録する。copy前後のpath/リンク境界と入力整合を確認する。
5. run: 制限したコンテナでsnapshotを組版する。
6. validate PDF: 今回のbuild-idの新規出力について、存在・非空・PDF形式/読取可能性・要求された成果物の充足を確認する。具体validatorはR2で固定する。
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
