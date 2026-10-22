# Pandocker-X vNext

ユーザー所有の原稿を、選択したテンプレートと固定した組版環境でPDF化するローカルアプリを新規開発します。CLI-firstで、利用者の入口は `pdx` とします。

## 現在の状態

**R0は仕様・文書・開発運用の基盤のみです。vNextのCLI、Rust workspace、組版image、テンプレート、インストーラは未実装です。実行できるクイックスタートはまだありません。** 文書にあるコマンド・型・安全保証・性能値は、後続Roundで実装・検証する契約または目標です。

2026-10-15承認の非互換な再開発です。旧実装・仕様・CLI互換性は継承しません。旧版は `archive/pre-rebuild-2026-10-15`、SHA `4fac1495e5418c57bb23e74509091ae333a5e7f4` に保全し、Git履歴と[LICENSE](LICENSE)を保持します。旧コードを新版treeへ移植せず、`legacy/` も残しません。保全の確認結果は[R0報告](docs/reports/r0-foundation.md)に記録します。

保護pathの削除・規約の置換は親の適用待ちで、現行treeには一部の旧ファイルが残っています。適用待ち一覧と検証範囲は[R0報告](docs/reports/r0-foundation.md)を参照してください。

## 読み始める場所

- [文書正本マップ](docs/README.md): 主題ごとの唯一の正本
- [機能要件](docs/requirements/functional.md)と[非機能要件](docs/requirements/non-functional.md): 受入条件と検証への追跡
- [構成と責務](docs/architecture/system.md): Rust共通コア、application API、Dockerと領域分離
- [実装契約の入口](docs/contracts/README.md): 設定・CLI・テンプレート・build
- [実装段階](docs/operations/rounds.md): R0からR5への進め方
- [貢献方法](CONTRIBUTING.md)と[agent入口](AGENTS.md)

初期対象はWindows 11 x64 + Docker DesktopのLinux containers、およびLinux x86_64 + Docker Engineです。WSLではLinux版pdxを使う設計です。Dockerによる組版を正とし、host TeXへfallbackしません。GUI、旧版互換、未知ZIPの自動適合は今回の対象外です。
