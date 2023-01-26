import { expectTX, expectTXTable } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import {
  createATAInstruction,
  createMint,
  createTokenAccount,
  getATAAddress,
  getOrCreateATA,
  getOrCreateATAs,
  getTokenAccount,
  sleep,
  SPLToken,
  Token,
  TOKEN_PROGRAM_ID,
  TokenAmount,
  u64,
} from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import BN from "bn.js";
import { expect } from "chai";
import invariant from "tiny-invariant";

import { findMergeMinerAddress, findMinerAddress, QuarrySDK } from "../src";
import { QuarryMergeMineErrors } from "../src/idls/quarry_merge_mine";
import type { RewarderAndQuarry } from "./quarryUtils";
import { createRewarderAndQuarry } from "./quarryUtils";
import { DEFAULT_DECIMALS } from "./utils";
import { makeSDK } from "./workspace";

describe("Quarry Merge Mine", () => {
  let provider: Provider;
  let adminKP: Keypair;
  let minterKP: Keypair;
  let ownerKP: Keypair;
  let stakedToken: Token;
  let primary: RewarderAndQuarry;

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

    const stakedTokenMint = await createMint(
      provider,
      minterKP.publicKey,
      DEFAULT_DECIMALS
    );
    stakedToken = Token.fromMint(stakedTokenMint, DEFAULT_DECIMALS);

    // primary pool
    primary = await createRewarderAndQuarry({
      connection,
      stakedToken,
      annualRate: new u64(1_000_000000),
    });
  });

  describe("MergeMiner SDK", () => {
    it("happy path", async () => {
      const { connection } = provider;

      const ownerSDK = QuarrySDK.load({
        provider,
      }).withSigner(ownerKP);
      const {
        tx: initPoolTX,
        key: poolKey,
        replicaToken,
      } = await ownerSDK.mergeMine.newPool({
        primaryMint: stakedToken.mintAccount,
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
            staked: stakedToken.mintAccount,
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
              stakedToken.mintAccount,
              ownerAccounts.staked,
              minterKP.publicKey,
              [],
              1_000_000_000_000
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
        expect(
          state.replicaABalance,
          msgFmt("replicaABalance")
        ).to.bignumber.eq(args.replicaABalance);
        expect(
          state.replicaBBalance,
          msgFmt("replicaBBalance")
        ).to.bignumber.eq(args.replicaBBalance);
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
      const claimPrimary = await mm.claimPrimaryRewards(primary.rewarder);
      const { ataIXs: claimPrimaryATATx, tx: claimPrimaryTx } =
        claimPrimary.splitATAIXs();
      await expectTX(claimPrimaryATATx, "Create ATA accounts for primary claim")
        .to.be.fulfilled;
      await expectTX(claimPrimaryTx, "Claim primary").to.be.fulfilled;

      // claim replica A rewards
      const claimReplicaA = await mm.claimReplicaRewards(replicaA.rewarder);
      const { ataIXs: claimReplicaATATx, tx: claimReplicaATX } =
        claimReplicaA.splitATAIXs();
      await expectTX(claimReplicaATATx, "Create ATA accounts for replica claim")
        .to.be.fulfilled;
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
          `0x${QuarryMergeMineErrors.OutstandingReplicaTokens.code.toString(
            16
          )}`
        );
      }
    });

    describe("Rescue Tokens", () => {
      let rescueMint: PublicKey;
      let minerKey: PublicKey;
      let minerATAKey: PublicKey;
      let mergePoolKey: PublicKey;
      let mergeMinerKey: PublicKey;

      let replicaToken: Token;
      const EXPECTED_RESCUE_AMOUNT = new u64(1_000_000);

      beforeEach("set up merge miner", async () => {
        const ownerSDK = QuarrySDK.load({
          provider,
        }).withSigner(ownerKP);
        const {
          tx: initPoolTX,
          key: poolKey,
          replicaToken: replicaTokenInner,
        } = await ownerSDK.mergeMine.newPool({
          primaryMint: stakedToken.mintAccount,
        });
        await expectTX(initPoolTX, "Init pool").to.be.fulfilled;

        // init merge miner
        const poolData =
          await ownerSDK.mergeMine.program.account.mergePool.fetch(poolKey);
        const { tx: initMMTX, key: mmKey } = await ownerSDK.mergeMine.newMM({
          pool: {
            key: poolKey,
            data: poolData,
          },
          rewarder: primary.rewarder,
          rewardsMint: primary.rewardsToken.mintAccount,
        });
        invariant(initMMTX, "initMMTX");
        await expectTX(initMMTX, "Init merge miner").to.be.fulfilled;

        mergePoolKey = poolKey;
        mergeMinerKey = mmKey;
        replicaToken = replicaTokenInner;
      });

      beforeEach("airdrop rescue token's to merge miner's miner", async () => {
        rescueMint = await createMint(provider);
        const [miner] = await findMinerAddress(primary.quarry, mergeMinerKey);
        minerATAKey = await getATAAddress({
          mint: rescueMint,
          owner: miner,
        });
        const mintToTX = new TransactionEnvelope(provider, [
          createATAInstruction({
            address: minerATAKey,
            mint: rescueMint,
            owner: miner,
            payer: provider.wallet.publicKey,
          }),
          SPLToken.createMintToInstruction(
            TOKEN_PROGRAM_ID,
            rescueMint,
            minerATAKey,
            provider.wallet.publicKey,
            [],
            EXPECTED_RESCUE_AMOUNT
          ),
        ]);
        await expectTX(mintToTX, "Mint rescue tokens to miner").to.be.fulfilled;
        minerKey = miner;
      });

      it("Cannot rescue with miner's token vault account", async () => {
        const ownerSDK = QuarrySDK.load({
          provider,
        }).withSigner(ownerKP);

        const { address: destinationTokenAccount, instruction } =
          await getOrCreateATA({
            provider: ownerSDK.provider,
            mint: rescueMint,
            owner: ownerKP.publicKey,
          });
        const tx = ownerSDK.mergeMine.rescueTokens({
          mergePool: mergePoolKey,
          mergeMiner: mergeMinerKey,
          miner: minerKey,
          minerTokenAccount: await getATAAddress({
            mint: primary.quarryW.quarryData.tokenMintKey,
            owner: minerKey,
          }),
          destinationTokenAccount,
        });
        if (instruction) {
          tx.instructions.unshift(instruction);
        }

        await expectTXTable(
          tx,
          "rescue tokens from mergeMiner"
        ).to.be.rejectedWith("0x454");
      });

      it("Cannot rescue with primary mint", async () => {
        const ownerSDK = QuarrySDK.load({
          provider,
        }).withSigner(ownerKP);

        const primaryMint = primary.quarryW.quarryData.tokenMintKey;
        const { key: minerTokenAccount, tx: createMinerTokenAccountTx } =
          await createTokenAccount({
            provider: ownerSDK.provider,
            mint: primaryMint,
            owner: minerKey,
          });
        const { address: destinationTokenAccount, instruction: primaryATAIx } =
          await getOrCreateATA({
            provider: ownerSDK.provider,
            mint: primaryMint,
            owner: ownerKP.publicKey,
          });
        const rescueTX = ownerSDK.mergeMine.rescueTokens({
          mergePool: mergePoolKey,
          mergeMiner: mergeMinerKey,
          miner: minerKey,
          minerTokenAccount,
          destinationTokenAccount,
        });
        if (primaryATAIx) {
          rescueTX.instructions.unshift(primaryATAIx);
        }

        const tx = createMinerTokenAccountTx.combine(rescueTX);
        await expectTXTable(
          tx,
          "rescue tokens from mergeMiner"
        ).to.be.rejectedWith("0x454");
      });

      it("Cannot rescue with replica mint account", async () => {
        const ownerSDK = QuarrySDK.load({
          provider,
        }).withSigner(ownerKP);

        const { address: destinationTokenAccount, instruction: rescueATAIX } =
          await getOrCreateATA({
            provider: ownerSDK.provider,
            mint: replicaToken.mintAccount,
            owner: ownerKP.publicKey,
          });
        const { address: minerTokenAccount, instruction: replicaATAIX } =
          await getOrCreateATA({
            provider: ownerSDK.provider,
            mint: replicaToken.mintAccount,
            owner: minerKey,
          });
        const tx = ownerSDK.mergeMine.rescueTokens({
          mergePool: mergePoolKey,
          mergeMiner: mergeMinerKey,
          miner: minerKey,
          minerTokenAccount,
          destinationTokenAccount,
        });
        if (replicaATAIX) {
          tx.instructions.unshift(replicaATAIX);
        }
        if (rescueATAIX) {
          tx.instructions.unshift(rescueATAIX);
        }

        await expectTXTable(
          tx,
          "rescue tokens from mergeMiner"
        ).to.be.rejectedWith("0x454");
      });

      it("Successfully rescue tokens", async () => {
        const ownerSDK = QuarrySDK.load({
          provider,
        }).withSigner(ownerKP);

        const { address: destinationTokenAccount, instruction } =
          await getOrCreateATA({
            provider: ownerSDK.provider,
            mint: rescueMint,
            owner: ownerKP.publicKey,
          });
        const tx = ownerSDK.mergeMine.rescueTokens({
          mergePool: mergePoolKey,
          mergeMiner: mergeMinerKey,
          miner: minerKey,
          minerTokenAccount: minerATAKey,
          destinationTokenAccount,
        });
        if (instruction) {
          tx.instructions.unshift(instruction);
        }

        await expectTXTable(tx, "rescue tokens from mergeMiner").to.be
          .fulfilled;

        const tokenAccount = await getTokenAccount(
          provider,
          destinationTokenAccount
        );
        expect(tokenAccount.amount).to.bignumber.eq(EXPECTED_RESCUE_AMOUNT);
      });
    });
  });

  describe("Merge Pool SDK", () => {
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
      const [mmKey] = await findMergeMinerAddress({
        pool: poolKey,
        owner: ownerKP.publicKey,
      });

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
        expect(
          state.replicaABalance,
          msgFmt("replicaABalance")
        ).to.bignumber.eq(args.replicaABalance);
        expect(
          state.replicaBBalance,
          msgFmt("replicaBBalance")
        ).to.bignumber.eq(args.replicaBBalance);
        expect(state.walletBalance, msgFmt("walletBalance")).to.bignumber.eq(
          args.walletBalance
        );
      };

      // deposit into pool
      const mp = ownerSDK.mergeMine.loadMP({
        mpKey: poolKey,
      });
      const depositTX = await mp.deposit({
        amount: stakedAmount,
        rewarder: primary.rewarder,
      });
      await expectTXTable(depositTX, "Deposit into rewarder").to.be.fulfilled;

      const replicaAStakeTX = await mp.stakeReplicaMiner(
        replicaA.rewarder,
        mmKey
      );
      await expectTXTable(replicaAStakeTX, "Stake into replica A").to.be
        .fulfilled;

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
      const claimPrimary = await mp.claimPrimaryRewards(
        primary.rewarder,
        mmKey
      );
      const { ataIXs: claimPrimaryATATx, tx: claimPrimaryTx } =
        claimPrimary.splitATAIXs();
      await expectTXTable(
        claimPrimaryATATx,
        "Create ATA accounts for primary claim"
      ).to.be.fulfilled;
      await expectTXTable(claimPrimaryTx, "Claim primary").to.be.fulfilled;

      // claim replica A rewards
      const claimReplicaA = await mp.claimReplicaRewards(
        replicaA.rewarder,
        mmKey
      );
      const { ataIXs: claimReplicaATATx, tx: claimReplicaATX } =
        claimReplicaA.splitATAIXs();
      await expectTXTable(
        claimReplicaATATx,
        "Create ATA accounts for replica claim"
      ).to.be.fulfilled;
      await expectTXTable(claimReplicaATX, "Claim replica A").to.be.fulfilled;

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
      const unstakeReplicaATX = await mp.unstakeAllReplica(
        replicaA.rewarder,
        mmKey
      );
      await expectTX(unstakeReplicaATX, "Unstake Replica A").to.be.fulfilled;
      const withdrawTX = await mp.withdraw({
        amount: stakedAmount,
        rewarder: primary.rewarder,
        mergeMiner: mmKey,
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
      const mp = ownerSDK.mergeMine.loadMP({
        mpKey: poolKey,
      });
      const depositTX = await mp.deposit({
        amount: stakedAmount,
        rewarder: primary.rewarder,
      });
      await expectTXTable(depositTX, "Deposit into rewarder").to.be.fulfilled;

      const [mmKey] = await findMergeMinerAddress({
        pool: poolKey,
        owner: ownerKP.publicKey,
      });
      const replicaAStakeTX = await mp.stakeReplicaMiner(
        replicaA.rewarder,
        mmKey
      );
      await expectTX(replicaAStakeTX, "Stake into replica A").to.be.fulfilled;

      const withdrawTX = await mp.withdraw({
        amount: stakedAmount,
        rewarder: primary.rewarder,
        mergeMiner: mmKey,
      });

      try {
        await withdrawTX.confirm();
      } catch (e) {
        const err = e as Error;
        expect(err.message).to.include(
          `0x${QuarryMergeMineErrors.OutstandingReplicaTokens.code.toString(
            16
          )}`
        );
      }
    });
  });
});
