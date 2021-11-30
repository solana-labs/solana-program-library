import {
  STAKE_POOL_LAYOUT,
  VALIDATOR_LIST_LAYOUT,
  ValidatorList,
} from '../src/layouts';
import { deepStrictEqualBN } from "./utils";
import { stakePoolMock, validatorListMock } from "./mocks";

describe('layouts', () => {

  describe('StakePoolAccount', () => {
    it('should successfully decode StakePoolAccount data', () => {
      const encodedData = Buffer.alloc(1024);
      STAKE_POOL_LAYOUT.encode(stakePoolMock, encodedData);
      const decodedData = STAKE_POOL_LAYOUT.decode(encodedData);
      deepStrictEqualBN(decodedData, stakePoolMock);
    });
  });

  describe('ValidatorListAccount', () => {
    it('should successfully decode ValidatorListAccount account data', () => {
      const expectedData = <ValidatorList>{
        accountType: 0,
        maxValidators: 10,
        validators: [],
      };
      const encodedData = Buffer.alloc(64);
      VALIDATOR_LIST_LAYOUT.encode(expectedData, encodedData);
      const decodedData = VALIDATOR_LIST_LAYOUT.decode(encodedData);
      expect(decodedData).toEqual(expectedData);
    });

    it('should successfully decode ValidatorListAccount with nonempty ValidatorInfo', () => {
      const encodedData = Buffer.alloc(1024);
      VALIDATOR_LIST_LAYOUT.encode(validatorListMock, encodedData);
      const decodedData = VALIDATOR_LIST_LAYOUT.decode(encodedData);
      deepStrictEqualBN(decodedData, validatorListMock);
    });
  });
});
