# 最小R2: 初回PDF経路の実装・検証

2026-10-22の明示承認に基づくPR21上のstackです。baseは `codex/18-template-diagnostics` / `307fc34b3cfd4a550d25fdb8695f83f13e1e67ea`、head branchは `codex/r2-first-pdf`。PR19/20/21は統合・変更しません。仕様は[最小R2契約](../contracts/runtime-r2.md)を正本とします。

## 実装した経路

Dockerfileの明示local build→完全local image IDのprobe/lock→選択入力snapshot→tmpfs内Pandoc/LuaLaTeX→qpdf→新規世代PDF/BuildRecord確定まで実装しました。CLIへ成功するstubを追加した段階ではありません。既存の純粋core、application順序、FS/Docker adaptersの責務を維持し、独自TeX処理・Compose・host fallback・template IDによる実行分岐は追加していません。

## 検証したこと

Windows MSVCと既存Ubuntu-24.04/WSLのRust 1.99.0でfmt/check/clippy、全42テストが成功しました。task専用cache/target/temp、2 cargo jobs・1 test threadを使い、skip/ignoreはありません。Linux compileは既存の確認済み開発imageをread-only/network-none/非root・2CPU/2GiBで実行しました。これは組版runtimeとは別です。

[build制御tests](../../tests/build_control.rs)はfake runtimeで、明示pin/版2schema、旧lock、宣言外fileの除外、local template、別Stateを含むOS排他、失敗/timeout/取消/不正PDF、入力変更、既存世代衝突、再実行と前回digest保全を確認します。synthetic PDFは実PDFとして扱わず、recordの両verifiedをfalseにします。Docker unitは累積32MiBとmountのCSV引用を確認します。

実Dockerは既存UbuntuのEngine 29.2.1を利用しました。[公式Pandoc Dockerfiles](https://github.com/pandoc/dockerfiles)の公式 `pandoc/latex:3.11.0.0` からlinux/amd64 manifest `sha256:bd7641edf35bb8fad1316bacdac15fcc15f4d8d4f89051da2edcace15a6366d8` を確認・取得し、署名付きAlpine v3.24のqpdf 12.3.2-r0とfont-noto-cjk 0_git20220127-r1を追加しました。Dockerから返ったlocal IDは `sha256:3c40e6f88a3a4f10e8f92faf6dde28ff8ea12104b70ce6bd7126d85c42b5dade` です。manifest digest、image config、Dockerが返すlocal IDを同一視しません。

新規雛形のinit→pin→buildが成功し、Pandoc 3.11、LuaHBTeX 1.24.0/TeX Live 2026、qpdf 12.3.2、日本語fontをprobeしました。今回のPDFはqpdf検査成功、1 page、NotoSerifCJKjp-Regular/Boldを含みます。実runtime/PDFのverified=trueと、新規世代のBuildRecordを確認しました。

Linux debug source installをcheckout外のtask専用app rootへ行い、PATHを `/usr/bin:/bin` としてRust/toolchainなしで、日本語・空白pathへinit→pin→check→plan→初回buildが成功しました。local packageへtemplateを切り替えた別Targetも、defaultsの変更がplanへ反映されて実PDFが成功し、既存report世代のdigestは不変でした。source installは配布installerの受入ではありません。

実Docker上で、存在しないimageはexit 3、無限TeXの12秒timeoutはexit 1、実行container観測後のSIGINTはexit 130でした。当初のTeX不正ケースは実際には30秒のbuild_timeoutだったため、TeX異常終了の証拠から除外しました。最終レビュー後に明示raw LaTeX blockのerrmessageを別途注入し、120秒期限内で `tex_failed` / exit 1を確認しました（観測44.223秒、性能保証ではない）。各失敗後に前回output file digestが不変で、原稿を戻した再実行はexit 0、所有label付きcontainerの残存は0です。raw logs・実PDF・BuildRecord・case別JSONはtaskのローカル `evidence-r2` に保存します。性能測定/改善率ではありません。

独立read-onlyレビューでStateの親component、別Stateによる排他迂回、image VOLUME、呼出し別log予算の4点が見つかり修正しました。再確認で解消し、追加P1/P2所見はありません。レビュー自身は実Dockerを実行しておらず、実行証拠と区別します。

## 残る検証・権限

Windows native + Docker Desktopでの実PDF、公式書式/日本語組版品質の目視適合、他端末再現性、PDF byte完全一致、累積log超過の実process故障注入、disk full・commit途中の電源断・敵対的path差替えは未検証です。fake制御試験をこれらの代用としません。local image IDは再buildの完全再現性を保証しません。並行taskのある共有端末であり、時間観測を改善率にしません。

fetch/doctor/clean、installer・更新/rollback/uninstall、all-targets、image/製品公開は未実装です。自動deploy workflowはなく、既存static validationだけです。共有はstacked Draftとし、merge順は #19 → #20 → #21 → 本変更。先行squash後はrebase・差分/再検証が必要です。merge/tag/Release/image公開・deploy、daemon/security設定・資格情報・他ユーザー原稿の変更は行っていません。
