# ⛏ Quarry

[![License](https://img.shields.io/badge/license-AGPL%203.0-blue)](https://github.com/QuarryProtocol/quarry/blob/master/LICENSE)
[![Build Status](https://img.shields.io/github/workflow/status/QuarryProtocol/quarry/E2E/master)](https://github.com/QuarryProtocol/quarry/actions/workflows/programs-e2e.yml?query=branch%3Amaster)
[![Contributors](https://img.shields.io/github/contributors/QuarryProtocol/quarry)](https://github.com/QuarryProtocol/quarry/graphs/contributors)

<p align="center">
    <img src="/images/banner.png" />
</p>

<p align="center">
    An open protocol for launching liquidity mining programs on Solana.
</p>

## Background

Quarry was built with the intention of helping more Solana projects launch on-chain liquidity mining programs. It is currently standard for projects to manually send tokens to addresses-- while this is better than no distribution, it would be much better for the growth of the ecosystem if liquidity mining programs were composable and enforceable on-chain.

## Audit

Quarry Protocol has been audited by [Quantstamp](https://quantstamp.com/). View the audit report
[here](https://github.com/QuarryProtocol/quarry/blob/master/audit/quantstamp.pdf).

## Packages

| Package                      | Description                                           | Version                                                                                                                         | Docs                                                                                             |
| :--------------------------- | :---------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------ | :----------------------------------------------------------------------------------------------- |
| `quarry-merge-mine`          | Mines multiple quarries at the same time              | [![Crates.io](https://img.shields.io/crates/v/quarry-merge-mine)](https://crates.io/crates/quarry-merge-mine)                   | [![Docs.rs](https://docs.rs/quarry-merge-mine/badge.svg)](https://docs.rs/quarry-merge-mine)     |
| `quarry-mine`                | Distributes liquidity mining rewards to token stakers | [![crates](https://img.shields.io/crates/v/quarry-mine)](https://crates.io/crates/quarry-mine)                                  | [![Docs.rs](https://docs.rs/quarry-mine/badge.svg)](https://docs.rs/quarry-mine)                 |
| `quarry-mint-wrapper`        | Mints tokens to authorized accounts                   | [![Crates.io](https://img.shields.io/crates/v/quarry-mint-wrapper)](https://crates.io/crates/quarry-mint-wrapper)               | [![Docs.rs](https://docs.rs/quarry-mint-wrapper/badge.svg)](https://docs.rs/quarry-mint-wrapper) |
| `quarry-operator`            | Delegates Quarry Rewarder authority roles.            | [![crates](https://img.shields.io/crates/v/quarry-operator)](https://crates.io/crates/quarry-operator)                          | [![Docs.rs](https://docs.rs/quarry-operator/badge.svg)](https://docs.rs/quarry-operator)         |
| `quarry-redeemer`            | Redeems one token for another                         | [![crates](https://img.shields.io/crates/v/quarry-redeemer)](https://crates.io/crates/quarry-redeemer)                          | [![Docs.rs](https://docs.rs/quarry-redeemer/badge.svg)](https://docs.rs/quarry-redeemer)         |
| `quarry-registry`            | Registry to index all quarries of a rewarder.         | [![crates](https://img.shields.io/crates/v/quarry-registry)](https://crates.io/crates/quarry-registry)                          | [![Docs.rs](https://docs.rs/quarry-registry/badge.svg)](https://docs.rs/quarry-registry)         |
| `@quarryprotocol/quarry-sdk` | TypeScript SDK for Quarry                             | [![npm](https://img.shields.io/npm/v/@quarryprotocol/quarry-sdk.svg)](https://www.npmjs.com/package/@quarryprotocol/quarry-sdk) | [![Docs](https://img.shields.io/badge/docs-typedoc-blue)](https://docs.quarry.so/ts/)            |

## Addresses

Program addresses are the same on devnet, testnet, and mainnet-beta.

- MergeMine: [`QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto`](https://explorer.solana.com/address/QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto)
- Mine: [`QMNeHCGYnLVDn1icRAfQZpjPLBNkfGbSKRB83G5d8KB`](https://explorer.solana.com/address/QMNeHCGYnLVDn1icRAfQZpjPLBNkfGbSKRB83G5d8KB)
- MintWrapper: [`QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV`](https://explorer.solana.com/address/QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV)
- Operator: [`QoP6NfrQbaGnccXQrMLUkog2tQZ4C1RFgJcwDnT8Kmz`](https://explorer.solana.com/address/QoP6NfrQbaGnccXQrMLUkog2tQZ4C1RFgJcwDnT8Kmz)
- Redeemer: [`QRDxhMw1P2NEfiw5mYXG79bwfgHTdasY2xNP76XSea9`](https://explorer.solana.com/address/QRDxhMw1P2NEfiw5mYXG79bwfgHTdasY2xNP76XSea9)
- Registry: [`QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc`](https://explorer.solana.com/address/QREGBnEj9Sa5uR91AV8u3FxThgP5ZCvdZUW2bHAkfNc)

## Documentation

Documentation is a work in progress. For now, one should read [the end-to-end tests of the SDK](/tests/mintWrapper.spec.ts).

We soon plan on releasing a React library to make it easy to integrate Quarry with your frontend.

## License

Quarry Protocol is licensed under the GNU Affero General Public License v3.0.

In short, this means that any changes to this code must be made open source and available under the AGPL-v3.0 license, even if only used privately. If you have a need to use this program and cannot respect the terms of the license, please message us our legal team directly at [legal@quarry.so](mailto:legal@quarry.so).
