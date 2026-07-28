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
- `publish = false` は **dist からもバイナリを隠してしまう**ため、`[package.metadata.dist] dist = true`
  が必須（dist 公式が案内している回避方法）。これを消すと `dist plan` が
  `This workspace doesn't have anything for dist to Release!` で失敗する。**絶対に消さない**

## 開発コマンド

```bash
# 前提ツール（初回のみ）
rustup update
cargo install cargo-deny --locked
# dist はコマンド名 `dist`（`cargo dist` では呼べない）。
# dist-workspace.toml の cargo-dist-version と必ず同じバージョンを入れる。
# ズレると `dist generate` の出力が変わり、PR の plan ジョブが out of date でハードエラーになる。
cargo install cargo-dist --locked --version 0.32.0

# ビルド
cargo build
cargo build --release

# 実行
cargo run -- --help
cargo run -- --version

# テスト（CI は全ジョブ --locked）
cargo test --locked --all-features --all-targets
cargo test --locked --all-features --doc
cargo test --test cli                     # 単一テストバイナリ
cargo test version_flag                   # 単一テスト関数

# Lint / フォーマット
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo clippy --fix --allow-dirty --all-targets
cargo fmt --all
cargo fmt --all --check

# ドキュメント（intra-doc link 切れが hard error になる）
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --all-features

# MSRV 検証（ローカル）
rustup toolchain install 1.85
RUSTUP_TOOLCHAIN=1.85 cargo check --locked --all-targets --all-features

# クロスチェック（初回のみ target 追加が必要）
rustup target add aarch64-unknown-linux-gnu
cargo check --locked --all-features --target aarch64-unknown-linux-gnu

# サプライチェーン
cargo deny --all-features check advisories bans licenses sources

# リリース確認（Release 本文のプレビューは「リリース手順」を参照）
dist plan                                 # 配布物の一覧
dist build                                # 既定は host モード。現在のプラットフォームのみ実ビルド
# `dist build --artifacts=local` は dist-workspace.toml の 4 ターゲット全部が対象。
# ローカルでは cargo-xwin / cargo-zigbuild が無く exit 255 で落ちる（CI 専用のモード）
```

**`Cargo.lock` は必ずコミットする。** CI は全ジョブ `--locked` なので、`Cargo.toml` の依存を変えたら
`Cargo.lock` も更新してコミットする（しないと `the lock file needs to be updated` で全ジョブが落ちる）。

## アーキテクチャ

### クレート構成（`src/`）

- **`main.rs`**: `fn main() -> ExitCode` のみ。`lib.rs` の `main_entry()` を呼ぶだけに保つ
- **`lib.rs`**: 公開 API。ロジックはここに置き `main.rs` から呼ぶ（二重実装しない）。サブコマンドの
  ディスパッチは `run()` の match
- **`cli.rs`**: clap の `#[derive(Parser)]`。`--version` は `#[command(version)]` = `env!("CARGO_PKG_VERSION")`
- **`error.rs`**: `thiserror` によるライブラリ層エラー型
- **`logging.rs`**: `tracing-subscriber` 初期化。**出力先は stderr**、タイムスタンプなし

### サブコマンドを追加する手順

1. `src/cli.rs` の `enum Command` に variant を追加する（doc コメントが `--help` に出るので **英語**で書く）
2. `src/lib.rs` の `run()` の match に arm を追加する（match は網羅的なので追加漏れはコンパイルエラーになる）。
   未実装スタブは `anyhow::bail!("the \`xxx\` subcommand is not implemented yet")`（`todo!()` は clippy で hard error）
3. `tests/cli.rs` に結合テストを追加する（終了コードと、診断が stdout に出ないこと）
4. `README.md` と `README.ja.md` の CLI リファレンス表を**両方**更新する
5. `CHANGELOG.md` の `[Unreleased]` に利用者向けの 1 行を追加する

### エラー処理の層分け

- **方針**: ライブラリ層（`cli.rs` / `logging.rs` 等の機能モジュール）は `thiserror` で型付きエラーを定義し、
  バイナリ層（`main_entry`）が `anyhow` で受けて表示・終了コードに変換する
- **現状（環境構築フェーズ）**: `src/error.rs` の `Error` は骨格のみで未使用。`lib.rs::run()` は暫定的に
  `anyhow::Result` を返している。TTS 実装時に `run()` の戻り値を型付きエラーに寄せ、`anyhow` は `main_entry` に閉じる
- `unwrap()` / `expect()` はテストコード以外で使わない。**例外はコンパイル時に決まるリテラルのパースだけ**
  （`src/logging.rs` の `"reqwest=warn".parse().expect("valid directive")`）。実行時入力・I/O・環境依存の値には使わない
  - `clippy::unwrap_used` / `expect_used` は有効化していないため **この規約は機械検出されない**。レビューで見る

### 出力の方針（認証情報を漏らさないための設計）

- **成果物・機械可読な出力は stdout、ログ・進捗・警告は stderr。** AI エージェントは CLI の stdout を
  そのままコンテキストに取り込むため、stdout に余計なものを出さない
- トークンを stdout に出すサブコマンドは作らない
- `--token <VALUE>` のようなフラグは作らない（プロセス引数は他プロセスから読めるうえ、
  AI エージェントがコマンドを組み立てるとトークンが会話ログに永久に残る）。受け口は stdin のみ
- `println!` の直接使用は `src/main.rs` / `src/lib.rs` の `#![warn(clippy::print_stdout)]` で検出する。
  **`Cargo.toml` の `[lints.clippy]` には置かない**（理由は `Cargo.toml` のコメント）

## 環境変数

- `MINIMAX_LOG`: ログレベル（`EnvFilter` 構文。未設定時は `RUST_LOG`、それも無ければ `-v` の回数で決定）
- **エラー表示は `main_entry` の `tracing::error!` 経由なのでログフィルタの影響を受ける。**
  `MINIMAX_LOG=off` / `RUST_LOG=off` を渡すとエラーメッセージが消え、終了コード 1 だけが返る。
  ログを絞る場合は `MINIMAX_LOG=error` 以上を指定する
- 環境変数で解決したフィルタに対し、`src/logging.rs` が**後から** `reqwest` / `hyper` / `hyper_util` / `h2` /
  `rustls` を `warn` に固定する（認証ヘッダのダンプ防止）。HTTP レイヤのトレースが必要なら環境変数ではなく
  `logging.rs` を変更する
- 認証情報を環境変数で渡す運用は非推奨。環境変数はサブプロセスに継承されるため、
  Claude Code の Bash ツールから `env` で読める

## テスト構成

- **結合テストはバイナリを spawn して外形を検証する**（`tests/common/mod.rs` の `bin()` =
  `assert_cmd::Command::cargo_bin("minimax")`。**バイナリ名は `minimax`、crate 名 `minimax-cli` ではない**）。
  stdout / stderr の出し分けや終了コードは lib API を直接呼ぶと検証できない
- 純粋なロジックの単体テストは各モジュール内の `#[cfg(test)] mod tests` に置く
- `tests/` は cargo の規約ディレクトリ。**1 ファイル = 1 テストバイナリ**
- `tests/common/mod.rs` に共有ヘルパーを置き、各テストで `mod common;` する
  - **`#![allow(dead_code)]` + `pub(crate)` が必須**（理由は `tests/common/mod.rs` 冒頭のコメント）
- **環境変数はテストで書き換えない。** edition 2024 では `std::env::set_var` / `remove_var` が unsafe で、
  かつ結合テストはスレッド並列実行される。設定解決層が環境変数ソースを引数で受け取る設計にし、
  テストでは空のソースを注入する
- **`std::fs::read_dir` は順序を保証しない。** コレクションを比較する前に安定キーでソートする
- **相対パスは必ず `/` 区切りに正規化してから出力・比較する**（Windows レグでの差分を防ぐ）
- `xtask` ワークスペースは作らない。生成器が必要なら `examples/` に置く
- まとまった出力（help 全文など）の回帰は **`insta` のスナップショットで固定する**（自前の golden ファイルは作らない）。
  現状 `tests/` は未導入で `predicates` の部分一致のみ。承認には cargo-insta が必要（`cargo install cargo-insta --locked`）
  - **可変値（一時パス・バージョン・タイムスタンプ）は必ず filters / redactions でマスクする**。
    ログにタイムスタンプを出さない（`logging.rs` の `without_time()`）のもこのため
  - `.snap` は `.gitattributes` の `*.snap text eol=lf` 管理下に置く（Windows CI で CRLF になると全部壊れる）

## サプライチェーン（`deny.toml`）

- **TLS は rustls 固定。** `openssl` / `openssl-sys` は `[bans] deny` で禁止しているため、
  `reqwest` を入れるときは必ず `default-features = false` + `rustls-tls`
  （既定 features は native-tls 経由で `openssl-sys` を引き込み、`audit.yml` が PR で落ちる）
- Windows API は `windows` ではなく `windows-sys`（`[bans] deny` で bloat 理由の禁止）
- `[licenses] allow` は **現在の依存グラフに実在する 4 ライセンスのみ**
  （Apache-2.0 / Apache-2.0 WITH LLVM-exception / MIT / Unicode-3.0）。
  依存追加で新しいライセンスが出たらその時点で追記する（先回りして並べると `unused-allowed-license` が warn）
- 依存を git 参照で入れない（`[sources] unknown-git = "deny"`）／バージョンのワイルドカード禁止（`wildcards = "deny"`）
- 配布ターゲットを増減したら `deny.toml` の `[graph] targets` も揃える
  （ここに無いターゲットでしか有効化されない cfg 依存は advisories / licenses / bans の全チェックから漏れる）

## CI パイプライン

GitHub Actions (`.github/workflows/ci.yml`) — `push: main` と `pull_request` のみ（**タグでは走らせない**）:

- **fmt** / **clippy** / **docs**: 内容は「開発コマンド」節と同一（全ジョブ `--locked`）
- **test**: ubuntu-24.04 / macos-15 / windows-2025 の 3 OS（配布 4 ターゲットのうち 3 つをネイティブ実行でカバー）
- **msrv**: Rust 1.85（`RUSTUP_TOOLCHAIN` は step ではなく **job env** に置く。
  step レベルだと `Swatinem/rust-cache` のキー計算時に効かず、キーと実 toolchain が食い違う）
- **cross-check**: test ジョブでカバーされない残り 1 ターゲット（`cargo check` はリンクしないのでクロスリンカ不要）
- **dist-guard**: `Cargo.toml` に `[profile.dist]` が残っていることを検証する。
  **`[package.metadata.dist] dist = true` は検証していない**ので、`Cargo.toml` を整理するときは自分で守る
  （消しても PR は green のまま、次のタグ push で初めて壊れる）

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
  - unstable オプションは書かない（一覧は `rustfmt.toml` 末尾のコメント）
- Clippy: `clippy::all` を deny + 個別 lint を warn（`Cargo.toml [lints.clippy]`）
  - **`clippy::pedantic` は入れない。** CI の `-D warnings` と併用すると公開関数を書くたびに
    `must_use_candidate` / `missing_errors_doc` 等が hard error になる
  - **`-D warnings` で hard error になる主なもの**（`Cargo.toml [lints.clippy]` / `clippy.toml`）:
    - `todo!()` / `dbg!()` は書けない（`todo` / `dbg_macro`）。**未実装のサブコマンドは `todo!()` ではなく
      `anyhow::bail!` でスタブする**（既存の `tts` と同じ形）
    - 変数名 `foo` / `bar` / `baz` / `tmp` は禁止（`clippy.toml` の `disallowed-names`）。一時変数でも使わない
    - `&str` への `.to_string()` は不可 → `.to_owned()`（`str_to_string`）
    - `format!("{}", x)` は不可 → `format!("{x}")`（`uninlined_format_args`）
- `clippy.toml` は **lint レベルではなく設定値専用**（レベルは `Cargo.toml [lints.clippy]` 側）。
  `msrv = "1.85"` / `avoid-breaking-exported-api = false` / `disallowed-names` を置いている
- `Cargo.toml [lints.rust]` は `unsafe_code = "forbid"` / `unused_qualifications = "warn"` /
  `unreachable_pub = "warn"`。CI は `-D warnings` なので warn も落ちる
- MSRV（`rust-version = "1.85"`）との互換性を維持する。依存の MSRV が上がったらこちらも上げる
- `rust-toolchain.toml` は置かない（dist の release.yml が toolchain を毎回 DL してしまう／CI の MSRV ジョブが無効化される）。
  ローカルは `rustup update` で stable を追随させる

## ドキュメント

- README は **`README.md`（英語）と `README.ja.md`（日本語）の 2 本立て。片方だけ更新しない**
- 配布ターゲット / CI ランナーの表は `dist-workspace.toml`・この CLAUDE.md・`README.md`・`README.ja.md` の
  **4 箇所に重複している**。ランナーラベルや配布ターゲットを変えたら 4 箇所すべてを揃える
- CLI のオプション・サブコマンド・環境変数を追加したら両 README の表を更新する（真実は `src/cli.rs`）
- 言語の使い分け:
  - `CHANGELOG.md` は **英語**（GitHub Release 本文にそのまま載る）
  - `README.md` は英語 / `README.ja.md` は日本語
  - `.sdd/` 配下は日本語（`.sdd-config.json` の `lang: ja`）
  - 設定ファイルのコメントとソースの doc コメントは日本語、CLI が利用者に見せる文字列（help・エラー）は英語

## リリース手順

1. `/prepare-release X.Y.Z` で CHANGELOG の `[Unreleased]` を確定し `Cargo.toml` の version を更新
2. `dist plan --tag=vX.Y.Z --output-format=json | jq -r '.announcement_title, .announcement_github_body'` で
   Release のタイトルと本文を事前確認
3. main へマージ後に `git tag vX.Y.Z && git push --tags`

   タグ名は `v` + `Cargo.toml` の `version` と**完全一致**させる（dist はタグから version を逆算する）。
   不一致だと `dist plan` / `release.yml` が `This workspace doesn't have anything for dist to Release!` で落ちる。
   **これは `publish = false` が原因のときと同じエラー文なので、まず `Cargo.toml` の `version` とタグを照合する
   （`[package.metadata.dist] dist = true` を疑うのは後）。**
4. `release.yml` が 4 ターゲットをビルドし GitHub Release を自動作成する

バージョンの真実は **`Cargo.toml` の `version` 1 箇所**（`Cargo.lock` は cargo が自動更新）。
CHANGELOG は `## [X.Y.Z] - YYYY-MM-DD` 形式を厳守する（dist が parse-changelog で抽出する）。
参照リンク定義（`[X.Y.Z]: https://...`）は書かない（Release 本文に混入する）。

## コミット

- このリポジトリは**ローカルの git 設定で author を個人アドレスに固定している**（公開 OSS のため意図的）。
  `git config user.email` を変更したり `--author` / `git -c user.email=...` で上書きしたりしない
  （グローバル設定は会社アドレスなので、上書きすると公開履歴に会社アドレスが残る）

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
