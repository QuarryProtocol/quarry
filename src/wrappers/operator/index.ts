import { TransactionEnvelope } from "@saberhq/solana-contrib";
import { u64 } from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, SystemProgram, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";

import type { OperatorData, QuarryOperatorProgram, QuarrySDK } from "../..";
import { findQuarryAddress } from "..";
import { findOperatorAddress } from "./pda";

/**
 * Operator helper functions.
 */
export class Operator {
  constructor(
    readonly sdk: QuarrySDK,
    readonly key: PublicKey,
    readonly data: OperatorData
  ) {}

  get program(): QuarryOperatorProgram {
    return this.sdk.programs.Operator;
  }

  /**
   * Reloads the Operator's data.
   * @returns
   */
  async reload(): Promise<Operator> {
    const data = await this.program.account.operator.fetch(this.key);
    return new Operator(this.sdk, this.key, data);
  }

  static async load({
    sdk,
    key,
  }: {
    sdk: QuarrySDK;
    key: PublicKey;
  }): Promise<Operator | null> {
    const program = sdk.programs.Operator;
    const data = (await program.account.operator.fetchNullable(
      key
    )) as OperatorData;
    if (!data) {
      return null;
    }
    return new Operator(sdk, key, data);
  }

  static async createOperator({
    sdk,
    rewarder,
    baseKP = Keypair.generate(),
    admin = sdk.provider.wallet.publicKey,
    payer = sdk.provider.wallet.publicKey,
  }: {
    sdk: QuarrySDK;
    rewarder: PublicKey;
    admin?: PublicKey;
    baseKP?: Keypair;
    payer?: PublicKey;
  }): Promise<{
    key: PublicKey;
    tx: TransactionEnvelope;
  }> {
    const [operatorKey, bump] = await findOperatorAddress(
      baseKP.publicKey,
      sdk.programs.Operator.programId
    );
    return {
      key: operatorKey,
      tx: new TransactionEnvelope(
        sdk.provider,
        [
          sdk.programs.Operator.instruction.createOperator(bump, {
            accounts: {
              base: baseKP.publicKey,
              operator: operatorKey,
              rewarder,
              admin,

              payer,
              systemProgram: SystemProgram.programId,
              quarryMineProgram: sdk.programs.Mine.programId,
            },
          }),
        ],
        [baseKP]
      ),
    };
  }

  setAdmin(delegate: PublicKey): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.setAdmin({
        accounts: {
          operator: this.key,
          admin: this.sdk.provider.wallet.publicKey,
          delegate,
        },
      }),
    ]);
  }

  setRateSetter(delegate: PublicKey): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.setRateSetter({
        accounts: {
          operator: this.key,
          admin: this.sdk.provider.wallet.publicKey,
          delegate,
        },
      }),
    ]);
  }

  setQuarryCreator(delegate: PublicKey): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.setQuarryCreator({
        accounts: {
          operator: this.key,
          admin: this.sdk.provider.wallet.publicKey,
          delegate,
        },
      }),
    ]);
  }

  setShareAllocator(delegate: PublicKey): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.setShareAllocator({
        accounts: {
          operator: this.key,
          admin: this.sdk.provider.wallet.publicKey,
          delegate,
        },
      }),
    ]);
  }

  get withDelegateAccounts(): {
    operator: PublicKey;
    delegate: PublicKey;
    rewarder: PublicKey;
    quarryMineProgram: PublicKey;
  } {
    return {
      operator: this.key,
      delegate: this.sdk.provider.wallet.publicKey,
      rewarder: this.data.rewarder,
      quarryMineProgram: this.sdk.programs.Mine.programId,
    };
  }

  delegateSetAnnualRewards(newAnnualRate: u64): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.delegateSetAnnualRewards(newAnnualRate, {
        accounts: {
          withDelegate: this.withDelegateAccounts,
        },
      }),
    ]);
  }

  delegateSetFamine(newFamineTs: u64, quarry: PublicKey): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.delegateSetFamine(newFamineTs, {
        accounts: {
          withDelegate: this.withDelegateAccounts,
          quarry,
        },
      }),
    ]);
  }

  async delegateCreateQuarry({
    tokenMint,
    payer = this.sdk.provider.wallet.publicKey,
  }: {
    tokenMint: PublicKey;
    payer?: PublicKey;
  }): Promise<{ tx: TransactionEnvelope; quarry: PublicKey }> {
    const [quarry, bump] = await findQuarryAddress(
      this.data.rewarder,
      tokenMint,
      this.sdk.programs.Mine.programId
    );
    return {
      quarry,
      tx: new TransactionEnvelope(this.sdk.provider, [
        this.program.instruction.delegateCreateQuarry(bump, {
          accounts: {
            withDelegate: this.withDelegateAccounts,
            quarry,
            tokenMint,
            payer,
            unusedClock: SYSVAR_CLOCK_PUBKEY,
            systemProgram: SystemProgram.programId,
          },
        }),
      ]),
    };
  }

  delegateSetRewardsShare({
    share,
    quarry,
  }: {
    share: number;
    quarry: PublicKey;
  }): TransactionEnvelope {
    return new TransactionEnvelope(this.sdk.provider, [
      this.program.instruction.delegateSetRewardsShare(new u64(share), {
        accounts: {
          withDelegate: this.withDelegateAccounts,
          quarry,
        },
      }),
    ]);
  }
}
