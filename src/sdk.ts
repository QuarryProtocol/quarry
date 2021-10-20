import type { Address } from "@project-serum/anchor";
import { Program, Provider as AnchorProvider } from "@project-serum/anchor";
import type { Provider } from "@saberhq/solana-contrib";
import {
  DEFAULT_PROVIDER_OPTIONS,
  SignerWallet,
  SolanaProvider,
  TransactionEnvelope,
} from "@saberhq/solana-contrib";
import type {
  ConfirmOptions,
  Keypair,
  PublicKey,
  Signer,
  TransactionInstruction,
} from "@solana/web3.js";
import mapValues from "lodash.mapvalues";

import type { Programs } from "./constants";
import { QUARRY_ADDRESSES, QUARRY_IDLS } from "./constants";
import type { PendingRedeemer } from "./programs/redeemer";
import {
  MergeMine,
  MineWrapper,
  MintWrapper,
  QuarryRegistry,
} from "./wrappers";
import { Operator } from "./wrappers/operator";
import { RedeemerWrapper } from "./wrappers/redeemer";

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
  public withSigner(signer: Signer): QuarrySDK {
    const wallet = new SignerWallet(signer);
    const provider = new SolanaProvider(
      this.provider.connection,
      this.provider.broadcaster,
      wallet,
      this.provider.opts
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

  get registry(): QuarryRegistry {
    return new QuarryRegistry(this);
  }

  get mergeMine(): MergeMine {
    return new MergeMine(this);
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
    confirmOptions = DEFAULT_PROVIDER_OPTIONS,
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
        const anchorProvider = new AnchorProvider(
          provider.connection,
          provider.wallet,
          confirmOptions
        );
        return new Program(idl, address, anchorProvider) as Program;
      }
    ) as unknown as Programs;
    return new QuarrySDK(provider, programs);
  }

  public async loadRedeemer({
    iouMint,
    redemptionMint,
  }: {
    iouMint: PublicKey;
    redemptionMint: PublicKey;
  }): Promise<RedeemerWrapper> {
    return await RedeemerWrapper.load({ iouMint, redemptionMint, sdk: this });
  }

  public async createRedeemer({
    iouMint,
    redemptionMint,
  }: {
    iouMint: PublicKey;
    redemptionMint: PublicKey;
  }): Promise<PendingRedeemer> {
    return await RedeemerWrapper.createRedeemer({
      iouMint,
      redemptionMint,
      sdk: this,
    });
  }

  /**
   * Loads an operator.
   * @param key
   * @returns
   */
  public async loadOperator(key: PublicKey): Promise<Operator | null> {
    return await Operator.load({
      sdk: this,
      key,
    });
  }

  public async createOperator({
    rewarder,
    baseKP,
    admin,
    payer,
  }: {
    rewarder: PublicKey;
    admin?: PublicKey;
    baseKP?: Keypair;
    payer?: PublicKey;
  }): Promise<{ key: PublicKey; tx: TransactionEnvelope }> {
    return await Operator.createOperator({
      sdk: this,
      rewarder,
      baseKP,
      admin,
      payer,
    });
  }
}
