import "chai-bn";

import * as anchor from "@project-serum/anchor";
import { expectTX } from "@saberhq/chai-solana";
import type { Provider } from "@saberhq/solana-contrib";
import { PendingTransaction } from "@saberhq/solana-contrib";
import {
  createInitMintInstructions,
  getMintInfo,
  Token,
  TokenAmount,
  u64,
} from "@saberhq/token-utils";
import type { PublicKey } from "@solana/web3.js";
import { Keypair, LAMPORTS_PER_SOL } from "@solana/web3.js";
import * as assert from "assert";
import { expect } from "chai";

import type { MintWrapper, MintWrapperProgram, QuarrySDK } from "../src";
import { findMinterAddress } from "../src";
import { DEFAULT_DECIMALS, DEFAULT_HARD_CAP } from "./utils";
import { makeSDK } from "./workspace";

describe("MintWrapper", () => {
  const { BN, web3 } = anchor;

  let sdk: QuarrySDK;
  let provider: Provider;
  let mintWrapper: MintWrapper;
  let MintWrapper: MintWrapperProgram;

  before("Initialize SDK", () => {
    sdk = makeSDK();
    provider = sdk.provider;
    mintWrapper = sdk.mintWrapper;
    MintWrapper = sdk.programs.MintWrapper;
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

  it("Check MintWrapper", async () => {
    const mintInfo = await getMintInfo(provider, rewardsMint);
    assert.ok(mintInfo.mintAuthority?.equals(mintWrapperKey));

    const mintWrapperState =
      await mintWrapper.program.account.mintWrapper.fetch(mintWrapperKey);
    expect(mintWrapperState.hardCap).to.bignumber.eq(hardCap.toU64());
    expect(mintWrapperState.admin).to.eqAddress(provider.wallet.publicKey);
    expect(mintWrapperState.tokenMint).to.eqAddress(rewardsMint);
  });

  describe("MintWrapper", () => {
    it("Transfer admin authority and accept admin authority", async () => {
      const newAuthority = web3.Keypair.generate();

      await assert.doesNotReject(async () => {
        await MintWrapper.rpc.transferAdmin({
          accounts: {
            mintWrapper: mintWrapperKey,
            admin: provider.wallet.publicKey,
            nextAdmin: newAuthority.publicKey,
          },
        });
      });

      let mintWrapperState =
        await mintWrapper.program.account.mintWrapper.fetch(mintWrapperKey);
      expect(mintWrapperState.admin).to.eqAddress(provider.wallet.publicKey);
      expect(mintWrapperState.pendingAdmin).to.eqAddress(
        newAuthority.publicKey
      );

      const ix = mintWrapper.program.instruction.acceptAdmin({
        accounts: {
          mintWrapper: mintWrapperKey,
          pendingAdmin: newAuthority.publicKey,
        },
      });
      let tx = sdk.newTx([ix], [newAuthority]);
      await expectTX(tx, "transfer authority").to.be.fulfilled;
      mintWrapperState = await mintWrapper.program.account.mintWrapper.fetch(
        mintWrapperKey
      );
      expect(mintWrapperState.admin).to.eqAddress(newAuthority.publicKey);
      expect(mintWrapperState.pendingAdmin).to.eqAddress(
        web3.PublicKey.default.toString()
      );

      // Transfer back
      const instructions = [];
      instructions.push(
        mintWrapper.program.instruction.transferAdmin({
          accounts: {
            mintWrapper: mintWrapperKey,
            admin: newAuthority.publicKey,
            nextAdmin: provider.wallet.publicKey,
          },
        })
      );
      instructions.push(
        mintWrapper.program.instruction.acceptAdmin({
          accounts: {
            mintWrapper: mintWrapperKey,
            pendingAdmin: provider.wallet.publicKey,
          },
        })
      );

      tx = sdk.newTx(instructions, [newAuthority]);
      await expectTX(tx, "transfer authority back to original authority").to.be
        .fulfilled;

      mintWrapperState = await mintWrapper.program.account.mintWrapper.fetch(
        mintWrapperKey
      );
      expect(mintWrapperState.admin).to.eqAddress(provider.wallet.publicKey);
      expect(mintWrapperState.pendingAdmin).to.eqAddress(
        web3.PublicKey.default
      );
    });

    it("Adds a Minter", async () => {
      const allowance = new u64(1_000_000);
      const id = Keypair.generate().publicKey;
      expect(
        (await mintWrapper.fetchMintWrapper(mintWrapperKey))?.numMinters,
        "initial num minters"
      ).to.bignumber.eq(new BN(0));

      await expectTX(
        mintWrapper.newMinterWithAllowance(mintWrapperKey, id, allowance),
        "add minter"
      ).to.be.fulfilled;
      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "allowance"
      ).to.bignumber.eq(allowance);

      expect(
        (await mintWrapper.fetchMintWrapper(mintWrapperKey))?.numMinters,
        "final num minters"
      ).to.bignumber.eq(new BN(1));
    });

    it("Removes a Minter", async () => {
      const allowance = new u64(1_000_000);
      const id = Keypair.generate().publicKey;
      await expectTX(
        mintWrapper.newMinterWithAllowance(mintWrapperKey, id, allowance),
        "add minter"
      ).to.be.fulfilled;

      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "allowance"
      ).to.bignumber.eq(allowance);

      await expectTX(
        mintWrapper.minterUpdate(mintWrapperKey, id, new u64(0)),
        "remove minter"
      ).to.be.fulfilled;
      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "no more allowance"
      ).to.bignumber.zero;
    });

    it("Cannot mint past allowance", async () => {
      const allowance = new u64(1_000_000);

      const kp = Keypair.generate();

      await expectTX(
        new PendingTransaction(
          provider.connection,
          await provider.connection.requestAirdrop(
            kp.publicKey,
            LAMPORTS_PER_SOL
          )
        )
      ).to.be.fulfilled;

      const id = kp.publicKey;
      await expectTX(
        mintWrapper.newMinterWithAllowance(mintWrapperKey, id, allowance),
        "add minter"
      ).to.be.fulfilled;

      expect(
        (await mintWrapper.fetchMinter(mintWrapperKey, id))?.allowance,
        "allowance"
      ).to.bignumber.eq(allowance);

      const amount = new TokenAmount(token, new u64(1_000));
      const minterSDK = sdk.withSigner(kp);
      const [minterAddress] = await findMinterAddress(mintWrapperKey, id);
      const minterData =
        await minterSDK.programs.MintWrapper.account.minter.fetch(
          minterAddress
        );
      const minterRaw = await provider.connection.getAccountInfo(minterAddress);
      assert.ok(minterRaw);

      await expectTX(
        minterSDK.mintWrapper.performMint({
          amount,
          minter: {
            accountId: minterAddress,
            accountInfo: {
              ...minterRaw,
              data: minterData,
            },
          },
        }),
        "mint"
      ).to.be.fulfilled;

      const minterData2 = await mintWrapper.fetchMinter(mintWrapperKey, id);
      const mwData2 = await mintWrapper.fetchMintWrapper(mintWrapperKey);
      assert.ok(minterData2 && mwData2);
      expect(minterData2.allowance, "allowance").to.bignumber.eq(
        new BN(999_000)
      );
      expect(minterData2.totalMinted, "minter total minted").to.bignumber.eq(
        new BN(1_000)
      );

      expect(mwData2.totalAllowance, "total allowance").to.bignumber.eq(
        new BN(999_000)
      );
      expect(mwData2.totalMinted, "total minted").to.bignumber.eq(
        new BN(1_000)
      );

      await expectTX(
        minterSDK.mintWrapper.performMint({
          amount: new TokenAmount(amount.token, allowance),
          minter: {
            accountId: minterAddress,
            accountInfo: {
              ...minterRaw,
              data: minterData,
            },
          },
        }),
        "mint"
      ).to.be.rejected;
    });
  });
});
