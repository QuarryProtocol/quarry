# `quarry-operator`

Delegates Quarry Rewarder authority roles.

This program defines four roles:

- `admin`, which can update the three authorized roles.
- `rate_setter`, which can modify rates.
- `quarry_creator`, which can create new quarries.
- `share_allocator`, which can choose the number of rewards shares each quarry receives.

## Usage

To use this program

1. Generate the PDA of the Operator.
2. Set the rewarder authority via `quarry_mine::transfer_authority`.
3. Create the Operator and accept the authority via `quarry_operator::create_operator`.
