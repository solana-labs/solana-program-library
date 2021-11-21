import BN from 'bn.js';
import {assert} from 'chai';
import {PublicKey} from '@solana/web3.js';
import {
  AccountType,
  STAKE_POOL_LAYOUT,
  StakePool,
  VALIDATOR_LIST_LAYOUT,
  ValidatorList
} from '../src/layouts';
import {decodeData, encodeData} from "../src/copied-from-solana-web3/instruction";
import {deepStrictEqualBN} from "./utils";

describe('layouts', () => {
  describe('StakePoolAccount', () => {
    it('should successfully decode StakePoolAccount data', () => {
      const expectedData = <StakePool>{
        accountType: AccountType.StakePool,
        manager: new PublicKey(
          new BN(
            'dc23cda2ad09ddec126f89ed7f67d06a4d167cca996503f1a1b3b5a13625964f',
            'hex',
          ),
        ),
        staker: new PublicKey(
          new BN(
            'dc23cda2ad09ddec126f89ed7f67d06a4d167cca996503f1a1b3b5a13625964f',
            'hex',
          ),
        ),
        stakeDepositAuthority: new PublicKey(
          new BN(
            new Buffer(
              '5911e7451a1a854fdc9e495081790f293eba623f8ec7e2b9d34a5fd25c7009bb',
              'hex',
            ),
          ),
        ),
        stakeWithdrawBumpSeed: 255,
        validatorList: new PublicKey(
          new BN(
            '7103ba4895b8804263197364da9e791db96ec8f0c8ca184dd666e69013838610',
            'hex',
          ),
        ),
        reserveStake: new PublicKey(
          new BN(
            '74a5b1ab8442103baa8bd39ab8494eb034e96035ac664e1693bb3eef458761ee',
            'hex',
          ),
        ),
        poolMint: new PublicKey(
          new BN(
            '8722bf107b95d2620008d256b18c13fa3a46ab7f643c24cf7656f57267563e00',
            'hex',
          ),
        ),
        managerFeeAccount: new PublicKey(
          new BN(
            new Buffer(
              'b783b4dcd341cbca22e781bbd49b2d16908a844a21b98e26b69d44fc50e1db0f',
              'hex',
            ),
          ),
        ),
        tokenProgramId: new PublicKey(
          new BN(
            'a900ff7e85f58c3a91375b5fed85b41cac79ebce46e1cbd993a165d7e1f6dd06',
            'hex',
          ),
        ),
        totalLamports: new BN('0', 'hex'),
        poolTokenSupply: new BN('0', 'hex'),
        lastUpdateEpoch: new BN('7c', 'hex'),
        epochFee: {
          denominator: new BN('3e8', 'hex'),
          numerator: new BN('38', 'hex'),
        },
      };

      const encodedData = encodeData(STAKE_POOL_LAYOUT, expectedData);
      const decodedData = decodeData(STAKE_POOL_LAYOUT, encodedData);

      deepStrictEqualBN(decodedData, expectedData);
    });
  });

  describe('ValidatorListAccount', () => {
    it('should successfully decode ValidatorListAccount account data', () => {
      const expectedData = <ValidatorList>{
        accountType: 0,
        maxValidators: 10,
        validators: [],
      };

      const encodedData = encodeData(VALIDATOR_LIST_LAYOUT, expectedData);
      const decodedData = decodeData(VALIDATOR_LIST_LAYOUT, encodedData);

      assert.deepStrictEqual(decodedData, expectedData);
    });

    it('should successfully decode ValidatorListAccount with nonempty ValidatorInfo', () => {
      // TODO also test for decoding ValidatorListAccount with actual ValidatorInfo
      // Do this once we have a stake pool with validators deployed on testnet

      const expectedData = {
        accountType: 0,
        maxValidators: 100,
        validators: [
          {
            status: 0,
            voteAccountAddress: new PublicKey(
              new BN(
                'a9946a889af14fd3c9b33d5df309489d9699271a6b09ff3190fcb41cf21a2f8c',
                'hex',
              ),
            ),
            activeStakeLamports: new BN('0', 'hex'),
            lastUpdateEpoch: new BN('c3', 'hex'),
          },
          {
            status: 1,
            voteAccountAddress: new PublicKey(
              new BN(
                '3796d40645ee07e3c64117e3f73430471d4c40465f696ebc9b034c1fc06a9f7d',
                'hex',
              ),
            ),
            stakeLamports: new BN('0', 'hex'),
            lastUpdateEpoch: new BN('c3', 'hex'),
          },
          {
            status: 0,
            voteAccountAddress: new PublicKey(
              new BN(
                'e4e37d6f2e80c0bb0f3da8a06304e57be5cda6efa2825b86780aa320d9784cf8',
                'hex',
              ),
            ),
            stakeLamports: new BN('0', 'hex'),
            lastUpdateEpoch: new BN('c3', 'hex'),
          },
        ],
      } as ValidatorList;

      const encodedData = encodeData(VALIDATOR_LIST_LAYOUT, expectedData);
      const decodedData = decodeData(VALIDATOR_LIST_LAYOUT, encodedData);

      // assert.deepStrictEqual(decodedData, expectedData);
      deepStrictEqualBN(decodedData, expectedData);
    });
  });
});
