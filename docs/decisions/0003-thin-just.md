# ADR-0003: 開発用Justは薄い委譲

- Status: Accepted
- Date: 2026-10-15
- 根拠: ユーザー承認、[Issue #17](https://github.com/Xpotato1024/Pandocker-X/issues/17)

## 背景と判断

開発作業の入口を短くしても、製品ロジックの第二実装を作らないことが必要です。開発用JustはCargoや正式な検証/runtime操作への薄い委譲に限定します。利用者はpdxを使います。

## 帰結

Justに設定解決、metadata結合、path安全検証、build orchestrationを実装しません。Just自体を製品利用の必須依存にしません。R0には委譲先がないため動作を装うjustfileを置かず、R1以降に必要なrecipeのみ導入します。
