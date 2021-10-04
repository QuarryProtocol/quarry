import type * as anchor from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  createMint,
  Token,
  TokenAmount,
} from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { BN } from "bn.js";

import type {
  MineWrapper,
  MintWrapper,
  QuarrySDK,
  RewarderWrapper,
} from "../src";
import { DEFAULT_DECIMALS, DEFAULT_HARD_CAP } from "./utils";
import { makeSDK } from "./workspace";

describe("Registry", () => {
  const dailyRewardsRate = new BN(1_000 * LAMPORTS_PER_SOL);
  const annualRewardsRate = dailyRewardsRate.mul(new BN(365));

  let sdk: QuarrySDK;
  let provider: Provider;
  let mintWrapper: MintWrapper;
  let mine: MineWrapper;

  let stakedMintAuthority: anchor.web3.Keypair;
  let stakeTokenMint: anchor.web3.PublicKey;
  let stakeToken: Token;

  let rewarder: PublicKey;
  let rewarderWrapper: RewarderWrapper;

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

  beforeEach(async () => {
    stakedMintAuthority = Keypair.generate();
    stakeTokenMint = await createMint(
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

    const setAnnualRewardsTX = await rewarderWrapper.setAndSyncAnnualRewards(
      annualRewardsRate,
      []
    );
    await expectTX(setAnnualRewardsTX).eventually.to.be.fulfilled;

    const { tx: createQuarryTX } = await rewarderWrapper.createQuarry({
      token: stakeToken,
    });
    await expectTX(createQuarryTX, "create quarry for stake token").to.be
      .fulfilled;
  });

  it("create registry", async () => {
    const { tx: createRegistryTX, registry } = await sdk.registry.newRegistry({
      numQuarries: 100,
      rewarderKey: rewarder,
    });
    await expectTX(createRegistryTX, "create registry").to.eventually.be
      .fulfilled;

    const syncQuarryTX = await sdk.registry.syncQuarry({
      tokenMint: stakeToken.mintAccount,
      rewarderKey: rewarder,
    });
    await expectTX(syncQuarryTX, "sync quarry").to.eventually.be.fulfilled;

    console.log(
      (await sdk.registry.program.account.registry.fetch(registry)).tokens.map(
        (tok) => tok.toString()
      )
    );
  });
});
