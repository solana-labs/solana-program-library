import {
  PublicKey,
  Connection,
  Keypair,
  SystemProgram,
  AccountInfo,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import {StakePoolLayout} from '../src/layouts';
import {STAKE_POOL_PROGRAM_ID} from '../src/constants';
import {decodeData} from '../src/copied-from-solana-web3/instruction';
import {
  STAKE_POOL_INSTRUCTION_LAYOUTS,
  DepositSolParams,
  StakePoolInstruction,
  depositSol,
  withdrawSol,
  withdrawStake,
} from '../src';

import {mockTokenAccount, mockValidatorList, stakePoolMock} from './mocks';

describe('StakePoolProgram', () => {
  const connection = new Connection('http://127.0.0.1:8899');

  connection.getMinimumBalanceForRentExemption = jest.fn(async () => 10000);

  const stakePoolPubkey = new PublicKey(
    'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
  );

  const data = Buffer.alloc(1024);
  StakePoolLayout.encode(stakePoolMock, data);

  const stakePoolAccount = <AccountInfo<any>>{
    executable: true,
    owner: stakePoolPubkey,
    lamports: 99999,
    data,
  };

  it('StakePoolInstruction.depositSol', () => {
    const payload: DepositSolParams = {
      stakePool: stakePoolPubkey,
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
    expect(instruction.keys[0].pubkey.toBase58()).toEqual(
      payload.stakePool.toBase58(),
    );
    expect(instruction.keys[1].pubkey.toBase58()).toEqual(
      payload.withdrawAuthority.toBase58(),
    );
    expect(instruction.keys[3].pubkey.toBase58()).toEqual(
      payload.fundingAccount.toBase58(),
    );
    expect(instruction.keys[4].pubkey.toBase58()).toEqual(
      payload.destinationPoolAccount.toBase58(),
    );
    expect(instruction.keys[5].pubkey.toBase58()).toEqual(
      payload.managerFeeAccount.toBase58(),
    );
    expect(instruction.keys[6].pubkey.toBase58()).toEqual(
      payload.referralPoolAccount.toBase58(),
    );
    expect(instruction.keys[8].pubkey.toBase58()).toEqual(
      SystemProgram.programId.toBase58(),
    );
    expect(instruction.keys[9].pubkey.toBase58()).toEqual(
      STAKE_POOL_PROGRAM_ID.toBase58(),
    );

    const decodedData = decodeData(
      STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol,
      instruction.data,
    );

    expect(decodedData.instruction).toEqual(
      STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol.index,
    );
    expect(decodedData.lamports).toEqual(payload.lamports);

    payload.depositAuthority = Keypair.generate().publicKey;

    const instruction2 = StakePoolInstruction.depositSol(payload);

    expect(instruction2.keys).toHaveLength(11);
    expect(instruction2.keys[10].pubkey.toBase58()).toEqual(
      payload.depositAuthority.toBase58(),
    );
  });

  describe('depositSol', () => {
    const from = Keypair.generate().publicKey;
    const balance = 10000;

    connection.getBalance = jest.fn(async () => balance);

    connection.getAccountInfo = jest.fn(async pubKey => {
      if (pubKey == stakePoolPubkey) {
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
      await expect(
        depositSol(connection, stakePoolPubkey, from, balance + 1),
      ).rejects.toThrow(
        Error(
          'Not enough SOL to deposit into pool. Maximum deposit amount is 0.00001 SOL.',
        ),
      );
    });

    it.only('should throw an error with invalid account', async () => {
      connection.getAccountInfo = jest.fn(async () => null);
      await expect(
        depositSol(connection, stakePoolPubkey, from, balance),
      ).rejects.toThrow(Error('Invalid account'));
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async pubKey => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        return <AccountInfo<any>>{
          executable: true,
          owner: from,
          lamports: balance,
          data: null,
        };
      });

      const res = await depositSol(connection, stakePoolPubkey, from, balance);

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(
        2,
      );
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
        withdrawSol(connection, stakePoolPubkey, tokenOwner, solReceiver, 1),
      ).rejects.toThrowError('Invalid account');
    });

    it.only('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        if (
          pubKey.toBase58() == '9q2rZU5RujvyD9dmYKhzJAZfG4aGBbvQ8rWY52jCNBai'
        ) {
          return null;
        }
        return null;
      });

      await expect(
        withdrawSol(connection, stakePoolPubkey, tokenOwner, solReceiver, 1),
      ).rejects.toThrow(Error('Invalid token account'));
    });

    it.only('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        if (
          pubKey.toBase58() == 'GQkqTamwqjaNDfsbNm7r3aXPJ4oTSqKC3d5t2PF9Smqd'
        ) {
          return mockTokenAccount(0);
        }
        return null;
      });

      await expect(
        withdrawSol(connection, stakePoolPubkey, tokenOwner, solReceiver, 1),
      ).rejects.toThrow(
        Error(
          'Not enough token balance to withdraw 1 pool tokens.\n          Maximum withdraw amount is 0 pool tokens.',
        ),
      );
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        if (
          pubKey.toBase58() == 'GQkqTamwqjaNDfsbNm7r3aXPJ4oTSqKC3d5t2PF9Smqd'
        ) {
          return mockTokenAccount(LAMPORTS_PER_SOL);
        }
        return null;
      });
      const res = await withdrawSol(
        connection,
        stakePoolPubkey,
        tokenOwner,
        solReceiver,
        1,
      );

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(
        2,
      );
      expect(res.instructions).toHaveLength(2);
      expect(res.signers).toHaveLength(1);
    });
  });

  describe('withdrawStake', () => {
    const tokenOwner = new PublicKey(0);

    it.only('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        return null;
      });

      await expect(
        withdrawStake(connection, stakePoolPubkey, tokenOwner, 1),
      ).rejects.toThrow(Error('Invalid token account'));
    });

    it.only('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        if (
          pubKey.toBase58() == 'GQkqTamwqjaNDfsbNm7r3aXPJ4oTSqKC3d5t2PF9Smqd'
        ) {
          return mockTokenAccount(0);
        }
        return null;
      });

      await expect(
        withdrawStake(connection, stakePoolPubkey, tokenOwner, 1),
      ).rejects.toThrow(
        Error(
          'Not enough token balance to withdraw 1 pool tokens.\n' +
            '        Maximum withdraw amount is 0 pool tokens.',
        ),
      );
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolPubkey) {
          return stakePoolAccount;
        }
        if (
          pubKey.toBase58() == 'GQkqTamwqjaNDfsbNm7r3aXPJ4oTSqKC3d5t2PF9Smqd'
        ) {
          return mockTokenAccount(LAMPORTS_PER_SOL * 2);
        }
        if (pubKey.toBase58() == stakePoolMock.validatorList.toBase58()) {
          return mockValidatorList();
        }
        return null;
      });

      const res = await withdrawStake(
        connection,
        stakePoolPubkey,
        tokenOwner,
        1,
      );

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(
        4,
      );
      expect(res.instructions).toHaveLength(3);
      expect(res.signers).toHaveLength(2);
      expect(res.stakeReceiver).toEqual(undefined);
      expect(res.totalRentFreeBalances).toEqual(10000);
    });
  });
});
