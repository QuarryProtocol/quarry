import { utils } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

import { QUARRY_ADDRESSES } from "../../constants";

/**
 * Finds the address of the Pool.
 * @returns
 */
export const findPoolAddress = async ({
  programId = QUARRY_ADDRESSES.MergeMine,
  primaryMint,
}: {
  programId?: PublicKey;
  primaryMint: PublicKey;
}): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [utils.bytes.utf8.encode("MergePool"), primaryMint.toBuffer()],
    programId
  );
};

/**
 * Finds the address of the Pool.
 * @returns
 */
export const findReplicaMintAddress = async ({
  programId = QUARRY_ADDRESSES.MergeMine,
  primaryMint,
}: {
  programId?: PublicKey;
  primaryMint: PublicKey;
}): Promise<[PublicKey, number]> => {
  const [pool] = await findPoolAddress({ programId, primaryMint });
  return await PublicKey.findProgramAddress(
    [utils.bytes.utf8.encode("ReplicaMint"), pool.toBuffer()],
    programId
  );
};

/**
 * Finds the address of the Merge Miner.
 * @returns
 */
export const findMergeMinerAddress = async ({
  programId = QUARRY_ADDRESSES.MergeMine,
  pool,
  owner,
}: {
  programId?: PublicKey;
  pool: PublicKey;
  owner: PublicKey;
}): Promise<[PublicKey, number]> => {
  return await PublicKey.findProgramAddress(
    [utils.bytes.utf8.encode("MergeMiner"), pool.toBuffer(), owner.toBuffer()],
    programId
  );
};
