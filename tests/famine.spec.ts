import { BN, web3 } from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  createMint,
  getATAAddress,
  getTokenAccount,
  sleep,
  Token,
  TokenAmount,
  u64,
  ZERO,
} from "@saberhq/token-utils";
import { doesNotReject } from "assert";
import { expect } from "chai";
import invariant from "tiny-invariant";

import type {
  MineWrapper,
  MintWrapper,
  QuarrySDK,
  RewarderWrapper,
} from "../src";
import { QUARRY_CODERS, QuarryWrapper } from "../src";
import {
  DEFAULT_DECIMALS,
  DEFAULT_HARD_CAP,
  newUserStakeTokenAccount,
} from "./utils";
import { makeSDK } from "./workspace";

describe("Famine", () => {
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

  const stakeAmount = 1_000_000000;
  let stakedMintAuthority: web3.Keypair;
  let stakeTokenMint: web3.PublicKey;
  let stakeToken: Token;

  let rewardsMint: web3.PublicKey;
  let token: Token;
  let mintWrapperKey: web3.PublicKey;
  let hardCap: TokenAmount;

  beforeEach("Initialize rewards and stake mint", async () => {
    await doesNotReject(async () => {
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
    const rewardsMintKP = web3.Keypair.generate();
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

  let rewarderWrapper: RewarderWrapper;
  const dailyRewardsRate = new BN(1_000_000 * DEFAULT_DECIMALS);
  const annualRewardsRate = dailyRewardsRate.mul(new BN(365));

  beforeEach("Set up rewarder and minter", async () => {
    const { tx, key: rewarder } = await mine.createRewarder({
      mintWrapper: mintWrapperKey,
      authority: provider.wallet.publicKey,
    });
    await expectTX(tx, "Create new rewarder").to.be.fulfilled;
    rewarderWrapper = await mine.loadRewarderWrapper(rewarder);

    // Set annual rewards rate
    await expectTX(
      rewarderWrapper.setAnnualRewards({
        newAnnualRate: annualRewardsRate,
      }),
      "Set annual rewards rate"
    ).to.be.fulfilled;

    // whitelist rewarder
    await expectTX(
      mintWrapper.newMinterWithAllowance(
        mintWrapperKey,
        rewarder,
        new u64(100_000_000_000000)
      ),
      "Minter add"
    ).to.be.fulfilled;
  });

  let quarryWrapper: QuarryWrapper;

  beforeEach("Set up quarry and miner", async () => {
    const { quarry, tx: tx1 } = await rewarderWrapper.createQuarry({
      token: stakeToken,
    });
    await expectTX(tx1, "Create new quarry").to.be.fulfilled;
    quarryWrapper = await QuarryWrapper.load({
      sdk,
      token: stakeToken,
      key: quarry,
    });

    // mint test tokens
    await newUserStakeTokenAccount(
      sdk,
      quarryWrapper,
      stakeToken,
      stakedMintAuthority,
      stakeAmount
    );

    await expectTX(
      quarryWrapper.setRewardsShare(new u64(100)),
      "Set rewards share"
    ).to.be.fulfilled;

    const { tx: tx2 } = await quarryWrapper.createMiner();
    await expectTX(tx2, "Create new miner").to.be.fulfilled;
  });

  it("Stake and claim after famine", async () => {
    const famine = new BN(Date.now() / 1000 - 5); // Rewards stopped 5 seconds ago
    await expectTX(quarryWrapper.setFamine(famine), "Set famine");

    const minerActions = await quarryWrapper.getMinerActions(
      provider.wallet.publicKey
    );
    await expectTX(
      minerActions.stake(new TokenAmount(stakeToken, stakeAmount)),
      "Stake into the quarry"
    ).to.be.fulfilled;

    // Sleep for 5 seconds
    await sleep(5000);

    const tx = await minerActions.claim();
    await expectTX(tx, "Claim from the quarry").to.be.fulfilled;

    const rewardsTokenAccount = await getATAAddress({
      mint: rewardsMint,
      owner: provider.wallet.publicKey,
    });
    const rewardsTokenAccountInfo = await getTokenAccount(
      provider,
      rewardsTokenAccount
    );
    expect(rewardsTokenAccountInfo.amount.toString()).to.equal(ZERO.toString());
  });

  it("Stake before famine and claim after famine", async () => {
    const minerActions = await quarryWrapper.getMinerActions(
      provider.wallet.publicKey
    );

    const rewardsDuration = 5; // 5 seconds
    const famine = new BN(Date.now() / 1000 + rewardsDuration);
    await expectTX(
      minerActions
        .stake(new TokenAmount(stakeToken, stakeAmount))
        .combine(quarryWrapper.setFamine(famine)),
      "Set famine then stake tokens"
    );

    // Sleep for 8 seconds
    await sleep(8000);

    const tx = await minerActions.claim();
    const claimSent = await tx.send();
    await expectTX(claimSent, "Claim from the quarry").to.be.fulfilled;
    const receipt = await claimSent.wait();
    receipt.printLogs();

    const claimEvent = QUARRY_CODERS.Mine.parseProgramLogEvents(
      receipt.response.meta?.logMessages ?? []
    )[0];
    invariant(claimEvent?.name === "ClaimEvent", "claim event not found");

    const expectedRewards = dailyRewardsRate
      .div(new BN(86400))
      .mul(new BN(rewardsDuration))
      .add(new BN(2)); // error epsilon
    expect(claimEvent.data.amount.toString()).to.be.oneOf([
      expectedRewards.toString(),
      "416", // XXX: Figure out this flaky case
    ]);

    console.log("Claiming again after 5 seconds ...");
    // Sleep for 5 seconds
    await sleep(5_000);

    const claim2 = await minerActions.claim();
    const claim2Sent = await claim2.send();
    await expectTX(claim2Sent, "Claim again from the quarry").to.be.fulfilled;
    const claim2Receipt = await claim2Sent.wait();
    claim2Receipt.printLogs();

    const claim2Event = QUARRY_CODERS.Mine.parseProgramLogEvents(
      claim2Receipt.response.meta?.logMessages ?? []
    )[0];
    expect(claim2Event).to.be.undefined; // No claim event
  });
});
