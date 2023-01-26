# mine

Distributes liquidity mining rewards to token stakers.

## Overview

The Quarry mine program starts with a `Rewarder`. A `Rewarder` corresponds to a reward token e.g. SBR, MNDE.

Once a `Rewarder` is created, we can add `Quarry`s to it. A `Quarry` corresponds to a staking token.

Finally, users/programs can create `Miner`s which allow a user to stake/unstake tokens to a `Quarry` and claim rewards from a `Quarry`.