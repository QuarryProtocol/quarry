import { getProvider } from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Token } from "@saberhq/token-utils";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  SPLToken,
  TOKEN_PROGRAM_ID,
} from "@saberhq/token-utils";
import type { Keypair, PublicKey, Signer } from "@solana/web3.js";
import { Transaction } from "@solana/web3.js";

import type { QuarrySDK, QuarryWrapper } from "../src";

export const DEFAULT_DECIMALS = 6;
export const DEFAULT_HARD_CAP = 1_000_000_000_000;

export const newUserStakeTokenAccount = async (
  sdk: QuarrySDK,
  quarry: QuarryWrapper,
  stakeToken: Token,
  stakedMintAuthority: Keypair,
  amount: number
): Promise<PublicKey> => {
  const minerActions = await quarry.getMinerActions(
    sdk.provider.wallet.publicKey
  );
  const createATA = await minerActions.createATAIfNotExists();
  if (createATA) {
    await expectTX(createATA, "create ATA").to.be.fulfilled;
  }

  const userStakeTokenAccount = minerActions.stakedTokenATA;
  await expectTX(
    sdk.newTx(
      [
        SPLToken.createMintToInstruction(
          TOKEN_PROGRAM_ID,
          stakeToken.mintAccount,
          userStakeTokenAccount,
          stakedMintAuthority.publicKey,
          [],
          amount
        ),
      ],
      [stakedMintAuthority]
    ),
    "mint initial"
  ).to.be.fulfilled;

  return userStakeTokenAccount;
};

export const initATA = async (
  token: Token,
  owner: Signer,
  mint?: { minter: Signer; mintAmount: number }
): Promise<PublicKey> => {
  const account = await SPLToken.getAssociatedTokenAddress(
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    token.mintAccount,
    owner.publicKey
  );

  const tx = new Transaction().add(
    SPLToken.createAssociatedTokenAccountInstruction(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      token.mintAccount,
      account,
      owner.publicKey,
      getProvider().wallet.publicKey
    )
  );

  if (mint) {
    tx.add(
      SPLToken.createMintToInstruction(
        TOKEN_PROGRAM_ID,
        token.mintAccount,
        account,
        mint.minter.publicKey,
        [],
        mint.mintAmount
      )
    );
  }
  // mint tokens
  await getProvider().send(tx, mint ? [mint.minter] : undefined, {
    commitment: "confirmed",
  });
  return account;
};
