# Adrena

## Introduction

Adrena protocol is an open-source implementation of a non-custodial decentralized exchange that supports leveraged trading in a variety of assets.

## Quick start

### Setup Environment

1. Clone the repository from <https://github.com/AdrenaFoundation/adrena-program.git>.
2. Install the latest Solana tools from <https://docs.solana.com/cli/install-solana-cli-tools>. If you already have Solana tools, run `solana-install update` to get the latest compatible version.
3. Install the latest Rust stable from <https://rustup.rs/>. If you already have Rust, run `rustup update` to get the latest version.
4. Install the latest Anchor framework from <https://www.anchor-lang.com/docs/installation>. If you already have Anchor, run `avm update` to get the latest version.

Rustfmt is used to format the code. It requires `nightly` features to be activated:

5. Install `nightly` rust toolchain. <https://rust-lang.github.io/rustup/installation/index.html#installing-nightly>
6. Execute `git config core.hooksPath .githooks` to activate pre-commit hooks.

#### [Optional] Vscode setup

1. Install `rust-analyzer` extension
2. If formatting doesn't work, make sure that `rust-analyzer.rustfmt.extraArgs` is set to `+nightly`

### Build

To build the program run `anchor build` command from the `adrena` directory:

```sh
cd adrena-program
anchor build
```

### Test

Integration tests (Rust) can be started as follows:

```sh
anchor build && RUST_BACKTRACE=1 cargo-test-sbf --features test -- --nocapture
```

Can test a specific part of "adrena/programs/adrena/tests/main.rs" with

```sh
anchor build && RUST_BACKTRACE=1 cargo-test-sbf --features test -- test_integration_part7 --nocapture
```

### Deploy & initialize

See cli/README.md

## Support

If you are experiencing technical difficulties while working with the Adrena codebase, open an issue on [Github](https://github.com/AdrenaFoundation/adrena-program/issues). For more general questions about programming on Solana blockchain use [StackExchange](https://solana.stackexchange.com).

If you find a bug in the code, you can raise an issue on [Github](https://github.com/AdrenaFoundation/adrena-program/issues). But if this is a security issue, please don't disclose it on Github or in public channels. Send information to adrena.orex@gmail.com or adrena.corto@gmail.com instead.
