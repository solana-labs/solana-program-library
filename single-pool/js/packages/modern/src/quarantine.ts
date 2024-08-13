import { address, getAddressCodec, getProgramDerivedAddress, Address } from '@solana/addresses';
import { AccountRole } from '@solana/instructions';

// HERE BE DRAGONS
// this is all the stuff that shouldn't be in our library once we can import from elsewhere

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
    from: Address;
    newAccount: Address;
    lamports: bigint;
    space: bigint;
    programAddress: Address;
  }) {
    const { encode } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(0),
      ...u64(params.lamports),
      ...u64(params.space),
      ...encode(params.programAddress),
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

  static transfer(params: { from: Address; to: Address; lamports: bigint }) {
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
    from: Address;
    newAccount: Address;
    base: Address;
    seed: string;
    lamports: bigint;
    space: bigint;
    programAddress: Address;
  }) {
    const { encode } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(3),
      ...encode(params.base),
      ...u64(BigInt(params.seed.length)),
      ...new TextEncoder().encode(params.seed),
      ...u64(params.lamports),
      ...u64(params.space),
      ...encode(params.programAddress),
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
  static approve(params: { account: Address; delegate: Address; owner: Address; amount: bigint }) {
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
    payer: Address;
    associatedAccount: Address;
    owner: Address;
    mint: Address;
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
  static initialize(params: { stakeAccount: Address; staker: Address; withdrawer: Address }) {
    const { encode } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(0),
      ...encode(params.staker),
      ...encode(params.withdrawer),
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
    stakeAccount: Address;
    authorized: Address;
    newAuthorized: Address;
    authorizationType: StakeAuthorizationType;
    custodian?: Address;
  }) {
    const { encode } = getAddressCodec();
    const data = new Uint8Array([
      ...u32(1),
      ...encode(params.newAuthorized),
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

  static delegate(params: { stakeAccount: Address; authorized: Address; voteAccount: Address }) {
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

export async function getAssociatedTokenAddress(mint: Address, owner: Address) {
  const { encode } = getAddressCodec();
  const [pda] = await getProgramDerivedAddress({
    programAddress: ATOKEN_PROGRAM_ID,
    seeds: [encode(owner), encode(TOKEN_PROGRAM_ID), encode(mint)],
  });

  return pda;
}
