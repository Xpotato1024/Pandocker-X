---
status: canonical
owner: maintainers
last_verified: 2026-10-23
canonical_for:
  - docs/operations/preview-release.md
---

# Preview配布

2026-10-23のユーザー承認により、統合済みmasterから `v3.0.0-preview.1` を公開対象とします。旧版2.xからの非互換なRust再開発を示す3.0.0のprereleaseです。R4/R5全項目の完了や正式版の品質保証を意味しません。コンテナregistryへの新規公開・課金・認証・権限変更は含みません。

## 利用者

手動展開の導入手順と既知の制約は、archiveに同梱する [README-FIRST](../../packaging/README-FIRST.ja.md) を正本とします。Windows x86_64 MSVC ZIP、Linux x86_64 GNU tar.gz、対応pdx source ZIP、SHA256SUMSを配布します。sourceとbinaryは同じcommit・Cargo.lock・versionに対応します。LinuxはUbuntu 24.04で確認し、Windows native CLIの確認とDocker Desktopの実PDF未検証を区別します。他OS/architectureは対応済みとしません。

## 作成と公開の境界

`.github/workflows/preview.yml` は手動起動のread-only workflowです。Rust 1.99.0とlockを使い、fresh runnerでrelease binaryを作ります。Python標準libraryの `packaging/preview.py` がcleanなcommitからsource・license・checksumを同梱し、`packaging/smoke.py` がcheckout外の新規app/project/stateへ展開して内部checksum、版、init/check/plan、失敗時の原稿保全をRustなしPATHで検査します。workflowはRelease/tag/imageを公開しません。

maintainerは対象masterの静的CIと配布CI、独立review、実環境での展開確認を終えてから、正確なcommitを指定したGitHub prereleaseを作ります。生成済みassetを入れ替えず、公開後のdownloadとhash照合を行います。署名鍵や新credentialsは作成しません。

MIT本文とpdx対応ソースは各binary archiveに含みます。targetから到達する固定crateのlicense本文とRust toolchainのlicenseを収集します。crate内に本文がない場合は `.cargo_vcs_info.json` の公式commitと一致する補完本文をhashで固定します。補完の出典は `packaging/license-overrides.json` に記録します。依存全sourceのvendor、コンテナimage配布、byte単位で同一な再buildは保証しません。

失敗したCI、未解決のreview、版/commit/hash不一致があれば公開しません。public downloadに失敗した場合は、公開状態と原因を記録して成功扱いにしません。
