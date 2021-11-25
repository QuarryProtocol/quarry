import { BN, web3 } from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  createMint,
  Token,
  TokenAmount,
  u64,
} from "@saberhq/token-utils";
import { doesNotReject } from "assert";

import type {
  MineWrapper,
  MintWrapper,
  QuarrySDK,
  RewarderWrapper,
} from "../src";
import { QuarryWrapper } from "../src";
import { DEFAULT_DECIMALS, DEFAULT_HARD_CAP } from "./utils";
import { makeSDK } from "./workspace";

describe("Famine", () => {
  let stakedMintAuthority: web3.Keypair;
  let stakeTokenMint: web3.PublicKey;
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
  });

  let rewardsMint: web3.PublicKey;
  let token: Token;
  let mintWrapperKey: web3.PublicKey;
  let hardCap: TokenAmount;

  beforeEach("Initialize mint", async () => {
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

  beforeEach("Set up rewarder and minter", async () => {
    const { tx: tx1, key: rewarder } = await mine.createRewarder({
      mintWrapper: mintWrapperKey,
      authority: provider.wallet.publicKey,
    });
    await expectTX(tx1, "Create new rewarder").to.be.fulfilled;
    rewarderWrapper = await mine.loadRewarderWrapper(rewarder);

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
  let minerKey: web3.PublicKey;

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
    const { miner, tx: tx2 } = await quarryWrapper.createMiner();
    await expectTX(tx2, "Create new miner").to.be.fulfilled;

    minerKey = miner;
  });

  it("Stake and claim after famine", async () => {
    const now = new BN(Date.now());
    await expectTX(
      rewarderWrapper.setFamine({
        newFamineTs: now,
        quarry: quarryWrapper.key,
      }),
      "Set famine"
    );
  });
});
