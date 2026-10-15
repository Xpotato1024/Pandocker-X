# ADR-0004: 旧agent規約を新版規約で置換

- Status: Accepted
- Date: 2026-10-15
- 根拠: ユーザー承認、[Issue #17](https://github.com/Xpotato1024/Pandocker-X/issues/17)

## 背景と判断

旧CLI維持、古いmodel固定、旧Issue順は非互換再開発と矛盾するため継承しません。旧agent文書を新版の短い入口と主題別の運用正本へ置換します。modelはruntimeで公開された実効設定を確認し、固定した旧model名を完了gateにしません。

## 帰結

[AGENTS](../../AGENTS.md)を入口とし、詳細は[開発運用](../operations/development.md)へ集約します。原稿保全、repository identity、認証情報、公開権限、workspace hygiene、Round scopeなどの安全境界は維持します。高リスク境界は実装者と別instanceの独立reviewを要求し、self-reviewで代替しません。旧model名の廃止はreview gateの廃止を意味しません。
