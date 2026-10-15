# ADR-0001: 履歴・LICENSEを保持した非互換再開発

- Status: Accepted
- Date: 2026-10-15
- 根拠: ユーザー承認、[Issue #17](https://github.com/Xpotato1024/Pandocker-X/issues/17)

## 背景と判断

旧実装の修理・移植ではなく、同じrepositoryで新しい製品責務を実現します。旧版は `archive/pre-rebuild-2026-10-15`、SHA `4fac1495e5418c57bb23e74509091ae333a5e7f4` に保全します。Git履歴とLICENSEは保持しますが、実装・仕様・互換性は継承しません。

## 帰結

確認済み専用cloneの旧追跡ファイルを撤去し、旧CLI、CI/release workflow、設定、template、旧docs、legacyを新版treeに残しません。ユーザーの他checkout・未追跡原稿には触れません。旧Issueのclose、Release変更、tag/merge/image公開は含みません。保全の実施証拠は[R0報告](../reports/r0-foundation.md)で扱います。
