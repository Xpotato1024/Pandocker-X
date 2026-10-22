# R0実装・検証記録

- 日付: 2026-10-15
- 対象: Xpotato1024/Pandocker-X、Issue #17
- 旧版SHA: 4fac1495e5418c57bb23e74509091ae333a5e7f4
- 保管ref: archive/pre-rebuild-2026-10-15
- 作業branch: codex/17-vnext-foundation

## 実施

旧版をremote branchとして保全し、fetchした参照と開始HEADの一致を確認した。新規専用cloneで旧追跡ファイルを撤去し、LICENSE、履歴、新版の文書・運用基盤だけを残した。既存ユーザーcheckout、原稿、設定、Releaseは変更していない。

編集担当のsandboxで保護されたファイルはcandidateとして引き継ぎ、親が承認済みscopeと元blobを照合して適用した。sandbox/ACL/認証設定は変更していない。GitHub CLIは未認証だが、接続済みGitHub連携でrepositoryとarchiveを確認した。

独立した文書確認で、CLIの親探索の記述を承認済み設計に合わせ、恒常AGENTSからsession固有の分業指定を除去した。FR/NFRは未実装の受入条件として保持し、R1以降の動作成功とは区別した。

## 検証

親による適用後snapshotについて、LICENSE blob一致、残存旧ファイルなし、UTF-8、Markdown内部リンク、git diff --checkを確認する。完全な検証結果は同じPRの検証欄へ記録する。

R0にはRust製品実装、Docker組版、配布物はないため、それらの動作試験はNot Run。実効model名やPDF適合は確認済みと主張しない。merge、tag、Release/image公開は行わない。
