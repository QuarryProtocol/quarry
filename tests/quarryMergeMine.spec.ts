import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import {
  createMint,
  getOrCreateATAs,
  getTokenAccount,
  sleep,
  SPLToken,
  Token,
  TOKEN_PROGRAM_ID,
  TokenAmount,
  u64,
} from "@saberhq/token-utils";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import BN from "bn.js";
import { expect } from "chai";

import { findMinerAddress, QuarrySDK } from "../src";
import { QuarryMergeMineErrors } from "../src/idls/quarry_merge_mine";
import { createRewarderAndQuarry } from "./quarryUtils";
import { DEFAULT_DECIMALS } from "./utils";
import { makeSDK } from "./workspace";

describe("Quarry Merge Mine", () => {
  let provider: Provider;
  let adminKP: Keypair;
  let minterKP: Keypair;
  let ownerKP: Keypair;

  beforeEach("Initialize", async () => {
    const sdk = makeSDK();
    provider = sdk.provider;

    ownerKP = Keypair.generate();
    minterKP = Keypair.generate();
    adminKP = Keypair.generate();

    const { connection } = provider;
    await connection.confirmTransaction(
      await connection.requestAirdrop(adminKP.publicKey, 10 * LAMPORTS_PER_SOL)
    );
    await connection.confirmTransaction(
      await connection.requestAirdrop(ownerKP.publicKey, 10 * LAMPORTS_PER_SOL)
    );
  });

  it("happy path", async () => {
    const { connection } = provider;

    const stakedTokenMint = await createMint(
      provider,
      minterKP.publicKey,
      DEFAULT_DECIMALS
    );
    const stakedToken = Token.fromMint(stakedTokenMint, DEFAULT_DECIMALS);

    // primary pool
    const primary = await createRewarderAndQuarry({
      connection,
      stakedToken,
      annualRate: new u64(1_000_000000),
    });

    const ownerSDK = QuarrySDK.load({
      provider,
    }).withSigner(ownerKP);
    const {
      tx: initPoolTX,
      key: poolKey,
      replicaToken,
    } = await ownerSDK.mergeMine.newPool({
      primaryMint: stakedTokenMint,
    });
    await expectTX(initPoolTX, "Init pool").to.be.fulfilled;

    // init replica rewarders
    const replicaA = await createRewarderAndQuarry({
      connection,
      stakedToken: replicaToken,
      adminKP: ownerKP,
      annualRate: new u64(1_000_000000),
    });
    const replicaB = await createRewarderAndQuarry({
      connection,
      stakedToken: replicaToken,
      adminKP: ownerKP,
      annualRate: new u64(1_000_000000),
    });

    // init merge miner
    const poolData = await ownerSDK.mergeMine.program.account.mergePool.fetch(
      poolKey
    );
    const { tx: initMMTX, key: mmKey } = await ownerSDK.mergeMine.newMM({
      pool: {
        key: poolKey,
        data: poolData,
      },
      rewarder: primary.rewarder,
      rewardsMint: primary.rewardsToken.mintAccount,
    });
    await expectTX(initMMTX, "Init merge miner").to.be.fulfilled;

    // set up user token accounts
    const { accounts: ownerAccounts, instructions: createUserATAs } =
      await getOrCreateATAs({
        provider,
        mints: {
          staked: stakedTokenMint,
          primaryRewards: primary.rewardsToken.mintAccount,
          replicaARewards: replicaA.rewardsToken.mintAccount,
          replicaBRewards: replicaB.rewardsToken.mintAccount,
        },
        owner: ownerKP.publicKey,
      });
    await expectTX(
      new TransactionEnvelope(
        provider,
        [
          ...createUserATAs,
          SPLToken.createMintToInstruction(
            TOKEN_PROGRAM_ID,
            stakedTokenMint,
            ownerAccounts.staked,
            minterKP.publicKey,
            [],
            1_000_000_000000
          ),
        ],
        [minterKP]
      ),
      "Mint staked token to owner"
    ).to.be.fulfilled;

    const ownerInitialBalance = (
      await getTokenAccount(provider, ownerAccounts.staked)
    ).amount;
    expect(ownerInitialBalance).to.bignumber.eq(
      TokenAmount.parse(stakedToken, "1000000").toU64()
    );

    const stakedAmount = TokenAmount.parse(stakedToken, "100");

    const fetchState = async () => {
      const mmData = (
        await ownerSDK.mergeMine.loadMM({
          mmKey,
        })
      ).mm.data;
      const [replicaAMiner] = await findMinerAddress(replicaA.quarry, mmKey);
      const replicaAData = await ownerSDK.programs.Mine.account.miner
        .fetch(replicaAMiner)
        .catch(() => null);
      const [replicaBMiner] = await findMinerAddress(replicaB.quarry, mmKey);
      const replicaBData = await ownerSDK.programs.Mine.account.miner
        .fetch(replicaBMiner)
        .catch(() => null);
      const walletBalance = (
        await getTokenAccount(provider, ownerAccounts.staked)
      ).amount;
      return {
        primaryBalance: mmData.primaryBalance,
        replicaBalance: mmData.replicaBalance,
        replicaABalance: replicaAData?.balance ?? new BN(0),
        replicaBBalance: replicaBData?.balance ?? new BN(0),
        walletBalance,
      };
    };

    const expectState = async (
      args: {
        primaryBalance: BN;
        replicaBalance: BN;
        replicaABalance: BN;
        replicaBBalance: BN;
        walletBalance: BN;
      },
      msg?: string
    ) => {
      const state = await fetchState();
      const msgFmt = (prop: string) => (msg ? `${prop} (${msg})` : undefined);
      expect(state.primaryBalance, msgFmt("primaryBalance")).to.bignumber.eq(
        args.primaryBalance
      );
      expect(state.replicaBalance, msgFmt("replicaBalance")).to.bignumber.eq(
        args.replicaBalance
      );
      expect(state.replicaABalance, msgFmt("replicaABalance")).to.bignumber.eq(
        args.replicaABalance
      );
      expect(state.replicaBBalance, msgFmt("replicaBBalance")).to.bignumber.eq(
        args.replicaBBalance
      );
      expect(state.walletBalance, msgFmt("walletBalance")).to.bignumber.eq(
        args.walletBalance
      );
    };

    // Initial balances
    await expectState(
      {
        primaryBalance: new BN(0),
        replicaBalance: new BN(0),
        replicaABalance: new BN(0),
        replicaBBalance: new BN(0),
        walletBalance: ownerInitialBalance,
      },
      "initial"
    );

    // deposit into pool
    const mm = await ownerSDK.mergeMine.loadMM({
      mmKey,
    });
    const depositTX = await mm.deposit({
      amount: stakedAmount,
      rewarder: primary.rewarder,
    });
    await expectTX(depositTX, "Deposit into rewarder").to.be.fulfilled;

    const replicaAStakeTX = await mm.stakeReplicaMiner(replicaA.rewarder);
    await expectTX(replicaAStakeTX, "Stake into replica A").to.be.fulfilled;

    // After deposit balances
    await expectState(
      {
        primaryBalance: stakedAmount.toU64(),
        replicaBalance: stakedAmount.toU64(),
        replicaABalance: stakedAmount.toU64(),
        replicaBBalance: new BN(0),
        walletBalance: ownerInitialBalance.sub(stakedAmount.toU64()),
      },
      "after deposit"
    );

    // sleep so we earn some tokens
    await sleep(2_000);

    expect(
      (await getTokenAccount(provider, ownerAccounts.primaryRewards)).amount
    ).to.bignumber.eq("0");
    expect(
      (await getTokenAccount(provider, ownerAccounts.replicaARewards)).amount
    ).to.bignumber.eq("0");
    expect(
      (await getTokenAccount(provider, ownerAccounts.replicaBRewards)).amount
    ).to.bignumber.eq("0");

    // claim primary rewards
    const claimPrimaryTX = await mm.claimPrimaryRewards(primary.rewarder);
    await expectTX(claimPrimaryTX, "Claim primary").to.be.fulfilled;

    // claim replica A rewards
    const claimReplicaATX = await mm.claimReplicaRewards(replicaA.rewarder);
    await expectTX(claimReplicaATX, "Claim replica A").to.be.fulfilled;

    expect(
      (await getTokenAccount(provider, ownerAccounts.primaryRewards)).amount
    ).to.bignumber.not.eq("0");
    expect(
      (await getTokenAccount(provider, ownerAccounts.replicaARewards)).amount
    ).to.bignumber.not.eq("0");
    expect(
      (await getTokenAccount(provider, ownerAccounts.replicaBRewards)).amount,
      "not claimed"
    ).to.bignumber.eq("0");

    // withdraw from merge miner
    const unstakeReplicaATX = await mm.unstakeAllReplica(replicaA.rewarder);
    await expectTX(unstakeReplicaATX, "Unstake Replica A").to.be.fulfilled;
    const withdrawTX = await mm.withdraw({
      amount: stakedAmount,
      rewarder: primary.rewarder,
    });
    await expectTX(withdrawTX, "Withdraw").to.be.fulfilled;

    // After withdraw balances:
    // mm: 0 primary
    // miner: 0 replica
    // owner: initial balance LP
    await expectState(
      {
        primaryBalance: new BN(0),
        replicaBalance: new BN(0),
        replicaABalance: new BN(0),
        replicaBBalance: new BN(0),
        walletBalance: ownerInitialBalance,
      },
      "after withdraw"
    );
  });

  it("Unstake primary with replica balance should error", async () => {
    const { connection } = provider;

    const stakedTokenMint = await createMint(
      provider,
      minterKP.publicKey,
      DEFAULT_DECIMALS
    );
    const stakedToken = Token.fromMint(stakedTokenMint, DEFAULT_DECIMALS);

    // primary pool
    const primary = await createRewarderAndQuarry({
      connection,
      stakedToken,
      annualRate: new u64(1_000_000000),
    });

    const ownerSDK = QuarrySDK.load({
      provider,
    }).withSigner(ownerKP);
    const {
      tx: initPoolTX,
      key: poolKey,
      replicaToken,
    } = await ownerSDK.mergeMine.newPool({
      primaryMint: stakedTokenMint,
    });
    await expectTX(initPoolTX, "Init pool").to.be.fulfilled;

    // init replica rewarders
    const replicaA = await createRewarderAndQuarry({
      connection,
      stakedToken: replicaToken,
      adminKP: ownerKP,
      annualRate: new u64(1_000_000000),
    });

    // init merge miner
    const poolData = await ownerSDK.mergeMine.program.account.mergePool.fetch(
      poolKey
    );
    const { tx: initMMTX, key: mmKey } = await ownerSDK.mergeMine.newMM({
      pool: {
        key: poolKey,
        data: poolData,
      },
      rewarder: primary.rewarder,
      rewardsMint: primary.rewardsToken.mintAccount,
    });
    await expectTX(initMMTX, "Init merge miner").to.be.fulfilled;

    // set up user token accounts
    const { accounts: ownerAccounts, instructions: createUserATAs } =
      await getOrCreateATAs({
        provider,
        mints: {
          staked: stakedTokenMint,
          primaryRewards: primary.rewardsToken.mintAccount,
          replicaARewards: replicaA.rewardsToken.mintAccount,
        },
        owner: ownerKP.publicKey,
      });
    await expectTX(
      new TransactionEnvelope(
        provider,
        [
          ...createUserATAs,
          SPLToken.createMintToInstruction(
            TOKEN_PROGRAM_ID,
            stakedTokenMint,
            ownerAccounts.staked,
            minterKP.publicKey,
            [],
            1_000_000_000000
          ),
        ],
        [minterKP]
      ),
      "Mint staked token to owner"
    ).to.be.fulfilled;

    const stakedAmount = TokenAmount.parse(stakedToken, "100");
    // deposit into pool
    const mm = await ownerSDK.mergeMine.loadMM({
      mmKey,
    });
    const depositTX = await mm.deposit({
      amount: stakedAmount,
      rewarder: primary.rewarder,
    });
    await expectTX(depositTX, "Deposit into rewarder").to.be.fulfilled;

    const replicaAStakeTX = await mm.stakeReplicaMiner(replicaA.rewarder);
    await expectTX(replicaAStakeTX, "Stake into replica A").to.be.fulfilled;

    const withdrawTX = await mm.withdraw({
      amount: stakedAmount,
      rewarder: primary.rewarder,
    });

    try {
      await withdrawTX.confirm();
    } catch (e) {
      const err = e as Error;
      expect(err.message).to.include(
        `0x${QuarryMergeMineErrors.OutstandingReplicaTokens.code.toString(16)}`
      );
    }
  });
});
