import { chaiSolana, expectTX } from "@saberhq/chai-solana";
import {
  createMint,
  createMintToInstruction,
  getOrCreateATAs,
  getTokenAccount,
  u64,
  ZERO,
} from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { expect, use } from "chai";
import invariant from "tiny-invariant";

import type { QuarrySDK } from "../src";
import { DEFAULT_DECIMALS } from "./utils";
import { makeSDK } from "./workspace";

use(chaiSolana);

describe("Redeemer", () => {
  const sdk: QuarrySDK = makeSDK();
  const provider = sdk.provider;

  let redeemerBump: number;

  let userAuthority: Keypair;

  let iouMint: PublicKey;
  let iouMintAuthority: Keypair;
  let iouSource: PublicKey;

  let redemptionMint: PublicKey;
  let redemptionMintAuthority: Keypair;
  let redemptionDestination: PublicKey;

  beforeEach(async () => {
    iouMintAuthority = Keypair.generate();
    iouMint = await createMint(sdk.provider, iouMintAuthority.publicKey);

    redemptionMintAuthority = Keypair.generate();
    redemptionMint = await createMint(
      sdk.provider,
      redemptionMintAuthority.publicKey
    );

    userAuthority = Keypair.generate();
    // Airdrop to user
    await sdk.provider.connection.requestAirdrop(
      userAuthority.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const { bump, tx, vaultTokenAccount } = await sdk.createRedeemer({
      iouMint,
      redemptionMint,
    });

    const { accounts, createAccountInstructions } = await getOrCreateATAs({
      provider,
      mints: {
        iouMint,
        redemptionMint,
      },
      owner: userAuthority.publicKey,
    });

    invariant(
      createAccountInstructions.iouMint,
      "create user ATA account for iouMint"
    );
    invariant(
      createAccountInstructions.redemptionMint,
      "create user ATA account for redemptionMint"
    );
    tx.instructions.push(
      createAccountInstructions.iouMint,
      createAccountInstructions.redemptionMint
    );
    tx.instructions.push(
      ...createMintToInstruction({
        provider,
        mint: redemptionMint,
        mintAuthorityKP: redemptionMintAuthority,
        to: vaultTokenAccount,
        amount: new u64(1_000 * DEFAULT_DECIMALS),
      }).instructions,
      ...createMintToInstruction({
        provider,
        mint: iouMint,
        mintAuthorityKP: iouMintAuthority,
        to: accounts.iouMint,
        amount: new u64(1_000 * DEFAULT_DECIMALS),
      }).instructions
    );
    tx.addSigners(iouMintAuthority, redemptionMintAuthority);
    await expectTX(tx, "create redeemer").to.be.fulfilled;

    iouSource = accounts.iouMint;

    redeemerBump = bump;
    redemptionDestination = accounts.redemptionMint;
  });

  it("Redeemer was initialized", async () => {
    const { data } = await sdk.loadRedeemer({
      iouMint,
      redemptionMint,
    });
    expect(data.bump).to.equal(redeemerBump);
    expect(data.iouMint).to.eqAddress(iouMint);
    expect(data.redemptionMint).to.eqAddress(redemptionMint);
  });

  it("Redeem tokens", async () => {
    const redeemerWrapper = await sdk.loadRedeemer({
      iouMint,
      redemptionMint,
    });

    let iouSourceAccount = await getTokenAccount(provider, iouSource);
    const expectedAmount = iouSourceAccount.amount;

    const tx = await redeemerWrapper.redeemTokens({
      tokenAmount: new u64(1_000 * DEFAULT_DECIMALS),
      sourceAuthority: userAuthority.publicKey,
      iouSource,
      redemptionDestination,
    });
    await expectTX(tx.addSigners(userAuthority), "redeem").to.be.fulfilled;

    iouSourceAccount = await getTokenAccount(provider, iouSource);
    expect(iouSourceAccount.amount.toString()).to.equal(ZERO.toString());
    const redemptionDestinationAccount = await getTokenAccount(
      provider,
      redemptionDestination
    );
    expect(redemptionDestinationAccount.amount.toString()).to.equal(
      expectedAmount.toString()
    );
  });
});
