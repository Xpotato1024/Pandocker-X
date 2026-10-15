---
status: canonical
owner: maintainers
last_verified: 2026-10-15
canonical_for:
  - docs/requirements/functional.md
---

# 機能要件

製品はユーザー所有原稿をテンプレートと固定した環境でPDF化するローカルアプリです。以下は実装予定の受入条件であり、R0では全FRが未実装・テストNot Runです。「検証」は将来追加するテストシナリオの識別名です。R1以降に実在するテストpathと結果をこの表へ結び付けます。

| ID | 要件・受入条件 | 検証シナリオ | 導入Round |
| --- | --- | --- | --- |
| FR-01 | 指定pathとテンプレートで初期化し、既存原稿を上書きしない | init: 空path・既存path・日本語/空白path | R1 |
| FR-02 | Documentのsourcesを宣言順で扱い、metadata・bibliography・resourcesを入力に含める | document: 順序変更と欠落入力 | R1 |
| FR-03 | 複数Documentと複数Target、default_targetを区別する | target: 同一文書別書式・別文書・不正参照 | R1 |
| FR-04 | IDまたはローカルpathでTemplatePackageを選択する | selection: ID/path解決と未登録 | R1 |
| FR-05 | テンプレート記述、必要資産・tool、許諾・出典・sampleを検証する | template: 正常packageと欠落宣言 | R1静的/R3実証 |
| FR-06 | metadata結合後にschemaで検証し、未知キー・不正型・重複を拒否する | metadata: defaults/Document/Target優先順 | R1 |
| FR-07 | lockは依存のみ固定し、fetchは固定依存を明示取得する | lock-fetch: 依存変更・本文変更・未準備 | R1 lock/R2 fetch |
| FR-08 | Docker/Git/networkなしでinit/check/planを実行できる | static: 接続不能環境と不正path | R1 |
| FR-09 | 準備済み依存をDockerで組版し、build中に取得・lock更新・Git操作しない | runtime: digest/tool不足・network遮断 | R2 |
| FR-10 | 今回のPDFを検証し、build-id付き成果物セットを確定する | artifact: 古いPDF・空/破損PDF・確定失敗 | R2 |
| FR-11 | doctorでruntime不足を診断し、中断時に子processと一時物を収束させる | diagnose-cancel: runtimeなし・timeout・cancel | R2 |
| FR-12 | version付きJSON一objectと終了コードで結果を返す | json: 正常・入力失敗・runtime失敗・未実装 | R1以降 |
| FR-13 | cleanは既定dry-run、--applyで確認済みアプリ所有物だけ削除する | clean: 外部path・危険リンク・原稿混在 | R2 |
| FR-14 | user単位で導入・更新・削除し、Projectを変更しない | lifecycle: 更新失敗とuninstall後の原稿 | R4 |
| FR-15 | 依存準備後はofflineでcheck/plan/buildできる | offline: networkなしで実PDF、依存欠落は失敗 | R2 |
| FR-16 | watchによる再build | watch: 後続設計で具体化 | 後続、R0〜R5必須外 |
| FR-17 | 提出用source export | export: 許諾・必要入力の追跡を後続設計 | 後続、R0〜R5必須外 |

コマンドは[CLI契約](../contracts/cli.md)、データは[Project契約](../contracts/project.md)、成功判定は[build契約](../contracts/build.md)に従います。GUI、旧版互換、未知ZIPの自動適合、旧Issue closeは今回の受入条件に含めません。
