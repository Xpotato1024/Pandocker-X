# Pandocker-X 3.0.0-preview.1

Rust版への非互換な再開発のpreviewです。旧2.xの設定・文書・CLIとの互換性はありません。既存原稿への適用前に、新しい専用directoryで確認してください。

## 配置と最初の確認

Releaseの `SHA256SUMS` とダウンロードしたarchiveのSHA-256を照合し、ユーザーが所有する新しいapp directoryに展開します。内部の `SHA256SUMS` は全同梱fileの検査用です。署名付きバイナリではありません。PATH・registry・Docker設定を変更するinstallerは含みません。

Windows用はx86_64 MSVC版、Linux用はx86_64 GNU版です。Windows native CLIとUbuntu 24.04のLinux CLIを検証対象とします。Linuxはglibcを必要とし、musl/macOS/ARMは対象外です。Windowsで実行時DLLが不足する場合はMicrosoft公式のVisual C++ Redistributable x64を別途用意してください。Windows native + Docker Desktopの実PDF生成は未検証です。

Windows PowerShellの例（appを展開したdirectoryで実行）:

```powershell
.\pdx.exe --version
.\pdx.exe init "$HOME\pdx-preview-example" --template default-report
.\pdx.exe check "$HOME\pdx-preview-example"
.\pdx.exe plan "$HOME\pdx-preview-example" --target report
```

Linuxの例（appを展開したdirectoryで実行）:

```sh
./pdx --version
./pdx init "$HOME/pdx-preview-example" --template default-report
./pdx check "$HOME/pdx-preview-example"
./pdx plan "$HOME/pdx-preview-example" --target report
```

initは新規/空directoryに限定します。app・Project・Stateを分けてください。標準のStateはユーザー領域です。必要なら `PDX_STATE_ROOT` にProjectやappの外の専用絶対pathを設定します。静的check/planはDocker不要で、PDF生成準備済みとは表示しません。

## 最初のPDF（Linux + 既存Docker Engine）

Dockerは利用者が事前に用意します。同梱sourceの `runtime/Dockerfile` は固定した公式Pandoc/TeX baseを使います。次はnetworkを利用する明示的なimage buildです。新しいregistryへ公開しません。pdx自体はimageをpullせず、既存local imageの完全なIDを固定します。

```sh
docker build --platform linux/amd64 --pull=false -t pdx-runtime:local source/runtime
IMAGE_ID=$(docker image inspect pdx-runtime:local --format '{{.Id}}')
./pdx lock "$HOME/pdx-preview-example" --runtime-image "$IMAGE_ID"
./pdx build "$HOME/pdx-preview-example" --target report --timeout-seconds 600
```

成功するとProject内の新しい `output/report/<build-id>/document.pdf` に出力します。失敗・取消・再実行では前の世代を維持します。local templateとTargetの変更方法は `source/docs/contracts/project.md`、エラー/終了コードは `source/docs/contracts/cli.md` を参照してください。

## 同梱物・制約・更新

`source/` はBUILD-INFO.jsonのcommitに対応するpdx全ソースです。MIT LICENSE、依存crateとRustのlicense本文、THIRD-PARTY-NOTICES.txtを同梱します。依存の対応ソースはnoticesの固定版crates.ioリンクから取得できます。Cargo依存をvendorしたoffline source distributionではありません。ソースからはRust 1.99.0とlinkerを用意し、`source` 内で `cargo install --path apps/pdx --locked --root <専用app directory>` を実行できます。

コンテナimage・Docker・TeX本体は配布archiveに含みません。PDFの生成と検査を確認するpreviewであり、正式な組版品質、Windows Docker Desktop、host間/byte単位の再現性、disk full/電源断は未検証です。fetch/doctor/clean、全Target同時build、installer、自動更新・削除はありません。更新は別の新規app directoryへ展開して版を確認し、既存Projectとlockを維持します。古いappやStateを自動削除しません。
