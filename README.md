# Minimax CLI

[![CI](https://github.com/ToshikiImagawa/minimax-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/ToshikiImagawa/minimax-cli/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![clippy](https://img.shields.io/badge/lint-clippy-blue?logo=rust)](https://doc.rust-lang.org/clippy/)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey)](https://github.com/ToshikiImagawa/minimax-cli)

Generate speech audio files with the [MiniMax AI](https://platform.minimax.io/) text-to-speech API. Your API credentials stay in the OS secure store and are never handed to an AI agent.

[日本語版 README](README.ja.md)

> **Status: scaffolding phase.** Only `--version` and `--help` work today. Text-to-speech is not implemented yet.

## Features

- **Credentials never reach AI agents.** The token is read from the OS secure store inside the CLI process. There is no `--token` flag, because process arguments are readable by other processes and an AI agent that builds the command would leave the token in its conversation log forever.
- **stdout stays clean.** Artifacts and machine-readable output go to stdout; logs, progress, and errors go to stderr. AI agents ingest stdout directly, so nothing extra is written there.
- **Logs cannot be made to leak credentials.** HTTP layer targets are capped at `warn` on top of whatever filter you set, so no `MINIMAX_LOG` or `RUST_LOG` value can enable wire-level tracing that would dump `Authorization` headers.
- **Prebuilt binaries for 4 platforms**, published to GitHub Releases with shell and PowerShell installers.
- **No runtime dependencies.** A single binary.

## Requirements

- A MiniMax API key from [platform.minimax.io](https://platform.minimax.io/)
- Rust 1.85 or later (edition 2024) — only when building from source

## Installation

> [!NOTE]
> No release has been published yet, so the prebuilt binaries and installers below are not available. Use **Build from source** for now.

### Install prebuilt binaries via shell script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ToshikiImagawa/minimax-cli/releases/latest/download/minimax-cli-installer.sh | sh
```

### Install prebuilt binaries via PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ToshikiImagawa/minimax-cli/releases/latest/download/minimax-cli-installer.ps1 | iex"
```

### Build from source

```sh
cargo install --git https://github.com/ToshikiImagawa/minimax-cli.git
```

> [!IMPORTANT]
> This crate is **not** published to crates.io. The `minimax-cli` crate on crates.io is an unrelated third-party package, so **do not run `cargo install minimax-cli` or `cargo binstall minimax-cli`** — you would install someone else's binary.

## Usage

```sh
minimax --help
minimax --version
```

Running `minimax` with no arguments prints the help text.

## CLI Reference

### Options

| Option | Description |
|:---|:---|
| `-v`, `--verbose` | Increase log verbosity (`-v` for info, `-vv` for debug) |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

### Commands

| Command | Description |
|:---|:---|
| `tts <TEXT>` | Convert text into a speech audio file (**not implemented yet**) |
| `help` | Print help for a subcommand |

### Environment Variables

| Variable | Description |
|:---|:---|
| `MINIMAX_LOG` | Log level in [`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html) syntax. Falls back to `RUST_LOG`, then to the `-v` count. An empty value counts as unset |

Two constraints apply to both variables:

- **HTTP layer targets are capped at `warn` and cannot be raised.** `MINIMAX_LOG=hyper=trace` and `MINIMAX_LOG=hyper::proto=trace` will not turn those logs on. The cap covers `reqwest`, `hyper`, `hyper_util`, `h2`, and `rustls` — including nested targets — so that no filter value can dump `Authorization` headers to stderr. Edit `src/logging.rs` if you genuinely need wire-level tracing.
- **`off` also silences error messages.** Diagnostics are emitted through the same filter, so `MINIMAX_LOG=off` (or `RUST_LOG=off`) leaves only exit code 1 with no explanation. Use `MINIMAX_LOG=error` or higher to keep errors visible while quieting the rest.

Passing credentials through environment variables is discouraged: environment variables are inherited by child processes, so an AI agent running a shell tool can read them.

## Supported Platforms

| Target | CI runner | Notes |
|:---|:---|:---|
| `aarch64-apple-darwin` | `macos-15` | Apple Silicon |
| `x86_64-unknown-linux-gnu` | `ubuntu-24.04` | glibc |
| `aarch64-unknown-linux-gnu` | `ubuntu-24.04-arm` | glibc |
| `x86_64-pc-windows-msvc` | `windows-2025` | |

> [!NOTE]
> macOS binaries are **not** code-signed or notarized. On first launch, macOS Gatekeeper will block the binary. Right-click the binary and choose **Open**, or run `xattr -d com.apple.quarantine <path>`.

## Development

### Setup

```sh
rustup update
cargo build
```

### Test

```sh
cargo test --all-features --all-targets
cargo test --all-features --doc
```

### Lint & Format

```sh
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo deny --all-features check advisories bans licenses sources
```

### Build

```sh
cargo build --release
```

### Release

Releases are handled by [cargo-dist](https://github.com/axodotdev/cargo-dist). `.github/workflows/release.yml` is a generated file and must never be edited by hand — change `dist-workspace.toml` and run `dist generate` instead.

```sh
dist plan
git tag vX.Y.Z && git push --tags
```

## License

MIT. See [LICENSE](LICENSE).
