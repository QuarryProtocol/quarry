import * as anchor from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  createMint,
  getATAAddress,
  getOrCreateATA,
  getTokenAccount,
  Token,
  TOKEN_PROGRAM_ID,
  TokenAmount,
} from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair } from "@solana/web3.js";
import * as assert from "assert";
import { BN } from "bn.js";
import { expect } from "chai";
import invariant from "tiny-invariant";

import type {
  MinerData,
  MineWrapper,
  MintWrapper,
  QuarryData,
  QuarrySDK,
  QuarryWrapper,
  RewarderWrapper,
} from "../src";
import { findQuarryAddress, QUARRY_FEE_TO } from "../src";
import {
  DEFAULT_DECIMALS,
  DEFAULT_HARD_CAP,
  newUserStakeTokenAccount,
} from "./utils";
import { makeSDK } from "./workspace";

const ZERO = new BN(0);

describe("Mine", () => {
  const { web3, BN } = anchor;

  const DAILY_REWARDS_RATE = new BN(1_000 * web3.LAMPORTS_PER_SOL);
  const ANNUAL_REWARDS_RATE = DAILY_REWARDS_RATE.mul(new BN(365));

  let stakedMintAuthority: anchor.web3.Keypair;
  let stakeTokenMint: anchor.web3.PublicKey;
  let stakeToken: Token;

  let sdk: QuarrySDK;
  let provider: Provider;
  let mintWrapper: MintWrapper;
  let mine: MineWrapper;

  before("Initialize SDK", () => {
    sdk = makeSDK();
    provider = sdk.provider;
    mintWrapper = sdk.mintWrapper;
    mine = sdk.mine;
  });

  before(async () => {
    await assert.doesNotReject(async () => {
      stakedMintAuthority = web3.Keypair.generate();
      stakeTokenMint = await createMint(
        provider,
        stakedMintAuthority.publicKey,
        DEFAULT_DECIMALS
      );
    });

    stakeToken = Token.fromMint(stakeTokenMint, DEFAULT_DECIMALS, {
      name: "stake token",
    });
  });

  let rewardsMint: PublicKey;
  let token: Token;
  let mintWrapperKey: PublicKey;
  let hardCap: TokenAmount;

  beforeEach("Initialize mint", async () => {
    const rewardsMintKP = Keypair.generate();
    rewardsMint = rewardsMintKP.publicKey;
    token = Token.fromMint(rewardsMint, DEFAULT_DECIMALS);
    hardCap = TokenAmount.parse(token, DEFAULT_HARD_CAP.toString());
    const { tx, mintWrapper: wrapperKey } = await mintWrapper.newWrapper({
      hardcap: hardCap.toU64(),
      tokenMint: rewardsMint,
    });

    await expectTX(
      await createInitMintInstructions({
        provider,
        mintKP: rewardsMintKP,
        decimals: DEFAULT_DECIMALS,
        mintAuthority: wrapperKey,
        freezeAuthority: wrapperKey,
      })
    ).to.be.fulfilled;

    mintWrapperKey = wrapperKey;
    await expectTX(tx, "Initialize mint").to.be.fulfilled;
  });

  describe("Rewarder", () => {
    let rewarderKey: PublicKey;

    beforeEach("rewarder", async () => {
      const { tx, key: rewarder } = await mine.createRewarder({
        mintWrapper: mintWrapperKey,
        authority: provider.wallet.publicKey,
      });
      await expectTX(tx, "Create new rewarder").to.be.fulfilled;
      rewarderKey = rewarder;
    });

    describe("DAO fees", () => {
      it("anyone can claim", async () => {
        const claimFeeTokenAccount = await getATAAddress({
          mint: rewardsMint,
          owner: rewarderKey,
        });
        const ata = await getOrCreateATA({
          owner: QUARRY_FEE_TO,
          mint: rewardsMint,
          provider,
        });

        assert.ok(ata.instruction);
        await expectTX(new TransactionEnvelope(provider, [ata.instruction])).to
          .be.fulfilled;
        await expect(
          mine.program.rpc.extractFees({
            accounts: {
              rewarder: rewarderKey,
              claimFeeTokenAccount,
              feeToTokenAccount: ata.address,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
          })
        ).to.be.fulfilled;
      });

      it("fail if token account does not exist", async () => {
        const claimFeeTokenAccount = await getATAAddress({
          mint: rewardsMint,
          owner: rewarderKey,
        });
        const ata = await getOrCreateATA({
          owner: QUARRY_FEE_TO,
          mint: rewardsMint,
          provider,
        });
        try {
          await mine.program.rpc.extractFees({
            accounts: {
              rewarder: rewarderKey,
              claimFeeTokenAccount,
              feeToTokenAccount: ata.address,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
          });
          assert.fail("passed");
        } catch (e) {
          console.error(e);
        }
      });

      it("fail if not fee to", async () => {
        const claimFeeTokenAccount = await getATAAddress({
          mint: rewardsMint,
          owner: rewarderKey,
        });
        const ata = await getOrCreateATA({
          owner: Keypair.generate().publicKey,
          mint: rewardsMint,
          provider,
        });
        assert.ok(ata.instruction);
        await expectTX(new TransactionEnvelope(provider, [ata.instruction])).to
          .be.fulfilled;
        try {
          await mine.program.rpc.extractFees({
            accounts: {
              rewarder: rewarderKey,
              claimFeeTokenAccount,
              feeToTokenAccount: ata.address,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
          });
          assert.fail("passed");
        } catch (e) {
          console.error(e);
        }
      });
    });

    it("Is initialized!", async () => {
      const rewarder = await mine.program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.eqAddress(provider.wallet.publicKey);
      expect(rewarder.annualRewardsRate.toString()).to.eql(ZERO.toString());
      expect(rewarder.numQuarries).to.eq(ZERO.toNumber());
      expect(rewarder.totalRewardsShares.toString()).to.bignumber.eq(
        ZERO.toString()
      );
    });

    it("Set daily rewards rate", async () => {
      await assert.doesNotReject(async () => {
        await mine.program.rpc.setAnnualRewards(ANNUAL_REWARDS_RATE, {
          accounts: {
            auth: {
              authority: provider.wallet.publicKey,
              rewarder: rewarderKey,
            },
          },
        });
      });

      const rewarder = await mine.program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.annualRewardsRate).bignumber.to.eq(ANNUAL_REWARDS_RATE);
    });

    it("Transfer authority and accept authority", async () => {
      const newAuthority = web3.Keypair.generate();

      await assert.doesNotReject(async () => {
        await mine.program.rpc.transferAuthority(newAuthority.publicKey, {
          accounts: {
            authority: provider.wallet.publicKey,
            rewarder: rewarderKey,
          },
        });
      });

      let rewarder = await mine.program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.eqAddress(provider.wallet.publicKey);
      expect(rewarder.pendingAuthority).to.eqAddress(newAuthority.publicKey);

      const ix = mine.program.instruction.acceptAuthority({
        accounts: {
          authority: newAuthority.publicKey,
          rewarder: rewarderKey,
        },
      });
      let tx = sdk.newTx([ix], [newAuthority]);
      await expectTX(tx, "accept authority").to.be.fulfilled;
      rewarder = await mine.program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.eqAddress(newAuthority.publicKey);
      expect(rewarder.pendingAuthority).to.eqAddress(web3.PublicKey.default);

      // Transfer back
      const instructions = [];
      instructions.push(
        mine.program.instruction.transferAuthority(provider.wallet.publicKey, {
          accounts: {
            authority: newAuthority.publicKey,
            rewarder: rewarderKey,
          },
        })
      );
      instructions.push(
        mine.program.instruction.acceptAuthority({
          accounts: {
            authority: provider.wallet.publicKey,
            rewarder: rewarderKey,
          },
        })
      );

      tx = sdk.newTx(instructions, [newAuthority]);
      await expectTX(tx, "transfer authority back to original authority").to.be
        .fulfilled;

      rewarder = await mine.program.account.rewarder.fetch(rewarderKey);
      expect(rewarder.authority).to.eqAddress(provider.wallet.publicKey);
      expect(rewarder.pendingAuthority).to.eqAddress(web3.PublicKey.default);
    });
  });

  describe("Quarry", () => {
    const quarryRewardsShare = ANNUAL_REWARDS_RATE.div(new BN(10));
    let quarryData: QuarryData;
    let quarryKey: anchor.web3.PublicKey;
    let rewarderKey: anchor.web3.PublicKey;
    let rewarder: RewarderWrapper;

    beforeEach(async () => {
      const { tx, key: theRewarderKey } = await mine.createRewarder({
        mintWrapper: mintWrapperKey,
        authority: provider.wallet.publicKey,
      });
      await expectTX(tx, "Create new rewarder").to.be.fulfilled;
      rewarderKey = theRewarderKey;
      rewarder = await mine.loadRewarderWrapper(rewarderKey);
      await expectTX(
        await rewarder.setAndSyncAnnualRewards(ANNUAL_REWARDS_RATE, []),
        "set annual rewards"
      );
    });

    describe("Single quarry", () => {
      beforeEach("Create a new quarry", async () => {
        const { quarry, tx } = await rewarder.createQuarry({
          token: stakeToken,
        });
        await expectTX(tx, "Create new quarry").to.be.fulfilled;

        const rewarderData = await mine.program.account.rewarder.fetch(
          rewarderKey
        );
        assert.strictEqual(rewarderData.numQuarries, 1);
        const quarryAccountInfo = await provider.connection.getAccountInfo(
          quarry
        );
        expect(quarryAccountInfo?.owner).to.eqAddress(mine.program.programId);

        assert.ok(quarryAccountInfo);
        quarryData = mine.program.coder.accounts.decode<QuarryData>(
          "Quarry",
          quarryAccountInfo.data
        );
        assert.strictEqual(
          quarryData.famineTs.toString(),
          "9223372036854775807"
        );
        assert.strictEqual(
          quarryData.tokenMintKey.toBase58(),
          stakeTokenMint.toBase58()
        );
        assert.strictEqual(
          quarryData.annualRewardsRate.toString(),
          ZERO.toString()
        );
        assert.strictEqual(quarryData.rewardsShare.toString(), ZERO.toString());

        quarryKey = quarry;
      });

      it("Set rewards share", async () => {
        const currentTime = Math.floor(new Date().getTime() / 1000);

        await assert.doesNotReject(async () => {
          await mine.program.rpc.setRewardsShare(quarryRewardsShare, {
            accounts: {
              auth: {
                authority: provider.wallet.publicKey,
                rewarder: rewarderKey,
              },
              quarry: quarryKey,
            },
          });
        });

        const rewarderData = await mine.program.account.rewarder.fetch(
          rewarderKey
        );
        expect(rewarderData.totalRewardsShares.toString()).to.equal(
          quarryRewardsShare.toString()
        );

        const quarry = await rewarder.getQuarry(stakeToken);
        expect(quarry.key).to.eqAddress(quarryKey);
        expect(
          quarry.quarryData.lastUpdateTs
            .sub(new BN(currentTime))
            .abs()
            .lte(new BN(1))
        ).to.be.true;
        const expectedRewardsRate = quarry.computeAnnualRewardsRate();
        expect(quarry.quarryData.annualRewardsRate.toString()).to.equal(
          expectedRewardsRate.toString()
        );
        expect(quarry.quarryData.rewardsShare.toString()).to.eq(
          quarryRewardsShare.toString()
        );
      });

      it("Set famine", async () => {
        const now = new BN(Date.now());
        await assert.doesNotReject(async () => {
          await mine.program.rpc.setFamine(now, {
            accounts: {
              auth: {
                authority: provider.wallet.publicKey,
                rewarder: rewarderKey,
              },
              quarry: quarryKey,
            },
          });
        });
        const quarryAccountInfo = await provider.connection.getAccountInfo(
          quarryKey
        );
        assert.ok(quarryAccountInfo);
        const quarryData = mine.program.coder.accounts.decode<QuarryData>(
          "Quarry",
          quarryAccountInfo?.data
        );
        assert.strictEqual(quarryData.famineTs.toString(), now.toString());

        await assert.doesNotReject(async () => {
          await mine.program.rpc.setFamine(quarryData.famineTs, {
            accounts: {
              auth: {
                authority: provider.wallet.publicKey,
                rewarder: rewarderKey,
              },
              quarry: quarryKey,
            },
          });
        });
      });

      it("Unauthorized", async () => {
        const fakeAuthority = web3.Keypair.generate();
        const nextMint = await createMint(
          provider,
          provider.wallet.publicKey,
          DEFAULT_DECIMALS
        );
        const [quarryKey, bump] = await findQuarryAddress(
          rewarderKey,
          nextMint
        );
        await assert.rejects(
          async () => {
            await mine.program.rpc.createQuarry(bump, {
              accounts: {
                quarry: quarryKey,
                auth: {
                  authority: fakeAuthority.publicKey,
                  rewarder: rewarderKey,
                },
                tokenMint: nextMint,
                payer: fakeAuthority.publicKey,
                unusedClock: web3.SYSVAR_CLOCK_PUBKEY,
                systemProgram: web3.SystemProgram.programId,
              },
              signers: [fakeAuthority],
            });
          },
          (err: Error) => {
            console.error(err);
            expect(err.message).to.include("custom program error: 0x1"); // mut constraint
            return true;
          }
        );
      });

      it("Invalid PDA", async () => {
        await assert.rejects(async () => {
          const [quarryKey, bump] = await findQuarryAddress(
            rewarderKey,
            Keypair.generate().publicKey
          );
          await mine.program.rpc.createQuarry(bump, {
            accounts: {
              quarry: quarryKey,
              auth: {
                authority: provider.wallet.publicKey,
                rewarder: rewarderKey,
              },
              tokenMint: stakeTokenMint,
              payer: provider.wallet.publicKey,
              unusedClock: web3.SYSVAR_CLOCK_PUBKEY,
              systemProgram: web3.SystemProgram.programId,
            },
          });
        });
      });
    });

    describe("Multiple quarries", () => {
      const tokens: Token[] = [];

      beforeEach("Create quarries", async () => {
        let totalRewardsShare = ZERO;
        const numQuarries = 5;
        for (let i = 0; i < numQuarries; i++) {
          const mint = await createMint(provider);
          const token = Token.fromMint(mint, DEFAULT_DECIMALS, {
            name: "stake token",
          });

          tokens.push(token);
          const rewardsShare = new BN(i + 1);
          const { tx } = await rewarder.createQuarry({
            token,
          });
          await expectTX(tx, "create quarry").to.be.fulfilled;

          const quarry = await rewarder.getQuarry(token);
          await expectTX(quarry.setRewardsShare(rewardsShare)).to.be.fulfilled;
          totalRewardsShare = totalRewardsShare.add(rewardsShare);
        }

        const rewarderData = await mine.program.account.rewarder.fetch(
          rewarderKey
        );
        expect(rewarderData.numQuarries).to.eq(numQuarries);
        expect(rewarderData.totalRewardsShares).to.bignumber.eq(
          totalRewardsShare
        );

        const mints = tokens.map((tok) => tok.mintAccount);
        const tx = await rewarder.syncQuarryRewards(mints);
        await expectTX(tx, "sync quarries").to.be.fulfilled;
      });

      it("Set annual rewards and make sure quarries update", async () => {
        const multiplier = new BN(10);
        let rewarderData = await mine.program.account.rewarder.fetch(
          rewarderKey
        );
        const nextAnnualRewardsRate = ANNUAL_REWARDS_RATE.mul(multiplier);
        const prevRates = await Promise.all(
          tokens.map(async (t) => {
            const quarry = await rewarder.getQuarry(t);
            return { token: t, rate: quarry.quarryData.annualRewardsRate };
          })
        );

        const tx = await rewarder.setAndSyncAnnualRewards(
          nextAnnualRewardsRate,
          tokens.map((t) => t.mintAccount)
        );
        console.log(await tx.simulate());
        await expectTX(tx, "set annual rewards and update quarry rewards").to.be
          .fulfilled;

        rewarderData = await mine.program.account.rewarder.fetch(rewarderKey);
        expect(rewarderData.annualRewardsRate).to.bignumber.eq(
          nextAnnualRewardsRate
        );

        let sumRewardsPerAnnum = new BN(0);
        for (const token of tokens) {
          const nextRate = (await rewarder.getQuarry(token)).quarryData
            .annualRewardsRate;
          sumRewardsPerAnnum = sumRewardsPerAnnum.add(nextRate);
          const prevRate = prevRates.find((r) => r.token.equals(token))?.rate;
          invariant(
            prevRate,
            `prev rate not found for token ${token.toString()}`
          );

          // Epsilon is 10
          // check to see difference is less than 10
          const expectedRate = prevRate.mul(multiplier);
          expect(
            nextRate,
            `mul rate: ${multiplier.toString()}; expected: ${expectedRate.toString()}; got: ${nextRate.toString()}`
          ).to.bignumber.closeTo(expectedRate, "10");
        }
        // Check on day multiple
        expect(
          sumRewardsPerAnnum,
          "rewards rate within one day multiple"
        ).bignumber.closeTo(
          nextAnnualRewardsRate,
          new BN(2) // precision lost
        );

        // Restore daily rewards rate
        const txRestore = await rewarder.setAndSyncAnnualRewards(
          ANNUAL_REWARDS_RATE,
          tokens.map((t) => t.mintAccount)
        );
        await expectTX(txRestore, "revert daily rewards to previous amount").to
          .be.fulfilled;

        for (const token of tokens) {
          const lastRate = (
            await rewarder.getQuarry(token)
          ).computeAnnualRewardsRate();
          const prevRate = prevRates.find((r) => r.token.equals(token))?.rate;
          invariant(
            prevRate,
            `prev rate not found for token ${token.toString()}`
          );
          expect(lastRate, `revert rate ${token.toString()}`).bignumber.to.eq(
            prevRate
          );
        }
      });
    });
  });

  describe("Miner", () => {
    let rewarderKey: anchor.web3.PublicKey;
    let rewarder: RewarderWrapper;
    let quarry: QuarryWrapper;

    beforeEach(async () => {
      const { tx, key: theRewarderKey } = await mine.createRewarder({
        mintWrapper: mintWrapperKey,
        authority: provider.wallet.publicKey,
      });
      await expectTX(tx, "Create new rewarder").to.be.fulfilled;
      rewarderKey = theRewarderKey;
      rewarder = await mine.loadRewarderWrapper(rewarderKey);
      await expectTX(
        await rewarder.setAndSyncAnnualRewards(ANNUAL_REWARDS_RATE, [])
      ).to.be.fulfilled;

      const { tx: quarryTx } = await rewarder.createQuarry({
        token: stakeToken,
      });
      await expectTX(quarryTx, "Create new quarry").to.be.fulfilled;
    });

    beforeEach("Create miner", async () => {
      quarry = await rewarder.getQuarry(stakeToken);
      expect(quarry).to.exist;

      // create the miner
      await expectTX((await quarry.createMiner()).tx, "create miner").to.be
        .fulfilled;
    });

    it("Valid miner", async () => {
      const miner = await quarry.getMinerAddress(provider.wallet.publicKey);
      const minerAccountInfo = await provider.connection.getAccountInfo(miner);
      expect(minerAccountInfo?.owner).to.eqAddress(mine.program.programId);
      assert.ok(minerAccountInfo?.data);
      const minerData = mine.program.coder.accounts.decode<MinerData>(
        "Miner",
        minerAccountInfo.data
      );
      expect(minerData.authority).to.eqAddress(provider.wallet.publicKey);
      assert.strictEqual(minerData.quarryKey.toBase58(), quarry.key.toBase58());

      const minerBalance = await getTokenAccount(
        provider,
        minerData.tokenVaultKey
      );
      expect(minerBalance.amount).to.bignumber.eq(ZERO);
    });

    it("Stake and withdraw", async () => {
      // mint test tokens
      const amount = 1_000_000000;
      const userStakeTokenAccount = await newUserStakeTokenAccount(
        sdk,
        quarry,
        stakeToken,
        stakedMintAuthority,
        amount
      );

      // stake into the quarry
      const minerActions = await quarry.getMinerActions(
        provider.wallet.publicKey
      );
      await expectTX(
        minerActions.stake(new TokenAmount(stakeToken, amount)),
        "Stake into the quarry"
      ).to.be.fulfilled;

      let miner = await quarry.getMiner(provider.wallet.publicKey);
      invariant(miner, "miner must exist");

      const minerBalance = await getTokenAccount(provider, miner.tokenVaultKey);
      expect(minerBalance.amount).to.bignumber.eq(new BN(amount));

      let minerVaultInfo = await getTokenAccount(provider, miner.tokenVaultKey);
      expect(minerVaultInfo.amount).to.bignumber.eq(new BN(amount));
      let userStakeTokenAccountInfo = await getTokenAccount(
        provider,
        userStakeTokenAccount
      );
      expect(userStakeTokenAccountInfo.amount).to.bignumber.eq(ZERO);

      // withdraw from the quarry
      await expectTX(
        minerActions.withdraw(new TokenAmount(stakeToken, amount)),
        "Withdraw from the quarry"
      ).to.be.fulfilled;
      miner = await quarry.getMiner(provider.wallet.publicKey);
      invariant(miner, "miner must exist");

      const endMinerBalance = await getTokenAccount(
        provider,
        miner.tokenVaultKey
      );
      expect(endMinerBalance.amount).to.bignumber.eq(ZERO);

      minerVaultInfo = await getTokenAccount(provider, miner.tokenVaultKey);
      expect(minerVaultInfo.amount.toNumber()).to.eq(ZERO.toNumber());
      userStakeTokenAccountInfo = await getTokenAccount(
        provider,
        userStakeTokenAccount
      );
      expect(userStakeTokenAccountInfo.amount.toNumber()).to.eq(amount);
    });
  });
});
