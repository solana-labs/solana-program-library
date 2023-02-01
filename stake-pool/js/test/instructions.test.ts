import {
  PublicKey,
  Connection,
  Keypair,
  SystemProgram,
  AccountInfo,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import { StakePoolLayout } from '../src/layouts';
import {
  STAKE_POOL_INSTRUCTION_LAYOUTS,
  STAKE_POOL_PROGRAM_ID,
  DepositSolParams,
  StakePoolInstruction,
  depositSol,
  withdrawSol,
  withdrawStake,
  redelegate,
  getStakeAccount,
} from '../src';

import { decodeData } from '../src/utils';

import {
  mockRpc,
  mockTokenAccount,
  mockValidatorList,
  mockValidatorsStakeAccount,
  stakePoolMock,
  CONSTANTS,
  stakeAccountData,
  uninitializedStakeAccount,
} from './mocks';

describe('StakePoolProgram', () => {
  const connection = new Connection('http://127.0.0.1:8899');

  connection.getMinimumBalanceForRentExemption = jest.fn(async () => 10000);

  const stakePoolAddress = new PublicKey('SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy');

  const data = Buffer.alloc(1024);
  StakePoolLayout.encode(stakePoolMock, data);

  const stakePoolAccount = <AccountInfo<any>>{
    executable: true,
    owner: stakePoolAddress,
    lamports: 99999,
    data,
  };

  it('StakePoolInstruction.depositSol', () => {
    const payload: DepositSolParams = {
      stakePool: stakePoolAddress,
      withdrawAuthority: Keypair.generate().publicKey,
      reserveStake: Keypair.generate().publicKey,
      fundingAccount: Keypair.generate().publicKey,
      destinationPoolAccount: Keypair.generate().publicKey,
      managerFeeAccount: Keypair.generate().publicKey,
      referralPoolAccount: Keypair.generate().publicKey,
      poolMint: Keypair.generate().publicKey,
      lamports: 99999,
    };

    const instruction = StakePoolInstruction.depositSol(payload);

    expect(instruction.keys).toHaveLength(10);
    expect(instruction.keys[0].pubkey.toBase58()).toEqual(payload.stakePool.toBase58());
    expect(instruction.keys[1].pubkey.toBase58()).toEqual(payload.withdrawAuthority.toBase58());
    expect(instruction.keys[3].pubkey.toBase58()).toEqual(payload.fundingAccount.toBase58());
    expect(instruction.keys[4].pubkey.toBase58()).toEqual(
      payload.destinationPoolAccount.toBase58(),
    );
    expect(instruction.keys[5].pubkey.toBase58()).toEqual(payload.managerFeeAccount.toBase58());
    expect(instruction.keys[6].pubkey.toBase58()).toEqual(payload.referralPoolAccount.toBase58());
    expect(instruction.keys[8].pubkey.toBase58()).toEqual(SystemProgram.programId.toBase58());
    expect(instruction.keys[9].pubkey.toBase58()).toEqual(STAKE_POOL_PROGRAM_ID.toBase58());

    const decodedData = decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol, instruction.data);

    expect(decodedData.instruction).toEqual(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol.index);
    expect(decodedData.lamports).toEqual(payload.lamports);

    payload.depositAuthority = Keypair.generate().publicKey;

    const instruction2 = StakePoolInstruction.depositSol(payload);

    expect(instruction2.keys).toHaveLength(11);
    expect(instruction2.keys[10].pubkey.toBase58()).toEqual(payload.depositAuthority.toBase58());
  });

  describe('depositSol', () => {
    const from = Keypair.generate().publicKey;
    const balance = 10000;

    connection.getBalance = jest.fn(async () => balance);

    connection.getAccountInfo = jest.fn(async (pubKey) => {
      if (pubKey == stakePoolAddress) {
        return stakePoolAccount;
      }
      return <AccountInfo<any>>{
        executable: true,
        owner: from,
        lamports: balance,
        data: null,
      };
    });

    it.only('should throw an error with invalid balance', async () => {
      await expect(depositSol(connection, stakePoolAddress, from, balance + 1)).rejects.toThrow(
        Error('Not enough SOL to deposit into pool. Maximum deposit amount is 0.00001 SOL.'),
      );
    });

    it.only('should throw an error with invalid account', async () => {
      connection.getAccountInfo = jest.fn(async () => null);
      await expect(depositSol(connection, stakePoolAddress, from, balance)).rejects.toThrow(
        Error('Invalid stake pool account'),
      );
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        return <AccountInfo<any>>{
          executable: true,
          owner: from,
          lamports: balance,
          data: null,
        };
      });

      const res = await depositSol(connection, stakePoolAddress, from, balance);

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(2);
      expect(res.instructions).toHaveLength(2);
      expect(res.signers).toHaveLength(1);
    });
  });

  describe('withdrawSol', () => {
    const tokenOwner = new PublicKey(0);
    const solReceiver = new PublicKey(1);

    it.only('should throw an error with invalid stake pool account', async () => {
      connection.getAccountInfo = jest.fn(async () => null);
      await expect(
        withdrawSol(connection, stakePoolAddress, tokenOwner, solReceiver, 1),
      ).rejects.toThrowError('Invalid stake pool account');
    });

    it.only('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(CONSTANTS.poolTokenAccount)) {
          return null;
        }
        return null;
      });

      await expect(
        withdrawSol(connection, stakePoolAddress, tokenOwner, solReceiver, 1),
      ).rejects.toThrow(Error('Invalid token account'));
    });

    it.only('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(CONSTANTS.poolTokenAccount)) {
          return mockTokenAccount(0);
        }
        return null;
      });

      await expect(
        withdrawSol(connection, stakePoolAddress, tokenOwner, solReceiver, 1),
      ).rejects.toThrow(
        Error(
          'Not enough token balance to withdraw 1 pool tokens.\n          Maximum withdraw amount is 0 pool tokens.',
        ),
      );
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(CONSTANTS.poolTokenAccount)) {
          return mockTokenAccount(LAMPORTS_PER_SOL);
        }
        return null;
      });
      const res = await withdrawSol(connection, stakePoolAddress, tokenOwner, solReceiver, 1);

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(2);
      expect(res.instructions).toHaveLength(2);
      expect(res.signers).toHaveLength(1);
    });
  });

  describe('withdrawStake', () => {
    const tokenOwner = new PublicKey(0);

    it.only('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        return null;
      });

      await expect(withdrawStake(connection, stakePoolAddress, tokenOwner, 1)).rejects.toThrow(
        Error('Invalid token account'),
      );
    });

    it.only('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(CONSTANTS.poolTokenAccount)) {
          return mockTokenAccount(0);
        }
        return null;
      });

      await expect(withdrawStake(connection, stakePoolAddress, tokenOwner, 1)).rejects.toThrow(
        Error(
          'Not enough token balance to withdraw 1 pool tokens.\n' +
            '        Maximum withdraw amount is 0 pool tokens.',
        ),
      );
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(CONSTANTS.poolTokenAccount)) {
          return mockTokenAccount(LAMPORTS_PER_SOL * 2);
        }
        if (pubKey.equals(stakePoolMock.validatorList)) {
          return mockValidatorList();
        }
        return null;
      });
      const res = await withdrawStake(connection, stakePoolAddress, tokenOwner, 1);

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(4);
      expect(res.instructions).toHaveLength(3);
      expect(res.signers).toHaveLength(2);
      expect(res.stakeReceiver).toEqual(undefined);
      expect(res.totalRentFreeBalances).toEqual(10000);
    });

    it.only('withdraw to a stake account provided', async () => {
      const stakeReceiver = new PublicKey(20);
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(CONSTANTS.poolTokenAccount)) {
          return mockTokenAccount(LAMPORTS_PER_SOL * 2);
        }
        if (pubKey.equals(stakePoolMock.validatorList)) {
          return mockValidatorList();
        }
        if (pubKey.equals(CONSTANTS.validatorStakeAccountAddress))
          return mockValidatorsStakeAccount();
        return null;
      });
      connection.getParsedAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey.equals(stakeReceiver)) {
          return mockRpc(stakeAccountData);
        }
        return null;
      });

      const res = await withdrawStake(
        connection,
        stakePoolAddress,
        tokenOwner,
        1,
        undefined,
        undefined,
        stakeReceiver,
      );

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(4);
      expect((connection.getParsedAccountInfo as jest.Mock).mock.calls.length).toBe(1);
      expect(res.instructions).toHaveLength(3);
      expect(res.signers).toHaveLength(2);
      expect(res.stakeReceiver).toEqual(stakeReceiver);
      expect(res.totalRentFreeBalances).toEqual(10000);
    });
  });
  describe('getStakeAccount', () => {
    it.only('returns an uninitialized parsed stake account', async () => {
      const stakeAccount = new PublicKey(20);
      connection.getParsedAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey.equals(stakeAccount)) {
          return mockRpc(uninitializedStakeAccount);
        }
        return null;
      });
      const parsedStakeAccount = await getStakeAccount(connection, stakeAccount);
      expect((connection.getParsedAccountInfo as jest.Mock).mock.calls.length).toBe(1);
      expect(parsedStakeAccount).toEqual(uninitializedStakeAccount.parsed);
    });
  });

  describe('redelegation', () => {
    it.only('should call successfully', async () => {
      const data = {
        connection,
        stakePoolAddress,
        sourceVoteAccount: PublicKey.default,
        sourceTransientStakeSeed: 10,
        destinationVoteAccount: PublicKey.default,
        destinationTransientStakeSeed: 20,
        ephemeralStakeSeed: 100,
        lamports: 100,
      };
      const res = await redelegate(data);

      const decodedData = STAKE_POOL_INSTRUCTION_LAYOUTS.Redelegate.layout.decode(
        res.instructions[0].data,
      );

      expect(decodedData.instruction).toBe(21);
      expect(decodedData.lamports).toBe(data.lamports);
      expect(decodedData.sourceTransientStakeSeed).toBe(data.sourceTransientStakeSeed);
      expect(decodedData.destinationTransientStakeSeed).toBe(data.destinationTransientStakeSeed);
      expect(decodedData.ephemeralStakeSeed).toBe(data.ephemeralStakeSeed);
    });
  });
});
