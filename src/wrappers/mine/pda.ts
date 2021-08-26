import { utils } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../../constants";

export const findRewarderAddress = async (
  base: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.Mine
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [Buffer.from(utils.bytes.utf8.encode("Rewarder")), base.toBytes()],
    programID
  );
};

export const findQuarryAddress = async (
  rewarder: PublicKey,
  tokenMint: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.Mine
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [
      Buffer.from(utils.bytes.utf8.encode("Quarry")),
      rewarder.toBytes(),
      tokenMint.toBytes(),
    ],
    programID
  );
};

export const findMinerAddress = async (
  quarry: PublicKey,
  authority: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.Mine
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [
      Buffer.from(utils.bytes.utf8.encode("Miner")),
      quarry.toBytes(),
      authority.toBytes(),
    ],
    programID
  );
};
