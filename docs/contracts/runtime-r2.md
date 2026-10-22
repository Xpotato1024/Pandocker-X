---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/contracts/runtime-r2.md
---

# 最小R2の具体契約

R1の静的契約へ、`lock [path] --runtime-image <完全local ID>` と `build [path] --target <id> --timeout-seconds <1..600>` を追加します。target省略はdefault_targetです。all-targets/fetch/doctor/cleanは未実装です。初回PDFまでの一つのrecipeだけを実装し、既存templateのdefaults/schema・metadata結合をそのまま使います。

## 固定と導入

開発checkoutの `runtime/Dockerfile` は公式pandoc/latexのlinux/amd64 manifest digestから作り、Alpine署名packageのqpdfとNoto CJKを追加します。tagの文字列を固定情報として使いません。Dockerの `image inspect --format '{{.Id}}'` が返す完全IDをlockします。このlocal IDとregistry manifest digest・Dockerfileの再build結果は別の同一性です。Alpineの推移的依存はrepositoryの更新を受けるので、Dockerfileだけで同じimageを再生成できる保証はありません。固定済みlocal imageを保持した再実行が今回の範囲です。

固定操作はimage・tool/version・日本語フォントと制限付きcontainer起動をprobeし、成功時だけlockへ保存します。imageはLinux amd64、local Unix socketまたはWindows named pipeのみ、VOLUME宣言なしを要求します。remote Docker、tag、架空ID、ゼロIDを拒否します。buildはpull・download・lock更新を行いません。専用DockerRuntimeインスタンスを一つのbuildへ使い、並列共有は非対応です。

旧lockはschema 1の `runtime: "unresolved"` を維持します。明示固定後は[lock schema 2](../../schemas/lock-v2.schema.json)の `runtime: {"docker":{"image_id":"sha256:…","platform":"linux/amd64"}}` とします。通常の `lock` は有効な既存pinを保持します。破損lockを明示更新した場合は未解決へ戻ることがあるため結果をcheckしてください。[plan schema 2](../../schemas/plan-v2.schema.json)は固定runtimeを返しますが、静的なcheck/planだけではruntime_verified/build_readyはfalseです。template lockとruntime pinを自動更新しません。

## snapshotと隔離

StateはWindowsのLOCALAPPDATA/Pandocker-X/state、LinuxのXDG_STATE_HOME/pandocker-xまたはHOME/.local/state/pandocker-xです。`PDX_STATE_ROOT` の絶対pathで上書きできます。リンク・親component・Projectとの重複を拒否します。target output内の予約file `.pdx-build.lock` をOS排他し、Stateの指定を変えても同一outputの二重buildを拒否します。空のlock fileは残り、OSがprocess終了時に排他を解放します。

Stateの専用workspaceへ選択Target/Documentの宣言入力だけをcopyします。1 file 32MiB・合計256MiB・10000 filesまでです。resource directoryはその宣言された子孫を含みます。設定・lockの元bytes、実行用に絞った設定・lock、結合metadata、template資産をhashしてBuildRecordへ記録します。snapshotと元Projectの再plan、および確定前の元入力bytes再hashで変更を拒否します。組版中はsnapshotだけを読みます。

Dockerにはsnapshot一つだけをread-only mountします。書込みhost mountはありません。non-root、network none、read-only rootfs、cap-drop ALL、no-new-privileges、2CPU・2GiB・256pids・512MiB tmpfsを指定します。tmpfs内でPandoc→LuaLaTeX二回（no-shell-escape）→qpdf --checkを実行し、検査成功後だけPDFをhostへ転送します。stdout/stderrの累積上限32MiBにはPDF転送bytesも含めます。tool本文logは永続保存せず、phaseとexit codeを診断します。cleanupには独立した小さい期限・出力予算を使います。

timeoutはprobe以後のsnapshot/runに共通の最大600秒です。cleanupの最大10秒は別枠です。Ctrl+Cは取消flagを設定し、processのkill/waitと、所有labelが一致するcontainerの完全IDによる削除へ進みます。cleanup失敗は残存名/pathとexit 5を返します。daemonの遅延応答・強制kill・電源断で完全収束できない場合は成功扱いせず、今回のbuild-idで所有物を確認して復旧します。daemon設定・security設定は変更しません。

## 世代の確定と復旧

qpdf成功に加えて、転送したPDFのheader/EOF・非空・上限を検査します。outputのbuild-id付き新規directoryへPDFを排他作成してsyncし、BuildRecordのpending fileをsyncし、最後に `build.json` へrenameします。これがcommit markerです。正常なrecordと一致するPDF digestがないdirectoryは成功世代ではありません。前回世代と任意のユーザーPDFを置換しません。失敗時は今回作成したfileと空directoryだけを戻し、所有不明のfileを消しません。

[BuildRecord schema 1](../../schemas/build-record-v1.schema.json)はbuild-id、Target/Document、template identity、local runtime pin、tool evidence、snapshot各入力のsha256/bytes、本文順、PDF digest/検査、committed状態を持ちます。失敗はState/failuresへbuild-id・code・exitだけを残し、本文を含めません。workspaceは成功/失敗とも削除し、失敗した場合はpathを報告します。強制終了で残るworkspace・未確定世代の自動cleanは未実装です。削除前に所有とcommit markerを個別確認してください。

`build --json` のdataは[build result schema 1](../../schemas/build-result-v1.schema.json)に従います。実Dockerとqpdfまで成功した場合だけ両verifiedをtrueにします。fake runtimeによる制御試験は両方falseです。書式適合・日本語組版品質・別端末/別OSの再現性・製品配布は別の受入です。敵対的な同一userによるpath差替えと電源断の完全transaction保証は含みません。
