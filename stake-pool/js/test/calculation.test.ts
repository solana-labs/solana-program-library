import { LAMPORTS_PER_SOL } from '@solana/web3.js';
import { stakePoolMock } from './mocks';
import { calcPoolTokensForDeposit } from '../src/utils/stake';
import BN from 'bn.js';

describe('calculations', () => {
  it('should successfully calculate pool tokens for a pool with a lot of stake', () => {
    const lamports = new BN(LAMPORTS_PER_SOL * 100);
    const bigStakePoolMock = stakePoolMock;
    bigStakePoolMock.totalLamports = new BN('11000000000000000'); // 11 million SOL
    bigStakePoolMock.poolTokenSupply = new BN('10000000000000000'); // 10 million tokens
    const availableForWithdrawal = calcPoolTokensForDeposit(bigStakePoolMock, lamports);
    expect(availableForWithdrawal.toNumber()).toEqual(90909090909);
  });
});
