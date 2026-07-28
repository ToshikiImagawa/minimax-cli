---
name: prepare-release
description: "minimax-cli のリリース準備。CHANGELOG.md の [Unreleased] を確定し、Cargo.toml のバージョンを更新して、dist が生成する Release ノートを事前確認する。"
version: 1.0.0
license: MIT
user-invocable: true
argument-hint: "<version>"
allowed-tools: Read, Edit, Glob, Grep, Bash
---

# Prepare Release - リリース準備スキル

`CHANGELOG.md` を更新し、`Cargo.toml` のバージョンを更新するリリース準備スキル。

**ハイブリッド方式**: `[Unreleased]` セクションに既存内容があればそれを活用し、git 変更履歴から不足分を補完する。

## Input

$ARGUMENTS

バージョン番号を引数として受け取る（`v` プレフィックスなし）。

### Input Examples

```
/prepare-release 0.2.0
/prepare-release 1.0.0-alpha
```

### Validation

- 引数が空の場合はエラー終了し、使用例を表示する
- セマンティックバージョニング形式（`X.Y.Z` または `X.Y.Z-prerelease`）であることを確認する

## Target Files

### CHANGELOG File

| ファイル | 言語 |
|:---|:---|
| `CHANGELOG.md` | English |

### Version Manifest Files

| ファイル | フィールド |
|:---|:---|
| `Cargo.toml` | `version` |

**バージョンの真実は `Cargo.toml` の `version` 1 箇所のみ。**

- `Cargo.lock` は `cargo` が自動更新する（手で書き換えない。ただしコミットに含める）
- `--version` の出力は `#[command(version)]` = `env!("CARGO_PKG_VERSION")` 経由なので、**テストコードにバージョン文字列を書かない**（`tests/cli.rs` は `env!("CARGO_PKG_VERSION")` と比較している）

## Processing Flow

### Step 1: Validate Version Argument

1. `$ARGUMENTS` からバージョン番号をパースする
2. セマンティックバージョニング形式を検証する
3. 不正な場合はエラーメッセージと使用例を表示して終了する

### Step 2: Detect Previous Release

1. `git tag --list 'v*' --sort=-version:refname` で最新のリリースタグを取得する
2. タグが存在しない場合は初回リリースとして扱う
3. 比較基点を記録する（例: `v0.1.0`）

### Step 3: Read Current [Unreleased] Content

`CHANGELOG.md` を Read し、`## [Unreleased]` セクションの内容を抽出する。

- **内容あり**: 既存エントリをベースとして保持する
- **内容なし**: Step 4 で全エントリを生成する

### Step 4: Analyze Git Changes

前回タグから HEAD までの変更を分析する。

```bash
git log <previous-tag>..HEAD --oneline --no-merges
git diff <previous-tag>..HEAD --stat
```

変更内容を以下のカテゴリに分類する。

| Category | 判定基準 |
|:---|:---|
| Breaking Changes | 互換性を破る変更、CLI の引数・出力形式の変更 |
| Added | 新機能、新サブコマンド、新オプション |
| Changed | 既存機能の変更、リファクタリング |
| Fixed | バグ修正 |
| Removed | 機能やオプションの削除 |

### Step 5: Generate / Supplement CHANGELOG Entries

**重要: CHANGELOG は利用者向けである。**

以下の変更は CHANGELOG に**含めない**:

- CI/CD ワークフロー（`.github/workflows/`）の追加・変更
- テストコード（`tests/`）の追加・変更
- `.claude/skills/` 配下の開発者向けスキル
- `.gitignore`、`.gitattributes`、PR テンプレート等のリポジトリ管理ファイル
- `rustfmt.toml` / `clippy.toml` / `deny.toml` の設定変更
- `dist-workspace.toml` の変更（ただし**配布ターゲットの増減は含める**。利用者に影響するため）
- その他、バイナリを入れたユーザーに影響しない変更

以下の変更は CHANGELOG に**含める**:

- `src/` 配下のコード変更（サブコマンド、機能、バグ修正）
- MSRV（`rust-version`）の引き上げ（ソースからビルドする利用者に影響する）
- 依存関係の変更でバイナリの挙動が変わるもの
- 配布ターゲットの追加・削除

**記述スタイル（既存 CHANGELOG に準拠）:**

- 各エントリは `- **対象名** - 変更内容の要約` 形式または `- 変更内容の要約` 形式
- サブ項目はインデント 2 スペースで `- 詳細内容`
- カテゴリは `###` ヘッダーで分類（Added, Changed, Fixed, Removed, Breaking Changes）

### Step 6: Update CHANGELOG File

1. `## [Unreleased]` セクションの既存内容をクリアする（**`## [Unreleased]` 見出し自体は残す**）
2. その直後に空行を挟んで新バージョンセクションを挿入する:
   ```
   ## [VERSION] - YYYY-MM-DD
   ```
3. 日付は実行日（`date +%Y-%m-%d` で取得）を使用する
4. Step 5 で生成/統合したエントリを配置する

**書式の厳守（dist が解析する）:**

- dist は内部で `taiki-e/parse-changelog` を使い `## [X.Y.Z] - YYYY-MM-DD` セクションを抽出し、
  GitHub Release のタイトルと本文にする
- **`## vX.Y.Z - ...` と書くとタイトルが `vX.Y.Z - ...` になる。必ず `## [X.Y.Z] - YYYY-MM-DD` に固定する**
- **参照リンク定義（`[X.Y.Z]: https://...`）は書かない**（Release 本文にそのまま混入する）
- 該当バージョンのセクションが無くても CI は落ちず、タイトルがタグ名にフォールバックして
  Release Notes 節が消えるだけ（＝**サイレントに劣化する**）。Step 8 で必ず確認する

### Step 7: Update Version Manifest

`Cargo.toml` の `version` を新バージョンに更新する。

```bash
# 更新後の整合性チェック（.packages[0] は使わない。
# 将来ワークスペース化したときに別パッケージを指すため）
cargo metadata --format-version 1 --no-deps \
  | jq -r '.packages[] | select(.name=="minimax-cli") | .version'

# Cargo.lock を追随させる（差分をコミットに含める）
cargo update --workspace
```

### Step 8: Verify the Generated Release Notes

**タグを打つ前に、dist が生成する Release のタイトルと本文を必ず確認する。**

```bash
dist plan --tag=vVERSION --output-format=json \
  | jq -r '.announcement_title, .announcement_github_body'
```

- タイトルが `VERSION - YYYY-MM-DD` になっていること（`vVERSION - ...` なら CHANGELOG の見出し書式ミス）
- 本文に `## Release Notes` として当該バージョンのエントリが入っていること
- 参照リンク定義が混入していないこと

併せて配布物の一覧も確認する。

```bash
dist plan
```

### Step 9: Summary

更新結果のサマリーを表示する。

## Output Format

```
## Release Preparation Complete

**Version**: OLD_VERSION → NEW_VERSION
**Date**: YYYY-MM-DD

### CHANGELOG Updates

| Category | Entries | Source |
|:---|:---|:---|
| Added | N | existing / generated / mixed |
| Changed | N | ... |
| Fixed | N | ... |

### Updated Files

- [ ] `CHANGELOG.md`
- [ ] `Cargo.toml`
- [ ] `Cargo.lock`

### Release Notes Preview

（dist plan --tag の出力から抜粋）

### Next Steps

1. 変更内容をレビューする
2. コミットする: `git add -A && git commit -m "[add] v{VERSION} リリース準備"`
3. main へマージする
4. タグを作成する: `git tag v{VERSION}`
5. プッシュする: `git push origin main --tags`
6. GitHub Actions の Release ワークフローが自動実行される
```

## Notes

- このスキルは CHANGELOG の **生成・編集** と **バージョン更新** を行う。コミットやタグ作成は行わない
- **タグ push が `release.yml` を発火させる**ため、タグ作成は main へのマージ後に慎重に行う
- 生成されたエントリは必ずユーザーにレビューを促す
- プレリリースバージョン（`-alpha`, `-rc.1` 等）もサポートする。バージョンに `-` を含むと
  dist が GitHub Release に `--prerelease` を付ける
- `.github/workflows/release.yml` は dist の生成物。**このスキルからも絶対に編集しない**
