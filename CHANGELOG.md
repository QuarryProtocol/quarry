# Quarry Protocol Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased Changes

### Features

- Add `quarry_mine::claim_rewards_v2` instruction, which reduces the required accounts for claiming rewards by 2 (64 bytes).
- Add `quarry_mine::create_quarry_v2` instruction, which reduces the required accounts for creating a new quarry by 1 (32 bytes).

### Breaking

- Rename `stake` to `claim` in `quarry_mine::claim_rewards`.
- Rename `Miner.quarry_key` to `Miner.quarry` in `quarry_mine`.

## [v4.2.1]

### Features

- Update to Anchor v0.24.
- Add support for Neodyme's [security.txt](https://github.com/neodyme-labs/solana-security-txt) standard.

## [v4.2.0]

### Features

- Publicly release Soteria audit code changes.

## [v4.1.0]

### Features

- Allow rescuing stuck tokens from Quarry mines ([#454](https://github.com/QuarryProtocol/quarry/pull/454)).

## [v4.0.0]

### Breaking

- Upgrade to Anchor v0.23.0 ([#447](https://github.com/QuarryProtocol/quarry/pull/447)).

## [v3.0.0]

### Breaking

- Upgrade to Anchor v0.22.0 ([#409](https://github.com/QuarryProtocol/quarry/pull/409)).

## [v2.0.1]

Fixed Cargo.toml dependency references.

## [v2.0.0]

### Fixes

- Upgrade to Vipers v1.6 ([#397](https://github.com/QuarryProtocol/quarry/pull/397)).

### Breaking

- Upgrade to Anchor v0.21.0 ([#397](https://github.com/QuarryProtocol/quarry/pull/397)).
