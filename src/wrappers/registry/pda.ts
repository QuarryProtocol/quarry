import { utils } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../../constants";

export const findRegistryAddress = async (
  rewarderKey: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.Registry
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [utils.bytes.utf8.encode("QuarryRegistry"), rewarderKey.toBytes()],
    programID
  );
};
