# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial CLI skeleton. `minimax --version` and `minimax --help` work; text-to-speech is not implemented yet
- Prebuilt binaries for 4 platforms (macOS arm64, Linux arm64/x64, Windows x64) published to GitHub Releases. Intel Mac (`x86_64-apple-darwin`) is not a distribution target; build from source with `cargo install --git https://github.com/ToshikiImagawa/minimax-cli.git` instead
- Shell and PowerShell installers for prebuilt binaries
- Diagnostics and logs go to stderr only, keeping stdout clean for machine-readable output
