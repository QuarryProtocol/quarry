import type { Provider } from "@saberhq/solana-contrib";
import { TransactionEnvelope } from "@saberhq/solana-contrib";
import { createInitMintInstructions } from "@saberhq/token-utils";
import type { u64 } from "@solana/spl-token";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, SystemProgram } from "@solana/web3.js";

import type {
  MinterData,
  MintWrapperProgram,
} from "../../programs/mintWrapper";
import type { QuarrySDK } from "../../sdk";
import { findMinterAddress, findMintWrapperAddress } from "./pda";
import type { PendingMintAndWrapper, PendingMintWrapper } from "./types";

export class MintWrapper {
  public readonly program: MintWrapperProgram;

  constructor(public readonly sdk: QuarrySDK) {
    this.program = sdk.programs.MintWrapper;
  }

  get provider(): Provider {
    return this.sdk.provider;
  }

  public async newWrapper({
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

  public async newWrapperAndMint({
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
   * Fetches info on a minter.
   * @param minter
   * @returns
   */
  public async fetchMinter(
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

  public async newMinter(
    wrapper: PublicKey,
    authority: PublicKey,
    allowance: u64
  ): Promise<TransactionEnvelope> {
    const [minter, bump] = await findMinterAddress(
      wrapper,
      authority,
      this.program.programId
    );
    return this.sdk.newTx([
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
   * Updates a minter's allowance.
   * @param minter
   * @param allowance
   * @returns
   */
  public async minterUpdate(
    wrapper: PublicKey,
    authority: PublicKey,
    allowance: u64
  ): Promise<TransactionEnvelope> {
    const [minter] = await findMinterAddress(
      wrapper,
      authority,
      this.program.programId
    );
    return this.sdk.newTx([
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

  public transferAdmin(
    wrapper: PublicKey,
    nextAdmin: PublicKey
  ): TransactionEnvelope {
    return this.sdk.newTx([
      this.program.instruction.transferAdmin({
        accounts: {
          mintWrapper: wrapper,
          admin: this.program.provider.wallet.publicKey,
          nextAdmin,
        },
      }),
    ]);
  }

  public acceptAdmin(wrapper: PublicKey): TransactionEnvelope {
    return this.sdk.newTx([
      this.program.instruction.acceptAdmin({
        accounts: {
          mintWrapper: wrapper,
          pendingAdmin: this.program.provider.wallet.publicKey,
        },
      }),
    ]);
  }
}
