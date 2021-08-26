import * as anchor from "@project-serum/anchor";
import * as serumCmn from "@project-serum/common";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  getATAAddress,
  Token,
  TokenAmount,
} from "@saberhq/token-utils";
import { u64 } from "@solana/spl-token";
import type { PublicKey } from "@solana/web3.js";
import { Keypair } from "@solana/web3.js";
import * as assert from "assert";
import BN = require("bn.js");
import { expect } from "chai";

import type {
  ClaimEvent,
  MineWrapper,
  MintWrapper,
  MintWrapperProgram,
  QuarrySDK,
  RewarderWrapper,
  StakeEvent,
} from "../src";
import {
  DEFAULT_DECIMALS,
  DEFAULT_HARD_CAP,
  newUserStakeTokenAccount,
} from "./utils";
import { makeSDK } from "./workspace";

const ZERO = new BN(0);

describe("MintWrapper", () => {
  const { BN, web3 } = anchor;

  let sdk: QuarrySDK;
  let provider: Provider;
  let mintWrapper: MintWrapper;
  let mine: MineWrapper;
  let MintWrapper: MintWrapperProgram;

  before("Initialize SDK", () => {
    sdk = makeSDK();
    provider = sdk.provider;
    mintWrapper = sdk.mintWrapper;
    mine = sdk.mine;
    MintWrapper = sdk.programs.MintWrapper;
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

  it("Check MintWrapper", async () => {
    const mintInfo = await serumCmn.getMintInfo(provider, rewardsMint);
    assert.ok(mintInfo.mintAuthority?.equals(mintWrapperKey));

    const mintWrapperState =
      await mintWrapper.program.account.mintWrapper.fetch(mintWrapperKey);
    expect(mintWrapperState.hardCap).to.bignumber.eq(hardCap.toU64());
    expect(mintWrapperState.admin).to.eqAddress(provider.wallet.publicKey);
    expect(mintWrapperState.tokenMint).to.eqAddress(rewardsMint);
  });

  describe("MintWrapper", () => {
    it("Transfer super authority and accept super authority", async () => {
      const newAuthority = web3.Keypair.generate();

      await assert.doesNotReject(async () => {
        await MintWrapper.rpc.transferAdmin({
          accounts: {
            mintWrapper: mintWrapperKey,
            admin: provider.wallet.publicKey,
            nextAdmin: newAuthority.publicKey,
          },
        });
      });

      let mintWrapperState =
        await mintWrapper.program.account.mintWrapper.fetch(mintWrapperKey);
      expect(mintWrapperState.admin).to.eqAddress(provider.wallet.publicKey);
      expect(mintWrapperState.pendingAdmin).to.eqAddress(
        newAuthority.publicKey
      );

      const ix = mintWrapper.program.instruction.acceptAdmin({
        accounts: {
          mintWrapper: mintWrapperKey,
          pendingAdmin: newAuthority.publicKey,
        },
      });
      let tx = sdk.newTx([ix], [newAuthority]);
      await expectTX(tx, "transfer authority").to.be.fulfilled;
      mintWrapperState = await mintWrapper.program.account.mintWrapper.fetch(
        mintWrapperKey
      );
      expect(mintWrapperState.admin).to.eqAddress(newAuthority.publicKey);
      expect(mintWrapperState.pendingAdmin).to.eqAddress(
        web3.PublicKey.default.toString()
      );

      // Transfer back
      const instructions = [];
      instructions.push(
        mintWrapper.program.instruction.transferAdmin({
          accounts: {
            mintWrapper: mintWrapperKey,
            admin: newAuthority.publicKey,
            nextAdmin: provider.wallet.publicKey,
          },
        })
      );
      instructions.push(
        mintWrapper.program.instruction.acceptAdmin({
          accounts: {
            mintWrapper: mintWrapperKey,
            pendingAdmin: provider.wallet.publicKey,
          },
        })
      );

      tx = sdk.newTx(instructions, [newAuthority]);
      await expectTX(tx, "transfer authority back to original authority").to.be
        .fulfilled;

      mintWrapperState = await mintWrapper.program.account.mintWrapper.fetch(
        mintWrapperKey
      );
      expect(mintWrapperState.admin).to.eqAddress(provider.wallet.publicKey);
      expect(mintWrapperState.pendingAdmin).to.eqAddress(
        web3.PublicKey.default
      );
    });

    it("Adds to the whitelist", async () => {
      const allowance = new u64(1_000_000);
      const id = Keypair.generate().publicKey;
      await expectTX(
        mintWrapper.newMinter(mintWrapperKey, id, allowance),
        "add minter"
      ).to.be.fulfilled;
      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "allowance"
      ).to.bignumber.eq(allowance);
    });

    it("Removes from the whitelist", async () => {
      const allowance = new u64(1_000_000);
      const id = Keypair.generate().publicKey;
      await expectTX(
        mintWrapper.newMinter(mintWrapperKey, id, allowance),
        "add minter"
      ).to.be.fulfilled;

      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "allowance"
      ).to.bignumber.eq(allowance);

      await expectTX(
        mintWrapper.minterUpdate(mintWrapperKey, id, new u64(0)),
        "remove minter"
      ).to.be.fulfilled;
      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "no more allowance"
      ).to.bignumber.zero;
    });
  });

  describe("Mine", () => {
    const dailyRewardsRate = new BN(1000 * web3.LAMPORTS_PER_SOL);
    const rewardsShare = dailyRewardsRate.div(new BN(10));
    const stakeAmount = 1_000_000000;
    let stakedMintAuthority: anchor.web3.Keypair;
    let stakeTokenMint: anchor.web3.PublicKey;
    let stakeToken: Token;

    let rewarder: PublicKey;
    let rewarderWrapper: RewarderWrapper;

    beforeEach(async () => {
      stakedMintAuthority = web3.Keypair.generate();
      stakeTokenMint = await serumCmn.createMint(
        provider,
        stakedMintAuthority.publicKey,
        DEFAULT_DECIMALS
      );
      stakeToken = Token.fromMint(stakeTokenMint, DEFAULT_DECIMALS, {
        name: "stake token",
      });

      const { tx: rewarderTx, key: rewarderKey } = await mine.createRewarder({
        mintWrapper: mintWrapperKey,
      });
      rewarder = rewarderKey;
      await expectTX(rewarderTx).eventually.to.be.fulfilled;

      rewarderWrapper = await mine.loadRewarderWrapper(rewarder);

      const setDailyRewardsTX = await rewarderWrapper.setDailyRewards(
        dailyRewardsRate,
        []
      );
      await expectTX(setDailyRewardsTX).eventually.to.be.fulfilled;

      const { tx: createQuarryTX } = await rewarderWrapper.createQuarry({
        token: stakeToken,
      });
      await expectTX(createQuarryTX, "create quarry for stake token").to.be
        .fulfilled;
      const quarryWrapper = await rewarderWrapper.getQuarry(stakeToken);
      await expectTX(
        (
          await quarryWrapper.createMiner()
        ).tx,
        "create miner for user"
      ).to.be.fulfilled;

      await expectTX(
        quarryWrapper.setRewardsShare(rewardsShare),
        "set rewards share"
      ).to.be.fulfilled;

      // mint test tokens
      await newUserStakeTokenAccount(
        sdk,
        await rewarderWrapper.getQuarry(stakeToken),
        stakeToken,
        stakedMintAuthority,
        stakeAmount
      );
    });

    it("#stake", async () => {
      let quarry = await rewarderWrapper.getQuarry(stakeToken);
      expect(quarry).to.exist;
      const minerActions = await quarry.getMinerActions(
        provider.wallet.publicKey
      );
      // stake into the quarry
      const tx = minerActions.stake(new TokenAmount(stakeToken, stakeAmount));
      const receipt = await (await tx.send()).wait();

      const parser = new anchor.EventParser(
        sdk.programs.Mine.programId,
        sdk.programs.Mine.coder
      );
      const theParser = (logs: string[]) => {
        const events: StakeEvent[] = [];
        parser.parseLogs(logs, (event) => {
          events.push(event as StakeEvent);
        });
        return events;
      };
      const event = receipt.getEvents(theParser)[0];

      quarry = await rewarderWrapper.getQuarry(stakeToken);
      // Checks
      const payroll = quarry.payroll;
      assert.ok(event);
      const expectedRewardsPerTokenStored = payroll.calculateRewardPerToken(
        event.data.timestamp
      );
      expect(quarry.quarryData.rewardsPerTokenStored.toString()).to.equal(
        expectedRewardsPerTokenStored.toString()
      );
      const expectedRewardsRate = quarry.computeDailyRewardsRate();
      expect(quarry.quarryData.dailyRewardsRate.toString()).to.equal(
        expectedRewardsRate.toString()
      );
    });

    it("#claim", async () => {
      let quarry = await rewarderWrapper.getQuarry(stakeToken);
      expect(quarry).to.exist;
      let miner = await quarry.getMiner(provider.wallet.publicKey);
      expect(miner).to.exist;
      const minerActions = await quarry.getMinerActions(
        provider.wallet.publicKey
      );

      const stakeTx = minerActions.stake(
        new TokenAmount(stakeToken, stakeAmount)
      );
      await expectTX(stakeTx, "Stake").to.be.fulfilled;

      const wagesPerTokenPaid = miner.rewardsPerTokenPaid;

      // whitelist rewarder
      await expectTX(
        mintWrapper.newMinter(
          mintWrapperKey,
          rewarder,
          new u64(100_000_000_000000)
        ),
        "Minter add"
      ).to.be.fulfilled;

      const tx = await minerActions.claim();
      const claimSent = tx.send();
      await expectTX(tx, "Claim").to.be.fulfilled;
      const receipt = await (await claimSent).wait();
      receipt.printLogs();

      const parser = new anchor.EventParser(
        sdk.programs.Mine.programId,
        sdk.programs.Mine.coder
      );
      const theParser = (logs: string[]) => {
        const events: ClaimEvent[] = [];
        parser.parseLogs(logs, (event) => {
          events.push(event as ClaimEvent);
        });
        return events;
      };
      const event = receipt.getEvents(theParser)[0];
      assert.ok(event, "claim event not found");

      quarry = await rewarderWrapper.getQuarry(stakeToken);
      miner = await quarry.getMiner(provider.wallet.publicKey);

      // Checks
      const payroll = quarry.payroll;
      const expectedWagesEarned = payroll.calculateRewardsEarned(
        event.data.timestamp,
        new BN(stakeAmount),
        wagesPerTokenPaid,
        ZERO
      );
      expect(event.data.amount.isZero()).to.be.false;
      expect(event.data.amount).to.bignumber.eq(expectedWagesEarned);
      expect(miner.rewardsEarned.toString()).to.equal(ZERO.toString());
      const rewardsTokenAccount = await getATAAddress({
        mint: rewardsMint,
        owner: provider.wallet.publicKey,
      });
      const rewardsTokenAccountInfo = await serumCmn.getTokenAccount(
        provider,
        rewardsTokenAccount
      );
      expect(rewardsTokenAccountInfo.amount.toString()).to.equal(
        event.data.amount.toString()
      );
    });
  });
});
