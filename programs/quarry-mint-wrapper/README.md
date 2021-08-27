# mint-wrapper

Mints tokens to authorized accounts.

## Description

The `mint-wrapper` program wraps a token mint and authorizes specific accounts to mint tokens up to given allowances.

The `mint-wrapper` also enforces a hard cap of a token.

Within the Quarry protocol, this should be used to prevent the `Rewarder` from over-issuing tokens.

This can also be used for several other use cases, including but not limited to:

- Allocating funds to a DAO
- Allocating team lockups

If you're building a use case, please [get in touch with us](mailto:team@quarry.so)!

## Roadmap

Future improvements may include:

- Allowing transfer of the `mint_authority` to a different address
