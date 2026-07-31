# Minimax CLI

[![CI](https://github.com/ToshikiImagawa/minimax-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/ToshikiImagawa/minimax-cli/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![clippy](https://img.shields.io/badge/lint-clippy-blue?logo=rust)](https://doc.rust-lang.org/clippy/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](https://github.com/ToshikiImagawa/minimax-cli)

[MiniMax AI](https://platform.minimax.io/) の音声合成 API を使って読み上げ音声ファイルを生成する CLI。API の認証情報は OS のセキュアな保存領域に置いたまま扱い、AI エージェントには渡しません。

[English README](README.md)

> **現状: 一部実装フェーズ。** 認証情報管理（`auth login` / `auth status` / `auth logout`）は動作します。音声合成（`tts`）は未実装です。

## 特徴

- **認証情報が AI エージェントに渡らない。** トークンは CLI プロセス内で OS のセキュアな保存領域から読み出します。`--token` フラグは用意しません。プロセス引数は他プロセスから読める上、AI エージェントがコマンドを組み立てるとトークンが会話ログに永久に残るためです。
- **stdout を汚さない。** 成果物と機械可読な出力は stdout、ログ・進捗・エラーは stderr に出します。AI エージェントは stdout をそのままコンテキストに取り込むため、余計なものを書きません。
- **ログから認証情報を漏らせません。** 指定したフィルタの上に HTTP レイヤの `warn` 打ち止めを重ねるため、`MINIMAX_LOG` や `RUST_LOG` にどんな値を与えても `Authorization` ヘッダが出るようなワイヤレベルのトレースは有効になりません。
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
| `auth login --region <REGION>` | 標準入力から読み取った API トークンを登録する。`--region`（`global` または `china`）は必須。既存の登録は確認無しで上書きする |
| `auth status` | トークンの登録有無と登録済みリージョンを表示する（リージョンを stdout に出力） |
| `auth logout` | 登録済みの API トークンを削除する。未登録でも成功として扱う |
| `help` | サブコマンドの help を表示する |

> [!NOTE]
> Linux ではトークンをカーネルキーリングに保存するため、**再起動すると失われます**。その後は `auth status` が未登録として報告するので、`auth login` を再実行してください。

### 環境変数

| 変数 | 説明 |
|:---|:---|
| `MINIMAX_LOG` | ログレベル（[`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) 構文）。未設定時は `RUST_LOG`、それも無ければ `-v` の回数で決まる。空文字は未設定として扱う |

どちらの変数にも次の 2 つの制約があります。

- **HTTP レイヤの target は `warn` で打ち止めで、引き上げられません。** `MINIMAX_LOG=hyper=trace` や `MINIMAX_LOG=hyper::proto=trace` を指定してもそれらのログは出ません。対象は `reqwest` / `hyper` / `hyper_util` / `h2` / `rustls` で、その配下の target も含みます。どんなフィルタ値でも `Authorization` ヘッダが stderr に出ないようにするためです。ワイヤレベルのトレースが本当に必要な場合は `src/logging.rs` を変更してください。
- **`off` にするとエラーメッセージも消えます。** 診断も同じフィルタを通るため、`MINIMAX_LOG=off`（または `RUST_LOG=off`）では説明のない終了コード 1 だけが残ります。エラーは見えるようにしたうえで他を静かにしたい場合は `MINIMAX_LOG=error` 以上を指定してください。

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
