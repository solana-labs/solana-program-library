import { Connection, PublicKey } from '@solana/web3.js';
import { amountToUiAmountForMintWithoutSimulation, uiAmountToAmountForMintWithoutSimulation } from '../../src/actions/amountToUiAmount';
import { AccountLayout, InterestBearingMintConfigStateLayout, TOKEN_2022_PROGRAM_ID } from '../../src';
import { expect } from 'chai';
import { MintLayout } from '../../src/state/mint';
import { ExtensionType } from '../../src/extensions/extensionType';
import { AccountType } from '../../src/extensions/accountType';
import sinon from 'sinon';

const ONE_YEAR_IN_SECONDS = 31556736;

// Mock connection class
class MockConnection {
    private mockAccountInfo: any;

    constructor() {
        this.mockAccountInfo = null;
    }

    getAccountInfo = async () => this.mockAccountInfo;

    setAccountInfo(info: any) {
        this.mockAccountInfo = info;
    }
}

function createMockMintData(decimals = 2, hasInterestBearingConfig = false, config: { preUpdateAverageRate?: number, currentRate?: number } = {}) {
    const mintData = Buffer.alloc(MintLayout.span);
    MintLayout.encode({
        mintAuthorityOption: 1,
        mintAuthority: new PublicKey(new Uint8Array(32).fill(1)),
        supply: BigInt(1000000),
        decimals: decimals,
        isInitialized: true,
        freezeAuthorityOption: 1,
        freezeAuthority: new PublicKey(new Uint8Array(32).fill(1))
    }, mintData);

    const baseData = Buffer.alloc(AccountLayout.span + 1);
    mintData.copy(baseData, 0);
    baseData[AccountLayout.span] = AccountType.Mint;

    if (!hasInterestBearingConfig) {
        return baseData;
    }

    const extensionData = Buffer.alloc(InterestBearingMintConfigStateLayout.span);
    InterestBearingMintConfigStateLayout.encode({
        rateAuthority: new PublicKey(new Uint8Array(32).fill(1)),
        initializationTimestamp: 0,
        preUpdateAverageRate: config.preUpdateAverageRate || 500, // default to 5%
        lastUpdateTimestamp: ONE_YEAR_IN_SECONDS, // 1 year in seconds
        currentRate: config.currentRate || 500, // default to 5%
    }, extensionData);

    const TYPE_SIZE = 2;
    const LENGTH_SIZE = 2;
    const tlvBuffer = Buffer.alloc(TYPE_SIZE + LENGTH_SIZE + extensionData.length);
    tlvBuffer.writeUInt16LE(ExtensionType.InterestBearingConfig, 0);
    tlvBuffer.writeUInt16LE(extensionData.length, TYPE_SIZE);
    extensionData.copy(tlvBuffer, TYPE_SIZE + LENGTH_SIZE);

    const fullData = Buffer.alloc(baseData.length + tlvBuffer.length);
    baseData.copy(fullData, 0);
    tlvBuffer.copy(fullData, baseData.length);
    
    return fullData;
}

describe('amountToUiAmountNow', () => {
    let connection: MockConnection;
    const mint = new PublicKey('So11111111111111111111111111111111111111112');

    beforeEach(() => {
        connection = new MockConnection() as unknown as MockConnection;
    });

    afterEach(() => {
        sinon.restore();
    });

    it('should return the correct UiAmount when interest bearing config is not present', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000);
        const testCases = [
            { decimals: 0, amount: '100', expected: '100' },
            { decimals: 2, amount: '100', expected: '1' },
            { decimals: 9, amount: '1000000000', expected: '1' },
            { decimals: 10, amount: '1', expected: '0.0000000001' },
            { decimals: 10, amount: '1000000000', expected: '0.1' }
        ];

        for (const { decimals, amount, expected } of testCases) {
            connection.setAccountInfo({
                owner: TOKEN_2022_PROGRAM_ID,
                lamports: 1000000,
                data: createMockMintData(decimals, false),
            });

            const result = await amountToUiAmountForMintWithoutSimulation(connection as unknown as Connection, mint, amount, TOKEN_2022_PROGRAM_ID);
            expect(result).to.equal(expected);
        }
    });

    // continuous compounding interest of 5% for 1 year for 1 token = 1.0512710963760240397
    it('should return the correct UiAmount for constant 5% rate', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000);
        const testCases = [
            { decimals: 0, amount: '1', expected: '1' },
            { decimals: 1, amount: '1', expected: '0.1' },
            { decimals: 10, amount: '1', expected: '0.0000000001' },
            { decimals: 10, amount: '10000000000', expected: '1.0512710963' }
        ];

        for (const { decimals, amount, expected } of testCases) {
            connection.setAccountInfo({
                owner: TOKEN_2022_PROGRAM_ID,
                lamports: 1000000,
                data: createMockMintData(decimals, true),
            });

            const result = await amountToUiAmountForMintWithoutSimulation(connection as unknown as Connection, mint, amount, TOKEN_2022_PROGRAM_ID);
            expect(result).to.equal(expected);
        }
    });

    it('should return the correct UiAmount for constant -5% rate', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(10, true, { preUpdateAverageRate: -500, currentRate: -500 }),
        });

        const result = await amountToUiAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '10000000000', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal('0.9512294245');
    });

    it('should return the correct UiAmount for netting out rates', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000*2);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(10, true, { preUpdateAverageRate: -500, currentRate: 500 }),
        });

        const result = await amountToUiAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '10000000000', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal('1');
    });

    it('should handle huge values correctly', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000*2);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(6, true),
        });

        const result = await amountToUiAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '18446744073709551615', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal('20386805083448.097198');

        // Note: This test might not be practical in JavaScript due to precision limitations
        // const hugeResult = await amountToUiAmountNow(connection as unknown as Connection, mint, BigInt('18446744073709551615'), TOKEN_2022_PROGRAM_ID);
        // expect(hugeResult).to.equal('258917064265813830000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000');
    });
});

describe('uiAmountToAmountNow', () => {

    let connection: MockConnection;
    const mint = new PublicKey('So11111111111111111111111111111111111111112');

    beforeEach(() => {
        connection = new MockConnection() as unknown as MockConnection;
    });

    afterEach(() => {
        sinon.restore();
    });
    it('should return the correct amount for constant 5% rate', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(0, true),
        });

        const result = await uiAmountToAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '1.0512710963760241', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal(1n);
    });

    it('should handle decimal places correctly', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000);
        const testCases = [
            { decimals: 1, uiAmount: '0.10512710963760241', expected: 1n },
            { decimals: 10, uiAmount: '0.00000000010512710963760242', expected: 1n },
            { decimals: 10, uiAmount: '1.0512710963760241', expected: 10000000000n },
        ];

        for (const { decimals, uiAmount, expected } of testCases) {
            connection.setAccountInfo({
                owner: TOKEN_2022_PROGRAM_ID,
                lamports: 1000000,
                data: createMockMintData(decimals, true),
            });

            const result = await uiAmountToAmountForMintWithoutSimulation(connection as unknown as Connection, mint, uiAmount, TOKEN_2022_PROGRAM_ID);
            expect(result).to.equal(expected);
        }
    });

    it('should return the correct amount for constant -5% rate', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(10, true, { preUpdateAverageRate: -500, currentRate: -500 }),
        });

        const result = await uiAmountToAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '0.951229424500714', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal(9999999999n); // calculation truncates to avoid floating point precision issues in transfers
    });

    it('should return the correct amount for netting out rates', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000*2);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(10, true, { preUpdateAverageRate: -500, currentRate: 500 }),
        });

        const result = await uiAmountToAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '1', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal(10000000000n);
    });

    it('should handle huge values correctly', async () => {
        sinon.useFakeTimers(ONE_YEAR_IN_SECONDS*1000*2);
        connection.setAccountInfo({
            owner: TOKEN_2022_PROGRAM_ID,
            lamports: 1000000,
            data: createMockMintData(0, true),
        });

        const result = await uiAmountToAmountForMintWithoutSimulation(connection as unknown as Connection, mint, '20386805083448100000', TOKEN_2022_PROGRAM_ID);
        expect(result).to.equal(18446744073709554150n);
    });
});
