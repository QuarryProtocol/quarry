import type { AugmentedProvider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import type { TokenAmount, u64 } from "@saberhq/token-utils";
import {
  createInitMintInstructions,
  getOrCreateATA,
  TOKEN_PROGRAM_ID,
} from "@saberhq/token-utils";
import type { AccountInfo, PublicKey } from "@solana/web3.js";
import { Keypair, SystemProgram } from "@solana/web3.js";

import type {
  MinterData,
  MintWrapperData,
  MintWrapperProgram,
} from "../../programs/mintWrapper";
import type { QuarrySDK } from "../../sdk";
import { findMinterAddress, findMintWrapperAddress } from "./pda";
import type { PendingMintAndWrapper, PendingMintWrapper } from "./types";

export class MintWrapper {
  readonly program: MintWrapperProgram;

  constructor(readonly sdk: QuarrySDK) {
    this.program = sdk.programs.MintWrapper;
  }

  get provider(): AugmentedProvider {
    return this.sdk.provider;
  }

  async newWrapper({
    hardcap,
    tokenMint,
    baseKP = Keypair.generate(),
    tokenProgram = TOKEN_PROGRAM_ID,
    admin = this.program.provider.wallet.publicKey,
    payer = this.program.provider.wallet.publicKey,
  }: {
    hardcap: u64;
    tokenMint: PublicKey;
    baseKP?: Keypair;
    tokenProgram?: PublicKey;
    admin?: PublicKey;
    payer?: PublicKey;
  }): Promise<PendingMintWrapper> {
    const [mintWrapper, nonce] = await findMintWrapperAddress(
      baseKP.publicKey,
      this.program.programId
    );
    return {
      mintWrapper,
      tx: new TransactionEnvelope(
        this.provider,
        [
          this.program.instruction.newWrapper(nonce, hardcap, {
            accounts: {
              base: baseKP.publicKey,
              mintWrapper,
              admin,
              tokenMint,
              tokenProgram,
              payer,
              systemProgram: SystemProgram.programId,
            },
          }),
        ],
        [baseKP]
      ),
    };
  }

  async newWrapperAndMint({
    mintKP = Keypair.generate(),
    decimals = 6,
    ...newWrapperArgs
  }: {
    mintKP?: Keypair;
    decimals?: number;

    hardcap: u64;
    baseKP?: Keypair;
    tokenProgram?: PublicKey;
    admin?: PublicKey;
    payer?: PublicKey;
  }): Promise<PendingMintAndWrapper> {
    const provider = this.provider;
    const { mintWrapper, tx: initMintProxyTX } = await this.newWrapper({
      ...newWrapperArgs,
      tokenMint: mintKP.publicKey,
    });
    const initMintTX = await createInitMintInstructions({
      provider,
      mintAuthority: mintWrapper,
      freezeAuthority: mintWrapper,
      mintKP,
      decimals,
    });
    return {
      mintWrapper,
      mint: mintKP.publicKey,
      tx: initMintTX.combine(initMintProxyTX),
    };
  }

  /**
   * Fetches info on a Mint Wrapper.
   * @param minter
   * @returns
   */
  async fetchMintWrapper(wrapper: PublicKey): Promise<MintWrapperData | null> {
    const accountInfo = await this.program.provider.connection.getAccountInfo(
      wrapper
    );
    if (!accountInfo) {
      return null;
    }
    return this.program.coder.accounts.decode<MintWrapperData>(
      "MintWrapper",
      accountInfo.data
    );
  }

  /**
   * Fetches info on a minter.
   * @param minter
   * @returns
   */
  async fetchMinter(
    wrapper: PublicKey,
    authority: PublicKey
  ): Promise<MinterData | null> {
    const [minterAddress] = await findMinterAddress(
      wrapper,
      authority,
      this.program.programId
    );
    const accountInfo = await this.program.provider.connection.getAccountInfo(
      minterAddress
    );
    if (!accountInfo) {
      return null;
    }
    return this.program.coder.accounts.decode<MinterData>(
      "Minter",
      accountInfo.data
    );
  }

  async newMinter(
    wrapper: PublicKey,
    authority: PublicKey
  ): Promise<TransactionEnvelope> {
    const [minter, bump] = await findMinterAddress(
      wrapper,
      authority,
      this.program.programId
    );
    return this.provider.newTX([
      this.program.instruction.newMinter(bump, {
        accounts: {
          auth: {
            mintWrapper: wrapper,
            admin: this.program.provider.wallet.publicKey,
          },
          minterAuthority: authority,
          minter,
          payer: this.program.provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        },
      }),
    ]);
  }

  /**
   * Updates a minter's allowance.
   * @param minter
   * @param allowance
   * @returns
   */
  async minterUpdate(
    wrapper: PublicKey,
    authority: PublicKey,
    allowance: u64
  ): Promise<TransactionEnvelope> {
    const [minter] = await findMinterAddress(
      wrapper,
      authority,
      this.program.programId
    );
    return this.provider.newTX([
      this.program.instruction.minterUpdate(allowance, {
        accounts: {
          auth: {
            mintWrapper: wrapper,
            admin: this.program.provider.wallet.publicKey,
          },
          minter,
        },
      }),
    ]);
  }

  /**
   * Creates a new Minter with an allowance.
   * @param wrapper
   * @param authority
   * @param allowance
   * @returns
   */
  async newMinterWithAllowance(
    wrapper: PublicKey,
    authority: PublicKey,
    allowance: u64
  ): Promise<TransactionEnvelope> {
    const newMinter = await this.newMinter(wrapper, authority);
    const updateAllowance = await this.minterUpdate(
      wrapper,
      authority,
      allowance
    );
    return newMinter.combine(updateAllowance);
  }

  transferAdmin(wrapper: PublicKey, nextAdmin: PublicKey): TransactionEnvelope {
    return this.provider.newTX([
      this.program.instruction.transferAdmin({
        accounts: {
          mintWrapper: wrapper,
          admin: this.program.provider.wallet.publicKey,
          nextAdmin,
        },
      }),
    ]);
  }

  acceptAdmin(wrapper: PublicKey): TransactionEnvelope {
    return this.provider.newTX([
      this.program.instruction.acceptAdmin({
        accounts: {
          mintWrapper: wrapper,
          pendingAdmin: this.program.provider.wallet.publicKey,
        },
      }),
    ]);
  }

  /**
   * Mints tokens to an address as a Minter on the Mint Wrapper.
   */
  async performMintTo({
    amount,
    mintWrapper,
    minterAuthority = this.provider.wallet.publicKey,
    destOwner = this.provider.wallet.publicKey,
  }: {
    amount: TokenAmount;
    mintWrapper: PublicKey;
    minterAuthority?: PublicKey;
    destOwner?: PublicKey;
  }): Promise<TransactionEnvelope> {
    const ata = await getOrCreateATA({
      provider: this.provider,
      mint: amount.token.mintAccount,
      owner: destOwner,
    });
    const [minter] = await findMinterAddress(mintWrapper, minterAuthority);
    return this.sdk.provider.newTX([
      ata.instruction,
      this.program.instruction.performMint(amount.toU64(), {
        accounts: {
          mintWrapper,
          minterAuthority,
          tokenMint: amount.token.mintAccount,
          destination: ata.address,
          minter,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }),
    ]);
  }

  /**
   * Performs a mint of tokens to an account.
   * @returns
   */
  async performMint({
    amount,
    minter,
  }: {
    amount: TokenAmount;
    minter: {
      accountId: PublicKey;
      accountInfo: AccountInfo<MinterData>;
    };
  }): Promise<TransactionEnvelope> {
    const minterData = minter.accountInfo.data;
    const ata = await getOrCreateATA({
      provider: this.provider,
      mint: amount.token.mintAccount,
      owner: this.provider.wallet.publicKey,
    });
    return this.provider.newTX([
      ata.instruction,
      this.program.instruction.performMint(amount.toU64(), {
        accounts: {
          mintWrapper: minterData.mintWrapper,
          minterAuthority: minterData.minterAuthority,
          tokenMint: amount.token.mintAccount,
          destination: ata.address,
          minter: minter.accountId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }),
    ]);
  }
}
