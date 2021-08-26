import { utils } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../../constants";

export const findMintWrapperAddress = async (
  base: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.MintWrapper
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [Buffer.from(utils.bytes.utf8.encode("MintWrapper")), base.toBytes()],
    programID
  );
};

export const findMinterAddress = async (
  wrapper: PublicKey,
  authority: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.MintWrapper
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [
      Buffer.from(utils.bytes.utf8.encode("MintWrapperMinter")),
      wrapper.toBytes(),
      authority.toBytes(),
    ],
    programID
  );
};
