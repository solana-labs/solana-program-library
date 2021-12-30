import {
  StakePoolLayout,
  ValidatorListLayout,
  ValidatorList,
} from '../src/layouts';
import {deepStrictEqualBN} from './equal';
import {stakePoolMock, validatorListMock} from './mocks';

describe('layouts', () => {
  describe('StakePoolAccount', () => {
    it('should successfully decode StakePoolAccount data', () => {
      const encodedData = Buffer.alloc(1024);
      StakePoolLayout.encode(stakePoolMock, encodedData);
      const decodedData = StakePoolLayout.decode(encodedData);
      deepStrictEqualBN(decodedData, stakePoolMock);
    });
  });

  describe('ValidatorListAccount', () => {
    it('should successfully decode ValidatorListAccount account data', () => {
      const expectedData: ValidatorList = {
        accountType: 0,
        maxValidators: 10,
        validators: [],
      };
      const encodedData = Buffer.alloc(64);
      ValidatorListLayout.encode(expectedData, encodedData);
      const decodedData = ValidatorListLayout.decode(encodedData);
      expect(decodedData).toEqual(expectedData);
    });

    it('should successfully decode ValidatorListAccount with nonempty ValidatorInfo', () => {
      const encodedData = Buffer.alloc(1024);
      ValidatorListLayout.encode(validatorListMock, encodedData);
      const decodedData = ValidatorListLayout.decode(encodedData);
      deepStrictEqualBN(decodedData, validatorListMock);
    });
  });
});
