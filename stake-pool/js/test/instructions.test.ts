// Very important! We need to do this polyfill before any of the imports because
// some web3.js dependencies store `crypto` elsewhere.
import { randomBytes } from 'crypto';
Object.defineProperty(globalThis, 'crypto', {
  value: {
    getRandomValues: (arr: any) => randomBytes(arr.length),
  },
});

import {
  PublicKey,
  Connection,
  Keypair,
  SystemProgram,
  StakeProgram,
  AccountInfo,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, TokenAccountNotFoundError } from '@solana/spl-token';
import { StakePoolLayout, ValidatorListLayout } from '../src/layouts';
import {
  STAKE_POOL_INSTRUCTION_LAYOUTS,
  DepositSolParams,
  AddValidatorToPoolParams,
  RemoveValidatorFromPoolParams,
  StakePoolInstruction,
  depositSol,
  withdrawSol,
  withdrawStake,
  getStakeAccount,
  createPoolTokenMetadata,
  updatePoolTokenMetadata,
  tokenMetadataLayout,
  addValidatorToPool,
  removeValidatorFromPool,
} from '../src';
import { STAKE_POOL_PROGRAM_ID } from '../src/constants';

import { decodeData, findStakeProgramAddress } from '../src/utils';

import {
  mockRpc,
  mockTokenAccount,
  mockValidatorList,
  mockValidatorsStakeAccount,
  stakePoolMock,
  CONSTANTS,
  stakeAccountData,
  uninitializedStakeAccount,
  validatorListMock,
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

  it('StakePoolInstruction.addValidatorToPool', () => {
    const payload: AddValidatorToPoolParams = {
      stakePool: stakePoolAddress,
      staker: Keypair.generate().publicKey,
      reserveStake: Keypair.generate().publicKey,
      withdrawAuthority: Keypair.generate().publicKey,
      validatorList: Keypair.generate().publicKey,
      validatorStake: Keypair.generate().publicKey,
      validatorVote: PublicKey.default,
      seed: 0,
    };

    const instruction = StakePoolInstruction.addValidatorToPool(payload);
    expect(instruction.keys).toHaveLength(13);
    expect(instruction.keys[0].pubkey).toEqual(payload.stakePool);
    expect(instruction.keys[1].pubkey).toEqual(payload.staker);
    expect(instruction.keys[2].pubkey).toEqual(payload.reserveStake);
    expect(instruction.keys[3].pubkey).toEqual(payload.withdrawAuthority);
    expect(instruction.keys[4].pubkey).toEqual(payload.validatorList);
    expect(instruction.keys[5].pubkey).toEqual(payload.validatorStake);
    expect(instruction.keys[6].pubkey).toEqual(payload.validatorVote);
    expect(instruction.keys[11].pubkey).toEqual(SystemProgram.programId);
    expect(instruction.keys[12].pubkey).toEqual(StakeProgram.programId);

    const decodedData = decodeData(
      STAKE_POOL_INSTRUCTION_LAYOUTS.AddValidatorToPool,
      instruction.data,
    );
    expect(decodedData.instruction).toEqual(
      STAKE_POOL_INSTRUCTION_LAYOUTS.AddValidatorToPool.index,
    );
    expect(decodedData.seed).toEqual(payload.seed);
  });

  it('StakePoolInstruction.removeValidatorFromPool', () => {
    const payload: RemoveValidatorFromPoolParams = {
      stakePool: stakePoolAddress,
      staker: Keypair.generate().publicKey,
      withdrawAuthority: Keypair.generate().publicKey,
      validatorList: Keypair.generate().publicKey,
      validatorStake: Keypair.generate().publicKey,
      transientStake: Keypair.generate().publicKey,
    };

    const instruction = StakePoolInstruction.removeValidatorFromPool(payload);
    expect(instruction.keys).toHaveLength(8);
    expect(instruction.keys[0].pubkey).toEqual(payload.stakePool);
    expect(instruction.keys[1].pubkey).toEqual(payload.staker);
    expect(instruction.keys[2].pubkey).toEqual(payload.withdrawAuthority);
    expect(instruction.keys[3].pubkey).toEqual(payload.validatorList);
    expect(instruction.keys[4].pubkey).toEqual(payload.validatorStake);
    expect(instruction.keys[5].pubkey).toEqual(payload.transientStake);
    expect(instruction.keys[7].pubkey).toEqual(StakeProgram.programId);

    const decodedData = decodeData(
      STAKE_POOL_INSTRUCTION_LAYOUTS.RemoveValidatorFromPool,
      instruction.data,
    );
    expect(decodedData.instruction).toEqual(
      STAKE_POOL_INSTRUCTION_LAYOUTS.RemoveValidatorFromPool.index,
    );
  });

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
    expect(instruction.keys[0].pubkey).toEqual(payload.stakePool);
    expect(instruction.keys[1].pubkey).toEqual(payload.withdrawAuthority);
    expect(instruction.keys[3].pubkey).toEqual(payload.fundingAccount);
    expect(instruction.keys[4].pubkey).toEqual(payload.destinationPoolAccount);
    expect(instruction.keys[5].pubkey).toEqual(payload.managerFeeAccount);
    expect(instruction.keys[6].pubkey).toEqual(payload.referralPoolAccount);
    expect(instruction.keys[8].pubkey).toEqual(SystemProgram.programId);
    expect(instruction.keys[9].pubkey).toEqual(TOKEN_PROGRAM_ID);

    const decodedData = decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol, instruction.data);

    expect(decodedData.instruction).toEqual(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol.index);
    expect(decodedData.lamports).toEqual(payload.lamports);

    payload.depositAuthority = Keypair.generate().publicKey;

    const instruction2 = StakePoolInstruction.depositSol(payload);

    expect(instruction2.keys).toHaveLength(11);
    expect(instruction2.keys[10].pubkey).toEqual(payload.depositAuthority);
  });

  describe('addValidatorToPool', () => {
    const validatorList = mockValidatorList();
    const decodedValidatorList = ValidatorListLayout.decode(validatorList.data);
    const voteAccount = decodedValidatorList.validators[0].voteAccountAddress;

    it('should throw an error when trying to add an existing validator', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey === stakePoolAddress) {
          return stakePoolAccount;
        }
        return mockValidatorList();
      });
      await expect(addValidatorToPool(connection, stakePoolAddress, voteAccount)).rejects.toThrow(
        Error('Vote account is already in validator list'),
      );
    });

    it('should successfully add a validator', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey === stakePoolAddress) {
          return stakePoolAccount;
        }
        return <AccountInfo<any>>{
          executable: true,
          owner: new PublicKey(0),
          lamports: 0,
          data,
        };
      });
      const res = await addValidatorToPool(
        connection,
        stakePoolAddress,
        validatorListMock.validators[0].voteAccountAddress,
      );
      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(2);
      expect(res.instructions).toHaveLength(1);
      // Make sure that the validator vote account being added is the one we passed
      expect(res.instructions[0].keys[6].pubkey).toEqual(
        validatorListMock.validators[0].voteAccountAddress,
      );
    });
  });

  describe('removeValidatorFromPool', () => {
    const voteAccount = Keypair.generate().publicKey;

    it('should throw an error when trying to remove a non-existing validator', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey === stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(stakePoolMock.validatorList)) {
          return mockValidatorList();
        }
        return <AccountInfo<any>>{
          executable: true,
          owner: new PublicKey(0),
          lamports: 0,
          data,
        };
      });
      await expect(
        removeValidatorFromPool(connection, stakePoolAddress, voteAccount),
      ).rejects.toThrow(Error('Vote account is not already in validator list'));
    });

    it('should successfully remove a validator', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey === stakePoolAddress) {
          return stakePoolAccount;
        }
        if (pubKey.equals(stakePoolMock.validatorList)) {
          return mockValidatorList();
        }
        return <AccountInfo<any>>{
          executable: true,
          owner: new PublicKey(0),
          lamports: 0,
          data,
        };
      });
      const res = await removeValidatorFromPool(
        connection,
        stakePoolAddress,
        validatorListMock.validators[0].voteAccountAddress,
      );
      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(2);
      expect(res.instructions).toHaveLength(1);
      // Make sure that the validator stake account being removed is the one we passed
      const validatorStake = await findStakeProgramAddress(
        STAKE_POOL_PROGRAM_ID,
        validatorListMock.validators[0].voteAccountAddress,
        stakePoolAddress,
        0,
      );
      expect(res.instructions[0].keys[4].pubkey).toEqual(validatorStake);
    });
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

    it('should throw an error with invalid balance', async () => {
      await expect(depositSol(connection, stakePoolAddress, from, balance + 1)).rejects.toThrow(
        Error('Not enough SOL to deposit into pool. Maximum deposit amount is 0.00001 SOL.'),
      );
    });

    it('should throw an error with invalid account', async () => {
      connection.getAccountInfo = jest.fn(async () => null);
      await expect(depositSol(connection, stakePoolAddress, from, balance)).rejects.toThrow(
        Error('Invalid stake pool account'),
      );
    });

    it('should call successfully', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey) => {
        if (pubKey === stakePoolAddress) {
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

      expect((connection.getAccountInfo as jest.Mock).mock.calls.length).toBe(1);
      expect(res.instructions).toHaveLength(3);
      expect(res.signers).toHaveLength(1);
    });
  });

  describe('withdrawSol', () => {
    const tokenOwner = new PublicKey(0);
    const solReceiver = new PublicKey(1);

    it('should throw an error with invalid stake pool account', async () => {
      connection.getAccountInfo = jest.fn(async () => null);
      await expect(
        withdrawSol(connection, stakePoolAddress, tokenOwner, solReceiver, 1),
      ).rejects.toThrowError('Invalid stake pool account');
    });

    it('should throw an error with invalid token account', async () => {
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
      ).rejects.toThrow(TokenAccountNotFoundError);
    });

    it('should throw an error with invalid token account balance', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey === stakePoolAddress) {
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

    it('should call successfully', async () => {
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

    it('should throw an error with invalid token account', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        return null;
      });

      await expect(withdrawStake(connection, stakePoolAddress, tokenOwner, 1)).rejects.toThrow(
        TokenAccountNotFoundError,
      );
    });

    it('should throw an error with invalid token account balance', async () => {
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

    it('should call successfully', async () => {
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

    it('withdraw to a stake account provided', async () => {
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
    it('returns an uninitialized parsed stake account', async () => {
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

  describe('createPoolTokenMetadata', () => {
    it('should create pool token metadata', async () => {
      connection.getAccountInfo = jest.fn(async (pubKey: PublicKey) => {
        if (pubKey == stakePoolAddress) {
          return stakePoolAccount;
        }
        return null;
      });
      const name = 'test';
      const symbol = 'TEST';
      const uri = 'https://example.com';

      const payer = new PublicKey(0);
      const res = await createPoolTokenMetadata(
        connection,
        stakePoolAddress,
        payer,
        name,
        symbol,
        uri,
      );

      const type = tokenMetadataLayout(17, name.length, symbol.length, uri.length);
      const data = decodeData(type, res.instructions[0].data);
      expect(Buffer.from(data.name).toString()).toBe(name);
      expect(Buffer.from(data.symbol).toString()).toBe(symbol);
      expect(Buffer.from(data.uri).toString()).toBe(uri);
    });

    it('should update pool token metadata', async () => {
      const name = 'test';
      const symbol = 'TEST';
      const uri = 'https://example.com';
      const res = await updatePoolTokenMetadata(connection, stakePoolAddress, name, symbol, uri);
      const type = tokenMetadataLayout(18, name.length, symbol.length, uri.length);
      const data = decodeData(type, res.instructions[0].data);
      expect(Buffer.from(data.name).toString()).toBe(name);
      expect(Buffer.from(data.symbol).toString()).toBe(symbol);
      expect(Buffer.from(data.uri).toString()).toBe(uri);
    });
  });
});
