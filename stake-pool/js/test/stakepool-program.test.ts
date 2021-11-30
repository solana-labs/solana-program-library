import {
  PublicKey,
  Connection,
  Keypair,
  clusterApiUrl,
  SystemProgram, AccountInfo, LAMPORTS_PER_SOL
} from '@solana/web3.js';
import {
  STAKE_POOL_INSTRUCTION_LAYOUTS,
  DepositSolParams,
  StakePoolProgram,
} from '../src/stakepool-program';
import { decodeData } from '../src/copied-from-solana-web3/instruction';
import { depositSol, withdrawSol, withdrawStake } from "../src";
import { STAKE_POOL_LAYOUT } from "../src/layouts";
import { mockTokenAccount, mockValidatorList, stakePoolMock } from "./mocks";

describe('StakePoolProgram', () => {

  const connection = new Connection(
    clusterApiUrl('devnet'),
    'confirmed',
  );

  connection.getMinimumBalanceForRentExemption = jest.fn(async () => 10000);

  const STAKE_POOL_PROGRAM_ADDR = new PublicKey(
    'SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy',
  );

  const data = Buffer.alloc(1024);
  STAKE_POOL_LAYOUT.encode(stakePoolMock, data)

  const stakePoolAccount = <AccountInfo<any>>{
    executable: true,
    owner: STAKE_POOL_PROGRAM_ADDR,
    lamports: 99999,
    data,
  };

  it('depositSolInstruction', () => {

    const payload: DepositSolParams = {
      stakePoolPubkey: STAKE_POOL_PROGRAM_ADDR,
      withdrawAuthority: Keypair.generate().publicKey,
      reserveStakeAccount: Keypair.generate().publicKey,
      lamportsFrom: Keypair.generate().publicKey,
      poolTokensTo: Keypair.generate().publicKey,
      managerFeeAccount: Keypair.generate().publicKey,
      referrerPoolTokensAccount: Keypair.generate().publicKey,
      poolMint: Keypair.generate().publicKey,
      lamports: 99999,
    };

    const instruction = StakePoolProgram.depositSolInstruction(payload);

    expect(instruction.keys).toHaveLength(10);
    expect(instruction.keys[0].pubkey.toBase58()).toEqual(payload.stakePoolPubkey.toBase58());
    expect(instruction.keys[1].pubkey.toBase58()).toEqual(payload.withdrawAuthority.toBase58());
    expect(instruction.keys[3].pubkey.toBase58()).toEqual(payload.lamportsFrom.toBase58());
    expect(instruction.keys[4].pubkey.toBase58()).toEqual(payload.poolTokensTo.toBase58());
    expect(instruction.keys[5].pubkey.toBase58()).toEqual(payload.managerFeeAccount.toBase58());
    expect(instruction.keys[6].pubkey.toBase58()).toEqual(payload.referrerPoolTokensAccount.toBase58());
    expect(instruction.keys[8].pubkey.toBase58()).toEqual(SystemProgram.programId.toBase58());
    expect(instruction.keys[9].pubkey.toBase58()).toEqual(StakePoolProgram.tokenProgramId.toBase58());

    const decodedData = decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol, instruction.data);

    expect(decodedData.instruction).toEqual(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol.index);
    expect(decodedData.lamports).toEqual(payload.lamports);

    payload.depositAuthority = Keypair.generate().publicKey;

    const instruction2 = StakePoolProgram.depositSolInstruction(payload);

    expect(instruction2.keys).toHaveLength(11);
    expect(instruction2.keys[10].pubkey.toBase58()).toEqual(payload.depositAuthority.toBase58());

  });

  describe('depositSol', () => {
    const from = Keypair.generate().publicKey;
    const balance = 10000;

    connection.getBalance = jest.fn(async () => balance);

    connection.getAccountInfo = jest.fn(async (pubKey) => {
      if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
        return stakePoolAccount
      }
      return <AccountInfo<any>>{
        executable: true,
        owner: from,
        lamports: balance,
        data: null,
      }
    });

    it.only('should throw an error with invalid balance', async () => {
      await expect(
        depositSol(connection, STAKE_POOL_PROGRAM_ADDR, from, balance + 1)
      ).rejects.toThrow(Error('Not enough SOL to deposit into pool. Maximum deposit amount is 0.00001 SOL.'));
    });

    it.only('should throw an error with invalid account', async () => {
      connection.getAccountInfo = jest.fn(async () => null);
      await expect(
        depositSol(connection, STAKE_POOL_PROGRAM_ADDR, from, balance)
      ).rejects.toThrow(Error('Invalid account'));
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        return <AccountInfo<any>>{
          executable: true,
          owner: from,
          lamports: balance,
          data: null,
        }
      });

      const res = await depositSol(connection, STAKE_POOL_PROGRAM_ADDR, from, balance)

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
        withdrawSol(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, solReceiver, 1)
      ).rejects.toThrowError('Invalid account');
    });

    it.only('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        if (pubKey.toBase58() == '9q2rZU5RujvyD9dmYKhzJAZfG4aGBbvQ8rWY52jCNBai') {
          return null
        }
        return null;
      });

      await expect(
        withdrawSol(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, solReceiver, 1)
      ).rejects.toThrow(Error('Invalid token account'));
    });

    it.only('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        if (pubKey.toBase58() == '9q2rZU5RujvyD9dmYKhzJAZfG4aGBbvQ8rWY52jCNBai') {
          return mockTokenAccount(0);
        }
        return null;
      });
      await expect(
        withdrawSol(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, solReceiver, 1)
      ).rejects.toThrow(Error('Not enough token balance to withdraw 1 pool tokens.\n          Maximum withdraw amount is 0 pool tokens.'));
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        if (pubKey.toBase58() == '9q2rZU5RujvyD9dmYKhzJAZfG4aGBbvQ8rWY52jCNBai') {
          return mockTokenAccount(LAMPORTS_PER_SOL);
        }
        return null;
      });
      const res = await withdrawSol(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, solReceiver, 1)

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(2);
      expect(res.instructions).toHaveLength(2);
      expect(res.signers).toHaveLength(1);
    });

  })

  describe('withdrawStake', () => {

    const tokenOwner = new PublicKey(0);

    it.only('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        return null;
      });

      await expect(
        withdrawStake(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, 1)
      ).rejects.toThrow(Error('Invalid token account'));
    });

    it.only('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        if (pubKey.toBase58() == '9q2rZU5RujvyD9dmYKhzJAZfG4aGBbvQ8rWY52jCNBai') {
          return mockTokenAccount(0);
        }
        return null;
      });

      await expect(
        withdrawStake(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, 1)
      ).rejects.toThrow(Error('Not enough token balance to withdraw 1 pool tokens.\n' +
        '          Maximum withdraw amount is 0 pool tokens.'));
    });

    it.only('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        console.log(pubKey.toBase58());
        if (pubKey == STAKE_POOL_PROGRAM_ADDR) {
          return stakePoolAccount
        }
        if (pubKey.toBase58() == '9q2rZU5RujvyD9dmYKhzJAZfG4aGBbvQ8rWY52jCNBai') {
          return mockTokenAccount(LAMPORTS_PER_SOL * 2);
        }
        if (pubKey.toBase58() == stakePoolMock.validatorList.toBase58()) {
          return mockValidatorList();
        }
        return null;
      });

      const res = await withdrawStake(connection, STAKE_POOL_PROGRAM_ADDR, tokenOwner, 1);

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(4);
      expect(res.instructions).toHaveLength(3);
      expect(res.signers).toHaveLength(2);
      expect(res.stakeReceiver).toEqual(undefined);
      expect(res.totalRentFreeBalances).toEqual(10000);
    });

  });

});
