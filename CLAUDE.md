# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

`minimax-cli` は MiniMax AI（platform.minimax.io）の API クライアントとして動作する Rust 製マルチプラットフォーム CLI。
OS のセキュアな保存領域に置いた API トークンを使い、**AI エージェントに認証情報を渡さずに** wav / mp3 などの
読み上げ音声ファイル（TTS）を生成することを目的とする。

- コマンド名は `minimax`（crate 名 `minimax-cli`）
- 現在は環境構築フェーズ。`--version` / `--help` が動く骨格のみで TTS は未実装
- 配布は cargo-dist（`dist`）に委譲し、4 ターゲット（macOS arm64、Linux arm64/x64、Windows x64）を GitHub Release で配る。
  Intel Mac（`x86_64-apple-darwin`）は配布対象外
- crates.io へは publish しない（`publish = false`）。crates.io の `minimax-cli` は無関係な第三者のクレート

## 開発コマンド

```bash
# ビルド
cargo build
cargo build --release

# 実行
cargo run -- --help
cargo run -- --version

# テスト
cargo test --all-features --all-targets
cargo test --all-features --doc
cargo test --test cli                     # 単一テストバイナリ
cargo test version_flag                   # 単一テスト関数

# Lint / フォーマット（CI と同一条件）
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --fix --allow-dirty --all-targets
cargo fmt --all
cargo fmt --all --check

# MSRV 検証（ローカル）
rustup toolchain install 1.85
RUSTUP_TOOLCHAIN=1.85 cargo check --locked --all-targets --all-features

# サプライチェーン
cargo deny --all-features check advisories bans licenses sources

# リリース確認（コマンド名は `dist`。`cargo dist` では呼べない）
dist plan
dist plan --tag=v0.1.0 --output-format=json | jq -r '.announcement_title, .announcement_github_body'
dist build --artifacts=local            # 現在のプラットフォームのみ実ビルド
```

## アーキテクチャ

### クレート構成（`src/`）

- **`main.rs`**: `fn main() -> ExitCode` のみ。`lib.rs` の `main_entry()` を呼ぶだけに保つ
- **`lib.rs`**: 公開 API。テストと将来の生成器はここを経由して呼ぶ（ロジックを二重実装しない）
- **`cli.rs`**: clap の `#[derive(Parser)]`。`--version` は `#[command(version)]` = `env!("CARGO_PKG_VERSION")`
- **`error.rs`**: `thiserror` によるライブラリ層エラー型
- **`logging.rs`**: `tracing-subscriber` 初期化。**出力先は stderr**、タイムスタンプなし

### エラー処理の層分け

- ライブラリ層（`lib.rs` 以下の各モジュール）は `thiserror` で型付きエラーを定義する
- バイナリ層（`main_entry`）は `anyhow` で受けて表示・終了コードに変換する
- `unwrap()` / `expect()` はテストコード以外で使わない

### 出力の方針（認証情報を漏らさないための設計）

- **成果物・機械可読な出力は stdout、ログ・進捗・警告は stderr。** AI エージェントは CLI の stdout を
  そのままコンテキストに取り込むため、stdout に余計なものを出さない
- トークンを stdout に出すサブコマンドは作らない
- `--token <VALUE>` のようなフラグは作らない（プロセス引数は他プロセスから読めるうえ、
  AI エージェントがコマンドを組み立てるとトークンが会話ログに永久に残る）。受け口は stdin のみ
- `println!` の直接使用は `src/main.rs` / `src/lib.rs` の `#![warn(clippy::print_stdout)]` で検出する。
  **`Cargo.toml` の `[lints.clippy]` には置かない**（`[lints]` はテストターゲットにも適用されるため、
  テストに `println!` を 1 つ書いた時点で `clippy --all-targets -- -D warnings` の CI が落ちる）

## 環境変数

- `MINIMAX_LOG`: ログレベル（`EnvFilter` 構文。未設定時は `RUST_LOG`、それも無ければ `-v` の回数で決定）
- 認証情報を環境変数で渡す運用は非推奨。環境変数はサブプロセスに継承されるため、
  Claude Code の Bash ツールから `env` で読める

## テスト構成

- `tests/` は cargo の規約ディレクトリ。**1 ファイル = 1 テストバイナリ**
- `tests/common/mod.rs` に共有ヘルパーを置き、各テストで `mod common;` する
  - **そのバイナリで未使用のヘルパーは dead_code になり `-D warnings` でエラーになるので `#![allow(dead_code)]` が必須**
  - ヘルパーは `pub` ではなく **`pub(crate)`** にする（`mod common;` は private module なので
    `pub fn` にすると `[lints.rust] unreachable_pub` に引っかかる）
- **環境変数はテストで書き換えない。** edition 2024 では `std::env::set_var` / `remove_var` が unsafe で、
  かつ結合テストはスレッド並列実行される。設定解決層が環境変数ソースを引数で受け取る設計にし、
  テストでは空のソースを注入する
- **`std::fs::read_dir` は順序を保証しない。** コレクションを比較する前に安定キーでソートする
- **相対パスは必ず `/` 区切りに正規化してから出力・比較する**（Windows レグでの差分を防ぐ）
- `xtask` ワークスペースは作らない。生成器が必要なら `examples/` に置く

## CI パイプライン

GitHub Actions (`.github/workflows/ci.yml`) — `push: main` と `pull_request` のみ（**タグでは走らせない**）:

- **fmt**: `cargo fmt --all --check`
- **clippy**: `cargo clippy --locked --all-targets --all-features -- -D warnings`
- **test**: ubuntu-24.04 / macos-15 / windows-2025 の 3 OS（配布 4 ターゲットのうち 3 つをネイティブ実行でカバー）
- **msrv**: Rust 1.85 で `cargo check`（`RUSTUP_TOOLCHAIN` は step ではなく **job env** に置く。
  step レベルだと `Swatinem/rust-cache` のキー計算時に効かず、キーと実 toolchain が食い違う）
- **cross-check**: `aarch64-unknown-linux-gnu` の `cargo check --target`
  （test ジョブでカバーされない残り 1 ターゲット。`cargo check` はリンクしないのでクロスリンカ不要）
- **docs**: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- **dist-guard**: `Cargo.toml` に `[profile.dist]` が残っていることを検証

`.github/workflows/audit.yml` — cargo-deny（advisories を別ジョブに分離 + 週次 cron）。
**`paths` フィルタは付けない**（GitHub の paths フィルタは「skip」ではなく「workflow を起動しない」なので、
required status check にすると依存を触らない PR が永久にマージできなくなる）。

`.github/workflows/release.yml` — **`dist generate` の生成物。絶対に手編集しない。**
手編集すると PR の `plan` ジョブが `has out of date contents and needs to be regenerated` でハードエラーになる。
変更したい場合は `dist-workspace.toml` を編集して `dist generate` で再生成する。
同じ理由で **Dependabot の github-actions ecosystem は有効化しない**（bot が release.yml を書き換えると全 PR が赤くなる）。

### ランナーラベルの追随（重要）

`dist` が既定で埋め込むランナーラベルは廃止スケジュール入りのものを含むため、
`dist-workspace.toml` の `[dist.github-custom-runners]` で明示的に上書きしている。
GitHub のランナー廃止告知を追い、期限前にここを更新する責任はこちら側にある
（`cargo-dist-version` の pin は dist 自身の取得を固定するだけで、ランナー廃止からは守らない）。

`x86_64-apple-darwin`（Intel Mac）は**配布対象外**。GitHub Actions の x86_64 macOS イメージ
（`macos-15-intel`）が 2027 年 8 月に終了し、期限付きの前提を抱えることになるため外した。
Intel Mac ではビルド済みバイナリを提供せず、shell installer も失敗する（ソースからのビルドは可能）。

## コードスタイル

- rustfmt: `style_edition = "2024"` / `max_width = 100` / `newline_style = "Unix"`（`rustfmt.toml`）
  - unstable オプション（`imports_granularity`, `group_imports`, `wrap_comments` 等）は書かない
- Clippy: `clippy::all` を deny + 個別 lint を warn（`Cargo.toml [lints.clippy]`）
  - **`clippy::pedantic` は入れない。** CI の `-D warnings` と併用すると公開関数を書くたびに
    `must_use_candidate` / `missing_errors_doc` 等が hard error になる
- `unsafe_code = "forbid"`
- MSRV（`rust-version = "1.85"`）との互換性を維持する。依存の MSRV が上がったらこちらも上げる
- `rust-toolchain.toml` は置かない（dist の release.yml が toolchain を毎回 DL してしまう／CI の MSRV ジョブが無効化される）。
  ローカルは `rustup update` で stable を追随させる
- CLI が利用者に見せる文字列（help・エラーメッセージ）は英語。コード内コメントは日本語

## リリース手順

1. `/prepare-release X.Y.Z` で CHANGELOG の `[Unreleased]` を確定し `Cargo.toml` の version を更新
2. `dist plan --tag=vX.Y.Z --output-format=json | jq -r '.announcement_title, .announcement_github_body'` で
   Release のタイトルと本文を事前確認
3. main へマージ後に `git tag vX.Y.Z && git push --tags`
4. `release.yml` が 5 ターゲットをビルドし GitHub Release を自動作成する

バージョンの真実は **`Cargo.toml` の `version` 1 箇所**（`Cargo.lock` は cargo が自動更新）。
CHANGELOG は `## [X.Y.Z] - YYYY-MM-DD` 形式を厳守する（dist が parse-changelog で抽出する）。
参照リンク定義（`[X.Y.Z]: https://...`）は書かない（Release 本文に混入する）。

## AI-SDD Instructions (v4.0.0)

<!-- sdd-workflow version: "4.0.0" -->

このプロジェクトは AI-SDD（AI駆動仕様駆動開発）ワークフローに従います。

### ドキュメント操作

`.sdd/` ディレクトリ配下のファイルを操作する際は、`.sdd/AI-SDD-PRINCIPLES.md` を参照し、AI-SDDワークフローに準拠してください。

**トリガー条件**:

- `.sdd/` 配下のファイルの読み取りまたは変更
- 新しい仕様書、設計書、要求仕様書の作成
- `.sdd/` ドキュメントを参照する機能の実装

詳細なディレクトリ構造・ファイル命名規則・ドキュメントリンク規約は、`.claude/rules/ai-sdd-instructions.md` を参照してください。
