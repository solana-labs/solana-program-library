import { TransferFee, TransferFeeConfig, calculateFee, calculateEpochFee, ONE_IN_BASIS_POINTS } from '../../src';
import { expect } from 'chai';
import { PublicKey } from '@solana/web3.js';

describe('calculateFee', () => {
    it('should return 0 fee when transferFeeBasisPoints is 0', () => {
        const transferFee = {
            epoch: 1n,
            maximumFee: 100n,
            transferFeeBasisPoints: 0,
        };
        const preFeeAmount = 100n;
        const fee = calculateFee(transferFee, preFeeAmount);
        expect(fee).to.eql(0n);
    });

    it('should return 0 fee when preFeeAmount is 0', () => {
        const transferFee = {
            epoch: 1n,
            maximumFee: 100n,
            transferFeeBasisPoints: 100,
        };
        const preFeeAmount = 0n;
        const fee = calculateFee(transferFee, preFeeAmount);
        expect(fee).to.eql(0n);
    });

    it('should calculate the fee correctly when preFeeAmount is non-zero', () => {
        const transferFee = {
            epoch: 1n,
            maximumFee: 100n,
            transferFeeBasisPoints: 50,
        };
        const preFeeAmount = 500n;
        const fee = calculateFee(transferFee, preFeeAmount);
        expect(fee).to.eql(3n);
    });

    it('fee should be equal to maximum fee', () => {
        const transferFee = {
            epoch: 1n,
            maximumFee: 5000n,
            transferFeeBasisPoints: 50,
        };
        const preFeeAmount = transferFee.maximumFee;
        const fee = calculateFee(transferFee, preFeeAmount * ONE_IN_BASIS_POINTS);
        expect(fee).to.eql(transferFee.maximumFee);
    });
    it('fee should be equal to maximum fee when added 1 to preFeeAmount', () => {
        const transferFee = {
            epoch: 1n,
            maximumFee: 5000n,
            transferFeeBasisPoints: 50,
        };
        const preFeeAmount = transferFee.maximumFee;
        const fee = calculateFee(transferFee, preFeeAmount * ONE_IN_BASIS_POINTS + 1n);
        expect(fee).to.eql(transferFee.maximumFee);
    });
});

describe('calculateEpochFee', () => {
    const transferFeeConfig = {
        transferFeeConfigAuthority: PublicKey.default,
        withdrawWithheldAuthority: PublicKey.default,
        withheldAmount: 500n,
        olderTransferFee: {
            epoch: 1n,
            maximumFee: 100n,
            transferFeeBasisPoints: 50,
        },
        newerTransferFee: {
            epoch: 2n,
            maximumFee: 200n,
            transferFeeBasisPoints: 75,
        },
    };

    it('should return olderTransferFee when epoch is less than newerTransferFee.epoch', () => {
        const preFeeAmount = 200n;
        const epoch = 1n;
        const fee = calculateEpochFee(transferFeeConfig, epoch, preFeeAmount);
        expect(fee).to.eql(1n);
    });

    it('should return newerTransferFee when epoch is greater than or equal to newerTransferFee.epoch', () => {
        const preFeeAmount = 200n;
        const epoch = 2n;
        const fee = calculateEpochFee(transferFeeConfig, epoch, preFeeAmount);
        expect(fee).to.eql(2n);
    });

    it('should cap the fee to the maximumFee when calculated fee exceeds maximumFee', () => {
        const preFeeAmount = 500n;
        const epoch = 2n;
        const fee = calculateEpochFee(transferFeeConfig, epoch, preFeeAmount);
        expect(fee).to.eql(4n);
    });
});
