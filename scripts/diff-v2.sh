#!/usr/bin/env bash

# Helper script for checking to see that instruction handlers have
# isomorphic logic.

cd $(dirname $0)/..

diff --color programs/quarry-mine/src/instructions/new_rewarder.rs \
    programs/quarry-mine/src/instructions/new_rewarder_v2.rs

diff --color programs/quarry-mine/src/instructions/claim_rewards.rs \
    programs/quarry-mine/src/instructions/claim_rewards_v2.rs

diff --color programs/quarry-mine/src/instructions/create_quarry.rs \
    programs/quarry-mine/src/instructions/create_quarry_v2.rs

diff --color programs/quarry-operator/src/instructions/delegate_create_quarry.rs \
    programs/quarry-operator/src/instructions/delegate_create_quarry_v2.rs
