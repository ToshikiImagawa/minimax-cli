# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `minimax tts` to convert text into a speech audio file via the MiniMax synchronous text-to-speech API. Text comes from exactly one of a positional argument, `--file <PATH>`, or standard input; `--voice`, `--model`, and `--output` are required, `--language` is optional, and `--force` allows overwriting an existing output file. Output format (`mp3`/`wav`/`flac`) is inferred from the `--output` extension. All send-before-charge checks (text validation, output format, existing-file check) run before the API is called, so invalid input is never billed. There is no `--token` flag; the token is always read from the OS credential store
- `minimax auth login` / `auth status` / `auth logout` to register, check, and remove the MiniMax API token in the OS secure credential store. The token is read from standard input only; there is no `--token` flag. `auth status` reports the registered region on stdout; on Linux the token is stored in the kernel keyring and is lost on reboot
- Initial CLI skeleton. `minimax --version` and `minimax --help` work
- Prebuilt binaries for 4 platforms (macOS arm64, Linux arm64/x64, Windows x64) published to GitHub Releases. Intel Mac (`x86_64-apple-darwin`) is not a distribution target; build from source with `cargo install --git https://github.com/ToshikiImagawa/minimax-cli.git` instead
- Shell and PowerShell installers for prebuilt binaries
- Diagnostics and logs go to stderr only, keeping stdout clean for machine-readable output
- HTTP layer log targets (`reqwest`, `hyper`, `hyper_util`, `h2`, `rustls`, and their nested targets) are capped at `warn` on top of the filter resolved from `MINIMAX_LOG` / `RUST_LOG` / `-v`, so no filter value can enable wire-level tracing that would dump `Authorization` headers
- An empty (or whitespace-only) `MINIMAX_LOG` or `RUST_LOG` counts as unset, so it falls through to the next source instead of forcing the level to `error`
