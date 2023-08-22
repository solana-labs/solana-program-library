import {
  PublicKey,
  TransactionInstruction,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  STAKE_CONFIG_ID,
  StakeProgram,
  SystemProgram,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { Buffer } from 'buffer';

import { MPL_METADATA_PROGRAM_ID, findMplMetadataAddress } from './mpl_metadata';
import {
  findPoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  findPoolStakeAuthorityAddress,
  findPoolMintAuthorityAddress,
  findPoolMplAuthorityAddress,
} from './addresses';
import {
  encodeData,
  SINGLE_POOL_PROGRAM_ID,
  SINGLE_POOL_INSTRUCTION_LAYOUTS,
  updateTokenMetadataLayout,
} from './internal';

export class SinglePoolInstruction {
  static initializePool(voteAccount: PublicKey): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;
    const pool = findPoolAddress(programId, voteAccount);

    const keys = [
      { pubkey: voteAccount, isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: findPoolStakeAddress(programId, pool), isSigner: false, isWritable: true },
      { pubkey: findPoolMintAddress(programId, pool), isSigner: false, isWritable: true },
      {
        pubkey: findPoolStakeAuthorityAddress(programId, pool),
        isSigner: false,
        isWritable: false,
      },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: STAKE_CONFIG_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.InitializePool;
    const data = encodeData(type);

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }

  static depositStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userTokenAccount: PublicKey,
    userLamportAccount: PublicKey,
  ): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;

    const keys = [
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: findPoolStakeAddress(programId, pool), isSigner: false, isWritable: true },
      { pubkey: findPoolMintAddress(programId, pool), isSigner: false, isWritable: true },
      {
        pubkey: findPoolStakeAuthorityAddress(programId, pool),
        isSigner: false,
        isWritable: false,
      },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: userStakeAccount, isSigner: false, isWritable: true },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: userLamportAccount, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.DepositStake;
    const data = encodeData(type);

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }

  static withdrawStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userStakeAuthority: PublicKey,
    userTokenAccount: PublicKey,
    userTokenAuthority: PublicKey,
    tokenAmount: number | bigint,
  ): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;

    const keys = [
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: findPoolStakeAddress(programId, pool), isSigner: false, isWritable: true },
      { pubkey: findPoolMintAddress(programId, pool), isSigner: false, isWritable: true },
      {
        pubkey: findPoolStakeAuthorityAddress(programId, pool),
        isSigner: false,
        isWritable: false,
      },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: userStakeAccount, isSigner: false, isWritable: true },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.WithdrawStake;
    const data = encodeData(type, {
      userStakeAuthority: userStakeAuthority.toBuffer(),
      tokenAmount,
    });

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }

  static createTokenMetadata(pool: PublicKey, payer: PublicKey): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;
    const mint = findPoolMintAddress(programId, pool);

    const keys = [
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: findPoolMplAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: findMplMetadataAddress(mint), isSigner: false, isWritable: true },
      { pubkey: MPL_METADATA_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.CreateTokenMetadata;
    const data = encodeData(type);

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }

  static updateTokenMetadata(
    voteAccount: PublicKey,
    authorizedWithdrawer: PublicKey,
    tokenName: string,
    tokenSymbol: string,
    tokenUri?: string,
  ): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;
    const pool = findPoolAddress(programId, voteAccount);

    tokenUri = tokenUri || '';

    const keys = [
      { pubkey: voteAccount, isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: findPoolMplAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: authorizedWithdrawer, isSigner: true, isWritable: false },
      {
        pubkey: findMplMetadataAddress(findPoolMintAddress(programId, pool)),
        isSigner: false,
        isWritable: true,
      },
      { pubkey: MPL_METADATA_PROGRAM_ID, isSigner: false, isWritable: false },
    ];

    const type = updateTokenMetadataLayout(tokenName.length, tokenSymbol.length, tokenUri.length);

    const data = encodeData(type, {
      tokenNameLen: tokenName.length,
      tokenName: Buffer.from(tokenName),
      tokenSymbolLen: tokenSymbol.length,
      tokenSymbol: Buffer.from(tokenSymbol),
      tokenUriLen: tokenUri.length,
      tokenUri: Buffer.from(tokenUri),
    });

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }
}
