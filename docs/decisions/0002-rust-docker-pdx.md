# ADR-0002: Rust共通コア + Docker組版 + pdx

- Status: Accepted
- Date: 2026-10-15
- 根拠: ユーザー承認、[Issue #17](https://github.com/Xpotato1024/Pandocker-X/issues/17)

## 背景と判断

OS別scriptやUIごとに検証・buildを複製すると、安全契約が分岐します。Rustで純粋なcore、application/use case/port、FS/Docker/OS adapters、CLIを分離し、利用者入口をpdxに統一します。固定したDocker組版環境を使い、host TeX fallbackを持ちません。

## 帰結

後続GUIも同じapplication APIを呼びます。常駐HTTP/DB/汎用DAG/Compose/独自文書parserは不要です。[構成](../architecture/system.md)と[build契約](../contracts/build.md)を実装上の正本とし、R0ではコード・配布imageを作りません。
