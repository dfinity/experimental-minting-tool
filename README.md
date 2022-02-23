# Minting Tool

## What this is

This is an (experimental) minting tool for simple NFTs for [DIP721] NFT canisters on the [Internet Computer][IC] that support the minting operation.

## What this is not

The minting tool only knows how to mint to existing NFT canisters; it does not provide one itself. An example NFT canister can be found in the [examples] repository.

## Installation

To install the minting tool you will need an installation of [Rust], updated at least to 1.58.0. Rust is usually installed via [Rustup], and there may be [extra steps depending on your operating system][instructions].

With Rust installed, run the following command:

```sh
cargo install --git https://github.com/dfinity/experimental-minting-tool
```

This will install the minting tool into your `PATH`, accessible via the `minting-tool` command.

See `minting-tool --help` for usage details.

[DIP721]: https://github.com/Psychedelic/DIP721
[IC]: https://smartcontracts.org
[examples]: https://github.com/dfinity/examples
[Rust]: https://rust-lang.org
[Rustup]: https://rustup.rs
[instructions]: https://doc.rust-lang.org/stable/book/ch01-01-installation.html
