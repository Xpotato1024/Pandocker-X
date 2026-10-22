# 貢献方法

現在はvNext R1の静的CLIを実装しています。ビルドと利用方法は[README](README.md)、保証範囲は[R1報告](docs/reports/r1-static-cli.md)を確認してください。[実装段階](docs/operations/rounds.md)で対象Roundを確認し、1 Issue = 1 PRで進めます。旧版の機能・Issue順を自動的に持ち込みません。

[正本マップ](docs/README.md)で該当主題を探し、要件ID・契約・検証方法を先にそろえてください。文章は日本語UTF-8・LFとし、仕様のコピーや空の大量stubを追加しません。ADRは採用理由、contractsは実装時の規範です。未実装・未検証を動作保証として書かないでください。

実装ではRustのcore/application/adapters/CLIの責務を守り、GUIも同じapplication APIを使う方針です。開発用Justを導入する場合は既存のCargo・検証・runtimeの処理へ薄く委譲し、製品ロジックを複製しません。

作業前確認、差分gate、検証証拠、PR記載事項、独立review、一時物の管理は[開発運用](docs/operations/development.md)を正本とします。原稿をfixtureとして無断収集・送信せず、第三者テンプレートや資産には出典と許諾を付けます。
