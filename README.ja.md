# Minimax CLI

[![CI](https://github.com/ToshikiImagawa/minimax-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/ToshikiImagawa/minimax-cli/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![clippy](https://img.shields.io/badge/lint-clippy-blue?logo=rust)](https://doc.rust-lang.org/clippy/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](https://github.com/ToshikiImagawa/minimax-cli)

[MiniMax AI](https://platform.minimax.io/) の音声合成 API を使って読み上げ音声ファイルを生成する CLI。API の認証情報は OS のセキュアな保存領域に置いたまま扱い、AI エージェントには渡しません。

[English README](README.md)

> **現状: 環境構築フェーズ。** 現時点で動作するのは `--version` と `--help` のみで、音声合成は未実装です。

## 特徴

- **認証情報が AI エージェントに渡らない。** トークンは CLI プロセス内で OS のセキュアな保存領域から読み出します。`--token` フラグは用意しません。プロセス引数は他プロセスから読める上、AI エージェントがコマンドを組み立てるとトークンが会話ログに永久に残るためです。
- **stdout を汚さない。** 成果物と機械可読な出力は stdout、ログ・進捗・エラーは stderr に出します。AI エージェントは stdout をそのままコンテキストに取り込むため、余計なものを書きません。
- **4 プラットフォーム向けのビルド済みバイナリ**を GitHub Release で配布します（シェル / PowerShell インストーラ付き）。
- **実行時依存なし。** 単一バイナリです。

## 動作要件

- [platform.minimax.io](https://platform.minimax.io/) で取得した MiniMax の API キー
- Rust 1.85 以降（edition 2024）— ソースからビルドする場合のみ

## インストール

> [!NOTE]
> まだリリースを公開していないため、以下のビルド済みバイナリとインストーラは利用できません。当面は「**ソースからビルド**」を使ってください。

### シェルスクリプトでビルド済みバイナリをインストール

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ToshikiImagawa/minimax-cli/releases/latest/download/minimax-cli-installer.sh | sh
```

### PowerShell でビルド済みバイナリをインストール

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ToshikiImagawa/minimax-cli/releases/latest/download/minimax-cli-installer.ps1 | iex"
```

### ソースからビルド

```sh
cargo install --git https://github.com/ToshikiImagawa/minimax-cli.git
```

> [!IMPORTANT]
> このクレートは crates.io に publish していません。crates.io の `minimax-cli` は**無関係な第三者のパッケージ**です。`cargo install minimax-cli` や `cargo binstall minimax-cli` を実行すると別人のバイナリが入るため、**使用しないでください**。

## 使い方

```sh
minimax --help
minimax --version
```

引数なしで `minimax` を実行すると help を表示します。

## CLI リファレンス

### オプション

| オプション | 説明 |
|:---|:---|
| `-v`, `--verbose` | ログ出力を詳細にする（`-v` で info、`-vv` で debug） |
| `-h`, `--help` | help を表示する |
| `-V`, `--version` | バージョンを表示する |

### コマンド

| コマンド | 説明 |
|:---|:---|
| `tts <TEXT>` | テキストを読み上げ音声ファイルに変換する（**未実装**） |
| `help` | サブコマンドの help を表示する |

### 環境変数

| 変数 | 説明 |
|:---|:---|
| `MINIMAX_LOG` | ログレベル（[`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) 構文）。未設定時は `RUST_LOG`、それも無ければ `-v` の回数で決まる |

認証情報を環境変数で渡す運用は非推奨です。環境変数は子プロセスに継承されるため、AI エージェントがシェルツールを実行すると読み取れてしまいます。

## 対応プラットフォーム

| ターゲット | CI ランナー | 備考 |
|:---|:---|:---|
| `aarch64-apple-darwin` | `macos-15` | Apple Silicon |
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` | glibc |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` | glibc |
| `x86_64-pc-windows-msvc` | `windows-2025` | |

> [!NOTE]
> macOS 向けバイナリには**署名・公証をしていません**。初回起動時に Gatekeeper がブロックするため、バイナリを右クリックして「開く」を選ぶか、`xattr -d com.apple.quarantine <path>` を実行してください。

## 開発

### セットアップ

```sh
rustup update
cargo build
```

### テスト

```sh
cargo test --all-features --all-targets
cargo test --all-features --doc
```

### Lint / フォーマット

```sh
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo deny --all-features check advisories bans licenses sources
```

### ビルド

```sh
cargo build --release
```

### リリース

リリースは [cargo-dist](https://github.com/axodotdev/cargo-dist) が担います。`.github/workflows/release.yml` は生成物なので**絶対に手編集しないでください**。変更したい場合は `dist-workspace.toml` を編集して `dist generate` で再生成します。

```sh
dist plan
git tag vX.Y.Z && git push --tags
```

## ライセンス

MIT。[LICENSE](LICENSE) を参照してください。
