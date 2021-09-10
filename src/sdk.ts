import type { Address } from "@project-serum/anchor";
import { Program, Provider as AnchorProvider } from "@project-serum/anchor";
import type { Provider } from "@saberhq/solana-contrib";
import {
  DEFAULT_PROVIDER_OPTIONS,
  SignerWallet,
  TransactionEnvelope,
} from "@saberhq/solana-contrib";
import type {
  ConfirmOptions,
  PublicKey,
  Signer,
  TransactionInstruction,
} from "@solana/web3.js";
import { mapValues } from "lodash";
import invariant from "tiny-invariant";

import type { Programs } from "./constants";
import { QUARRY_ADDRESSES, QUARRY_IDLS } from "./constants";
import { MineWrapper, MintWrapper } from "./wrappers";

export interface Environment {
  rewarder: PublicKey;
  landlord: PublicKey;
  creator: PublicKey;
}

/**
 * Quarry SDK.
 */
export class QuarrySDK {
  constructor(
    public readonly provider: Provider,
    public readonly programs: Programs
  ) {}

  /**
   * Creates a new instance of the SDK with the given keypair.
   */
  public withKeypair(keypair: Signer): QuarrySDK {
    const provider = new SignerWallet(keypair).createProvider(
      this.provider.connection,
      this.provider.sendConnection
    );
    return QuarrySDK.load({
      provider,
      addresses: mapValues(this.programs, (v) => v.programId),
    });
  }

  get programList(): Program[] {
    return Object.values(this.programs) as Program[];
  }

  get mintWrapper(): MintWrapper {
    return new MintWrapper(this);
  }

  get mine(): MineWrapper {
    return new MineWrapper(this);
  }

  /**
   * Constructs a new transaction envelope.
   * @param instructions
   * @param signers
   * @returns
   */
  public newTx(
    instructions: TransactionInstruction[],
    signers?: Signer[]
  ): TransactionEnvelope {
    return new TransactionEnvelope(this.provider, instructions, signers);
  }

  /**
   * Loads the SDK.
   * @returns
   */
  public static load({
    provider,
    addresses = QUARRY_ADDRESSES,
    confirmOptions,
  }: {
    // Provider
    provider: Provider;
    // Addresses of each program.
    addresses?: { [K in keyof Programs]?: Address };
    confirmOptions?: ConfirmOptions;
  }): QuarrySDK {
    const allAddresses = { ...QUARRY_ADDRESSES, ...addresses };
    const programs: Programs = mapValues(
      QUARRY_ADDRESSES,
      (_: Address, programName: keyof Programs): Program => {
        const address = allAddresses[programName];
        const idl = QUARRY_IDLS[programName];
        invariant(idl, `Unknown IDL: ${programName}`);
        const anchorProvider = new AnchorProvider(
          provider.sendConnection,
          provider.wallet,
          confirmOptions ?? DEFAULT_PROVIDER_OPTIONS
        );
        return new Program(idl, address, anchorProvider);
      }
    ) as unknown as Programs;
    return new QuarrySDK(provider, programs);
  }
}
