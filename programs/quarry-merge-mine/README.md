# quarry-merge-mine

**WARNING: this is beta software. Do not use in production.**

Allows mining multiple quarries simultaneously.

## Overview

The Quarry merge mine program works by defining a `MergePool`, which is a pool of tokens associated with a staked mint, and a `MergeMiner`, which is a user's association with a `MergePool`.

A merge miner can stake two types of mints:

- Primary, the underlying staked token.
- Replica, which can only be minted for a pool if there are enough primary tokens.

There can be an unlimited number of Replica tokens minted, but a merge miner may only mint + stake Replica tokens if it has a corresponding amount of Primary tokens.
