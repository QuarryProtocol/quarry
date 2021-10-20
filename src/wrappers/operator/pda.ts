import { utils } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../../constants";

export const findOperatorAddress = async (
  base: PublicKey,
  programID: PublicKey = QUARRY_ADDRESSES.Operator
): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [utils.bytes.utf8.encode("Operator"), base.toBytes()],
    programID
  );
};
