import * as anchor from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import { PendingTransaction } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  createMint,
  Token,
  TokenAmount,
  u64,
} from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert, expect } from "chai";
import invariant from "tiny-invariant";

import type {
  MineWrapper,
  MintWrapper,
  QuarrySDK,
  RewarderWrapper,
} from "../src";
import { QuarryOperatorErrors } from "../src";
import type { Operator } from "../src/wrappers/operator";
import { DEFAULT_DECIMALS, DEFAULT_HARD_CAP } from "./utils";
import { makeSDK } from "./workspace";

describe("Operator", () => {
  // Read the provider from the configured environment.

  const { web3, BN } = anchor;

  const DAILY_REWARDS_RATE = new BN(1_000 * web3.LAMPORTS_PER_SOL);
  const ANNUAL_REWARDS_RATE = DAILY_REWARDS_RATE.mul(new BN(365));

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

  it("Create operator", async () => {
    const { key: operatorKey, tx: createTX } = await sdk.createOperator({
      rewarder: rewarderKey,
    });
    await expectTX(rewarder.transferAuthority({ nextAuthority: operatorKey }))
      .to.be.fulfilled;

    await expectTX(createTX).to.be.fulfilled;

    const operator = await sdk.loadOperator(operatorKey);
    expect(operator).to.exist;
  });

  it("Create operator unauthorized", async () => {
    const { tx: createTX } = await sdk.createOperator({
      rewarder: rewarderKey,
    });
    await expectTX(createTX).to.be.rejected;
  });

  describe("Admin functions", () => {
    let operator: Operator;

    beforeEach("Create operator", async () => {
      const { key: operatorKey, tx: createTX } = await sdk.createOperator({
        rewarder: rewarderKey,
      });
      await expectTX(rewarder.transferAuthority({ nextAuthority: operatorKey }))
        .to.be.fulfilled;

      await expectTX(createTX).to.be.fulfilled;

      const maybeOperator = await sdk.loadOperator(operatorKey);
      assert.exists(maybeOperator);
      invariant(maybeOperator);
      operator = maybeOperator;
    });

    const nextDelegate = Keypair.generate().publicKey;

    it("#setAdmin", async () => {
      await expectTX(operator.setAdmin(nextDelegate)).to.be.fulfilled;
      const op2 = await operator.reload();
      expect(op2.data.admin).to.eqAddress(nextDelegate);
    });

    it("#setRateSetter", async () => {
      await expectTX(operator.setRateSetter(nextDelegate)).to.be.fulfilled;
      const op2 = await operator.reload();
      expect(op2.data.admin).to.eqAddress(sdk.provider.wallet.publicKey);
      expect(op2.data.rateSetter).to.eqAddress(nextDelegate);
    });

    it("#setQuarryCreator", async () => {
      await expectTX(operator.setQuarryCreator(nextDelegate)).to.be.fulfilled;
      const op2 = await operator.reload();
      expect(op2.data.admin).to.eqAddress(sdk.provider.wallet.publicKey);
      expect(op2.data.quarryCreator).to.eqAddress(nextDelegate);
    });

    it("#setShareAllocator", async () => {
      await expectTX(operator.setShareAllocator(nextDelegate)).to.be.fulfilled;
      const op2 = await operator.reload();
      expect(op2.data.admin).to.eqAddress(sdk.provider.wallet.publicKey);
      expect(op2.data.shareAllocator).to.eqAddress(nextDelegate);
    });
  });

  describe("create quarry", () => {
    let operator: Operator;

    beforeEach("Create operator", async () => {
      const { key: operatorKey, tx: createTX } = await sdk.createOperator({
        rewarder: rewarderKey,
      });
      await expectTX(rewarder.transferAuthority({ nextAuthority: operatorKey }))
        .to.be.fulfilled;

      await expectTX(createTX).to.be.fulfilled;

      const maybeOperator = await sdk.loadOperator(operatorKey);
      assert.exists(maybeOperator);
      invariant(maybeOperator);
      operator = maybeOperator;
    });

    it("only quarry creator", async () => {
      const randomKP = Keypair.generate();
      const randomOperator = await sdk
        .withSigner(randomKP)
        .loadOperator(operator.key);
      invariant(randomOperator);

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          randomKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      const stakeMint = await createMint(sdk.provider);
      const { tx } = await randomOperator.delegateCreateQuarry({
        tokenMint: stakeMint,
      });
      await expectTX(tx).to.be.rejected.and.to.match(
        new RegExp(`0x${QuarryOperatorErrors.Unauthorized.code.toString(16)}`)
      ); // unauthorized
    });

    it("create quarry", async () => {
      const quarryCreatorKP = Keypair.generate();
      await expectTX(operator.setQuarryCreator(quarryCreatorKP.publicKey)).to.be
        .fulfilled;

      const quarryCreatorOperator = await sdk
        .withSigner(quarryCreatorKP)
        .loadOperator(operator.key);
      invariant(quarryCreatorOperator, "operator must exist");

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          quarryCreatorKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      const stakeMint = await createMint(sdk.provider);
      const { tx } = await quarryCreatorOperator.delegateCreateQuarry({
        tokenMint: stakeMint,
      });
      await expectTX(tx).to.be.fulfilled;
    });
  });

  describe("set rewards share", () => {
    let operator: Operator;
    let quarryKey: PublicKey;

    beforeEach("Create operator", async () => {
      const { key: operatorKey, tx: createTX } = await sdk.createOperator({
        rewarder: rewarderKey,
      });
      await expectTX(rewarder.transferAuthority({ nextAuthority: operatorKey }))
        .to.be.fulfilled;

      await expectTX(createTX).to.be.fulfilled;

      const maybeOperator = await sdk.loadOperator(operatorKey);
      assert.exists(maybeOperator);
      invariant(maybeOperator);
      operator = maybeOperator;

      const stakeMint = await createMint(sdk.provider);
      const { tx, quarry } = await operator.delegateCreateQuarry({
        tokenMint: stakeMint,
      });
      await expectTX(tx).to.be.fulfilled;
      quarryKey = quarry;
    });

    it("only share allocator", async () => {
      const randomKP = Keypair.generate();
      const randomOperator = await sdk
        .withSigner(randomKP)
        .loadOperator(operator.key);
      invariant(randomOperator);

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          randomKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      await expectTX(
        randomOperator.delegateSetRewardsShare({
          quarry: quarryKey,
          share: 1,
        })
      ).to.be.rejected.and.to.match(
        new RegExp(`0x${QuarryOperatorErrors.Unauthorized.code.toString(16)}`)
      ); // unauthorized
    });

    it("allocate share", async () => {
      const shareAllocatorKP = Keypair.generate();
      await expectTX(operator.setShareAllocator(shareAllocatorKP.publicKey)).to
        .be.fulfilled;

      const shareAllocatorOperator = await sdk
        .withSigner(shareAllocatorKP)
        .loadOperator(operator.key);
      invariant(shareAllocatorOperator, "operator must exist");

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          shareAllocatorKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      const tx = shareAllocatorOperator.delegateSetRewardsShare({
        quarry: quarryKey,
        share: 1,
      });
      await expectTX(tx).to.be.fulfilled;
    });

    it("set famine", async () => {
      const rateSetterKP = Keypair.generate();
      await expectTX(operator.setRateSetter(rateSetterKP.publicKey)).to
        .fulfilled;

      const rateSetterOperator = await sdk
        .withSigner(rateSetterKP)
        .loadOperator(operator.key);
      invariant(rateSetterOperator, "operator must exist");

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          rateSetterKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      const tx = rateSetterOperator.delegateSetFamine(
        new u64("9000000000000000000"),
        quarryKey
      );
      await expectTX(tx).to.be.fulfilled;
    });
  });

  describe("set annual rewards", () => {
    let operator: Operator;

    beforeEach("Create operator", async () => {
      const { key: operatorKey, tx: createTX } = await sdk.createOperator({
        rewarder: rewarderKey,
      });
      await expectTX(rewarder.transferAuthority({ nextAuthority: operatorKey }))
        .to.be.fulfilled;

      await expectTX(createTX).to.be.fulfilled;

      const maybeOperator = await sdk.loadOperator(operatorKey);
      assert.exists(maybeOperator);
      invariant(maybeOperator);
      operator = maybeOperator;
    });

    it("only share allocator", async () => {
      const randomKP = Keypair.generate();
      const randomOperator = await sdk
        .withSigner(randomKP)
        .loadOperator(operator.key);
      invariant(randomOperator);

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          randomKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      await expectTX(
        randomOperator.delegateSetAnnualRewards(new u64(1_000000))
      ).to.be.rejected.and.to.match(
        new RegExp(`0x${QuarryOperatorErrors.Unauthorized.code.toString(16)}`)
      );
    });

    it("set annual rewards", async () => {
      const rateSetterKP = Keypair.generate();
      await expectTX(operator.setRateSetter(rateSetterKP.publicKey)).to
        .fulfilled;

      const rateSetterOperator = await sdk
        .withSigner(rateSetterKP)
        .loadOperator(operator.key);
      invariant(rateSetterOperator, "operator must exist");

      await new PendingTransaction(
        sdk.provider.connection,
        await sdk.provider.connection.requestAirdrop(
          rateSetterKP.publicKey,
          LAMPORTS_PER_SOL
        )
      ).wait();

      const tx = rateSetterOperator.delegateSetAnnualRewards(new u64(1_000000));
      await expectTX(tx).to.be.fulfilled;
    });
  });
});
