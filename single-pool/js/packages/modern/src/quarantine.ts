import {
  address,
  getAddressCodec,
  Base58EncodedAddress,
  AccountRole,
  getProgramDerivedAddress,
} from '@solana/web3.js';

// HERE BE DRAGONS
// this is all the stuff that shouldnt be in our library once we can import from elsewhere

export const SYSTEM_PROGRAM_ID = address('11111111111111111111111111111111');
export const STAKE_PROGRAM_ID = address('Stake11111111111111111111111111111111111111');
export const SYSVAR_RENT_ID = address('SysvarRent111111111111111111111111111111111');
export const SYSVAR_CLOCK_ID = address('SysvarC1ock11111111111111111111111111111111');
export const SYSVAR_STAKE_HISTORY_ID = address('SysvarStakeHistory1111111111111111111111111');
export const STAKE_CONFIG_ID = address('StakeConfig11111111111111111111111111111111');
export const STAKE_ACCOUNT_SIZE = 200n;

export const TOKEN_PROGRAM_ID = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
export const ATOKEN_PROGRAM_ID = address('ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL');
export const MINT_SIZE = 82n;

export function u32(n: number): Uint8Array {
  const bns = Uint32Array.from([n]);
  return new Uint8Array(bns.buffer);
}

export function u64(n: bigint): Uint8Array {
  const bns = BigUint64Array.from([n]);
  return new Uint8Array(bns.buffer);
}

export class SystemInstruction {
  static createAccount(params: {
    from: Base58EncodedAddress;
    newAccount: Base58EncodedAddress;
    lamports: bigint;
    space: bigint;
    programAddress: Base58EncodedAddress;
  }) {
    const { serialize } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(0),
      ...u64(params.lamports),
      ...u64(params.space),
      ...serialize(params.programAddress),
    ]);

    const accounts = [
      { address: params.from, role: AccountRole.WRITABLE_SIGNER },
      { address: params.newAccount, role: AccountRole.WRITABLE_SIGNER },
    ];

    return {
      data,
      accounts,
      programAddress: SYSTEM_PROGRAM_ID,
    };
  }

  static transfer(params: {
    from: Base58EncodedAddress;
    to: Base58EncodedAddress;
    lamports: bigint;
  }) {
    const data = new Uint8Array([...u32(2), ...u64(params.lamports)]);

    const accounts = [
      { address: params.from, role: AccountRole.WRITABLE_SIGNER },
      { address: params.to, role: AccountRole.WRITABLE },
    ];

    return {
      data,
      accounts,
      programAddress: SYSTEM_PROGRAM_ID,
    };
  }

  static createAccountWithSeed(params: {
    from: Base58EncodedAddress;
    newAccount: Base58EncodedAddress;
    base: Base58EncodedAddress;
    seed: string;
    lamports: bigint;
    space: bigint;
    programAddress: Base58EncodedAddress;
  }) {
    const { serialize } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(3),
      ...serialize(params.base),
      ...u64(BigInt(params.seed.length)),
      ...new TextEncoder().encode(params.seed),
      ...u64(params.lamports),
      ...u64(params.space),
      ...serialize(params.programAddress),
    ]);

    const accounts = [
      { address: params.from, role: AccountRole.WRITABLE_SIGNER },
      { address: params.newAccount, role: AccountRole.WRITABLE },
    ];
    if (params.base != params.from) {
      accounts.push({ address: params.base, role: AccountRole.READONLY_SIGNER });
    }

    return {
      data,
      accounts,
      programAddress: SYSTEM_PROGRAM_ID,
    };
  }
}

export class TokenInstruction {
  static approve(params: {
    account: Base58EncodedAddress;
    delegate: Base58EncodedAddress;
    owner: Base58EncodedAddress;
    amount: bigint;
  }) {
    const data = new Uint8Array([...u32(4), ...u64(params.amount)]);

    const accounts = [
      { address: params.account, role: AccountRole.WRITABLE },
      { address: params.delegate, role: AccountRole.READONLY },
      { address: params.owner, role: AccountRole.READONLY_SIGNER },
    ];

    return {
      data,
      accounts,
      programAddress: TOKEN_PROGRAM_ID,
    };
  }

  static createAssociatedTokenAccount(params: {
    payer: Base58EncodedAddress;
    associatedAccount: Base58EncodedAddress;
    owner: Base58EncodedAddress;
    mint: Base58EncodedAddress;
  }) {
    const data = new Uint8Array([0]);

    const accounts = [
      { address: params.payer, role: AccountRole.WRITABLE_SIGNER },
      { address: params.associatedAccount, role: AccountRole.WRITABLE },
      { address: params.owner, role: AccountRole.READONLY },
      { address: params.mint, role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ID, role: AccountRole.READONLY },
    ];

    return {
      data,
      accounts,
      programAddress: ATOKEN_PROGRAM_ID,
    };
  }
}

export enum StakeAuthorizationType {
  Staker,
  Withdrawer,
}

export class StakeInstruction {
  // idc about doing it right unless this goes in a lib
  static initialize(params: {
    stakeAccount: Base58EncodedAddress;
    staker: Base58EncodedAddress;
    withdrawer: Base58EncodedAddress;
  }) {
    const { serialize } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(0),
      ...serialize(params.staker),
      ...serialize(params.withdrawer),
      ...Array(48).fill(0),
    ]);

    const accounts = [
      { address: params.stakeAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_RENT_ID, role: AccountRole.READONLY },
    ];

    return {
      data,
      accounts,
      programAddress: STAKE_PROGRAM_ID,
    };
  }

  static authorize(params: {
    stakeAccount: Base58EncodedAddress;
    authorized: Base58EncodedAddress;
    newAuthorized: Base58EncodedAddress;
    authorizationType: StakeAuthorizationType;
    custodian?: Base58EncodedAddress;
  }) {
    const { serialize } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(1),
      ...serialize(params.newAuthorized),
      ...u32(params.authorizationType),
    ]);

    const accounts = [
      { address: params.stakeAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_CLOCK_ID, role: AccountRole.READONLY },
      { address: params.authorized, role: AccountRole.READONLY_SIGNER },
    ];
    if (params.custodian) {
      accounts.push({ address: params.custodian, role: AccountRole.READONLY });
    }

    return {
      data,
      accounts,
      programAddress: STAKE_PROGRAM_ID,
    };
  }

  static delegate(params: {
    stakeAccount: Base58EncodedAddress;
    authorized: Base58EncodedAddress;
    voteAccount: Base58EncodedAddress;
  }) {
    const data = new Uint8Array(u32(2));

    const accounts = [
      { address: params.stakeAccount, role: AccountRole.WRITABLE },
      { address: params.voteAccount, role: AccountRole.READONLY },
      { address: SYSVAR_CLOCK_ID, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ID, role: AccountRole.READONLY },
      { address: STAKE_CONFIG_ID, role: AccountRole.READONLY },
      { address: params.authorized, role: AccountRole.READONLY_SIGNER },
    ];

    return {
      data,
      accounts,
      programAddress: STAKE_PROGRAM_ID,
    };
  }
}

export async function getAssociatedTokenAddress(
  mint: Base58EncodedAddress,
  owner: Base58EncodedAddress,
) {
  const { serialize } = getAddressCodec();
  const [pda] = await getProgramDerivedAddress({
    programAddress: ATOKEN_PROGRAM_ID,
    seeds: [serialize(owner), serialize(TOKEN_PROGRAM_ID), serialize(mint)],
  });

  return pda;
}
