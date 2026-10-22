---
status: canonical
owner: maintainers
last_verified: 2026-10-22
canonical_for:
  - docs/requirements/functional.md
---

# 機能要件

製品はユーザー所有原稿をテンプレートと固定した環境でPDF化するローカルアプリです。以下は全体の受入条件です。R1の対応状況と実在テストは下表の追跡に分け、実行結果を[R1報告](../reports/r1-static-cli.md)に記録します。静的実装だけで全FRを完了とは扱いません。

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

## R1 traceability

最小R2の追加追跡: FR-07/09/10/11/12/15は[build制御tests](../../tests/build_control.rs)の明示pin、旧lock、選択snapshot、排他、失敗/取消/不正PDF/入力変更/世代衝突/再実行と、[R2実PDF報告](../reports/r2-first-pdf.md)に追跡します。doctor/fetch/clean、全Target実行、配布lifecycleは未実装のままです。以下はR1時点の対応表です。

| 要件 | R1対応とテスト |
| --- | --- |
| FR-01 | 新規/空directory init。[contracts](../../tests/contracts.rs): init_collision_rollback_and_concurrent_init、init_check_plan_japanese_path_and_preservation |
| FR-02 / FR-03 | ordered sources、複数Target/default、全Document入力。[contracts](../../tests/contracts.rs): sources_order_crlf_two_targets_and_metadata_merge、configuration_error_matrix_preserves_files |
| FR-04 | built-in IDとProject内local path。[contracts](../../tests/contracts.rs): local_package_identity_digest_and_explicit_lock |
| FR-05 | 部分対応: manifest/tool宣言/schema/資産/digestの静的検証。template_and_schema_rejections_and_local_refs、schema_names_and_literal_data_are_not_schema_keywords、schema_guard_rejects_percent_encoded_reference_bypass、schema_guard_rejects_nested_dialect_switch、[template unit](../../crates/pdx-core/src/template.rs)のschema位置別拒否・予算試験。実tool/公式書式適合はR3 |
| FR-06 | YAML厳格parse、merge、標準schema。[core unit](../../crates/pdx-core/src/metadata.rs)、[contracts](../../tests/contracts.rs): metadata_error_matrix、sources_order_crlf_two_targets_and_metadata_merge |
| FR-07 | 部分対応: ローカル依存lock、stale検出、本文変更時不変。manuscript_edits_do_not_change_dependency_lock、local_package_identity_digest_and_explicit_lock。runtime/fetchはR2 |
| FR-08 | 実CLIをcheckout外へcopyしPATHを空にしてinit/check/plan。[cli](../../tests/cli.rs): real_binary_outside_checkout_runs_init_check_plan_and_nearest_root。network利用コードなし。OS network監視は未実施 |
| FR-12 | R1コマンドの成功/失敗/引数不正JSONと終了2/3/4/5。[cli](../../tests/cli.rs)。human_output_guides_template_switch_and_error_recoveryは通常表示のstdout/stderr、template切替・明示lock・metadata位置と復旧案を検証。runtime失敗はR2 |
| FR-09 / FR-10 / FR-11 / FR-13 / FR-14 / FR-15 | 未完了: Docker実行、PDF検証、doctor/cancel、clean、配布lifecycle、準備後offline buildはR2以降。静的check/planの実行だけはR1 |
| FR-16 / FR-17 | 後続設計。未実装 |

path異常系と入力非変更はportable_path_and_output_overlap_matrix、filesystem_alias_and_non_directory_outputs、symlinks_are_rejected_in_all_filesystem_roles（Linux）、windows_junctions_are_rejected_without_symlink_privilege（Windows）へ追跡します。fixtureはtest内でtask temp配下に生成し、通常checkoutへ原稿を生成しません。
