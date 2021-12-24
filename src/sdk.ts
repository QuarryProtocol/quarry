import type { Program } from "@project-serum/anchor";
import { newProgramMap } from "@saberhq/anchor-contrib";
import type {
  AugmentedProvider,
  Provider,
  TransactionEnvelope,
} from "@saberhq/solana-contrib";
import { SolanaAugmentedProvider } from "@saberhq/solana-contrib";
import type {
  PublicKey,
  Signer,
  TransactionInstruction,
} from "@solana/web3.js";
import { Keypair } from "@solana/web3.js";

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
    readonly provider: AugmentedProvider,
    readonly programs: Programs
  ) {}

  /**
   * Creates a new instance of the SDK with the given keypair.
   */
  withSigner(signer: Signer): QuarrySDK {
    return QuarrySDK.load({
      provider: this.provider.withSigner(signer),
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
  newTx(
    instructions: TransactionInstruction[],
    signers?: Signer[]
  ): TransactionEnvelope {
    return this.provider.newTX(instructions, signers);
  }

  /**
   * Loads the SDK.
   * @returns
   */
  static load({
    provider,
    addresses = QUARRY_ADDRESSES,
  }: {
    // Provider
    provider: Provider;
    // Addresses of each program.
    addresses?: { [K in keyof Programs]?: PublicKey };
  }): QuarrySDK {
    const allAddresses = { ...QUARRY_ADDRESSES, ...addresses };
    const programs = newProgramMap<Programs>(
      provider,
      QUARRY_IDLS,
      allAddresses
    );
    return new QuarrySDK(new SolanaAugmentedProvider(provider), programs);
  }

  async loadRedeemer({
    iouMint,
    redemptionMint,
  }: {
    iouMint: PublicKey;
    redemptionMint: PublicKey;
  }): Promise<RedeemerWrapper> {
    return await RedeemerWrapper.load({ iouMint, redemptionMint, sdk: this });
  }

  async createRedeemer({
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
  async loadOperator(key: PublicKey): Promise<Operator | null> {
    return await Operator.load({
      sdk: this,
      key,
    });
  }

  /**
   * Creates an Operator.
   * @returns
   */
  async createOperator({
    rewarder,
    baseKP = Keypair.generate(),
    admin = this.provider.wallet.publicKey,
    payer = this.provider.wallet.publicKey,
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
