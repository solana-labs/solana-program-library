import { getExtraAccountMetas, resolveExtraAccountMeta } from '../../src';
import { expect } from 'chai';
import { PublicKey } from '@solana/web3.js';

describe('transferHookExtraAccounts', () => {
    const testProgramId = new PublicKey('7N4HggYEJAtCLJdnHGCtFqfxcB5rhQCsQTze3ftYstVj');
    const instructionData = Buffer.from(Array.from(Array(32).keys()));
    const plainAccount = new PublicKey('6c5q79ccBTWvZTEx3JkdHThtMa2eALba5bfvHGf8kA2c');
    const seeds = [Buffer.from('seed'), Buffer.from([4, 5, 6, 7]), plainAccount.toBuffer()];
    const pdaPublicKey = PublicKey.findProgramAddressSync(seeds, testProgramId)[0];
    const pdaPublicKeyWithProgramId = PublicKey.findProgramAddressSync(seeds, plainAccount)[0];

    const plainSeed = Buffer.concat([
        Buffer.from([1]), // u8 discriminator
        Buffer.from([4]), // u8 length
        Buffer.from('seed'), // 4 bytes seed
    ]);

    const instructionDataSeed = Buffer.concat([
        Buffer.from([2]), // u8 discriminator
        Buffer.from([4]), // u8 offset
        Buffer.from([4]), // u8 length
    ]);

    const accountKeySeed = Buffer.concat([
        Buffer.from([3]), // u8 discriminator
        Buffer.from([0]), // u8 index
    ]);

    const addressConfig = Buffer.concat([plainSeed, instructionDataSeed, accountKeySeed], 32);

    const plainExtraAccountMeta = {
        discriminator: 0,
        addressConfig: plainAccount.toBuffer(),
        isSigner: false,
        isWritable: false,
    };
    const plainExtraAccount = Buffer.concat([
        Buffer.from([0]), // u8 discriminator
        plainAccount.toBuffer(), // 32 bytes address
        Buffer.from([0]), // bool isSigner
        Buffer.from([0]), // bool isWritable
    ]);

    const pdaExtraAccountMeta = {
        discriminator: 1,
        addressConfig,
        isSigner: true,
        isWritable: false,
    };
    const pdaExtraAccount = Buffer.concat([
        Buffer.from([1]), // u8 discriminator
        addressConfig, // 32 bytes address config
        Buffer.from([1]), // bool isSigner
        Buffer.from([0]), // bool isWritable
    ]);

    const pdaExtraAccountMetaWithProgramId = {
        discriminator: 128,
        addressConfig,
        isSigner: false,
        isWritable: true,
    };
    const pdaExtraAccountWithProgramId = Buffer.concat([
        Buffer.from([128]), // u8 discriminator
        addressConfig, // 32 bytes address config
        Buffer.from([0]), // bool isSigner
        Buffer.from([1]), // bool isWritable
    ]);

    const extraAccountList = Buffer.concat([
        Buffer.from([0, 0, 0, 0, 0, 0, 0, 0]), // u64 accountDiscriminator
        Buffer.from([0, 0, 0, 0]), // u32 length
        Buffer.from([3, 0, 0, 0]), // u32 count
        plainExtraAccount,
        pdaExtraAccount,
        pdaExtraAccountWithProgramId,
    ]);

    it('getExtraAccountMetas', () => {
        const accountInfo = {
            data: extraAccountList,
            owner: PublicKey.default,
            executable: false,
            lamports: 0,
        };
        const parsedExtraAccounts = getExtraAccountMetas(accountInfo);
        expect(parsedExtraAccounts).to.not.be.null;
        if (parsedExtraAccounts == null) {
            return;
        }

        expect(parsedExtraAccounts).to.have.length(3);
        if (parsedExtraAccounts.length !== 3) {
            return;
        }

        expect(parsedExtraAccounts[0].discriminator).to.eql(0);
        expect(parsedExtraAccounts[0].addressConfig).to.eql(plainAccount.toBuffer());
        expect(parsedExtraAccounts[0].isSigner).to.be.false;
        expect(parsedExtraAccounts[0].isWritable).to.be.false;

        expect(parsedExtraAccounts[1].discriminator).to.eql(1);
        expect(parsedExtraAccounts[1].addressConfig).to.eql(addressConfig);
        expect(parsedExtraAccounts[1].isSigner).to.be.true;
        expect(parsedExtraAccounts[1].isWritable).to.be.false;

        expect(parsedExtraAccounts[2].discriminator).to.eql(128);
        expect(parsedExtraAccounts[2].addressConfig).to.eql(addressConfig);
        expect(parsedExtraAccounts[2].isSigner).to.be.false;
        expect(parsedExtraAccounts[2].isWritable).to.be.true;
    });
    it('resolveExtraAccountMeta', () => {
        const resolvedPlainAccount = resolveExtraAccountMeta(plainExtraAccountMeta, [], instructionData, testProgramId);

        expect(resolvedPlainAccount.pubkey).to.eql(plainAccount);
        expect(resolvedPlainAccount.isSigner).to.be.false;
        expect(resolvedPlainAccount.isWritable).to.be.false;

        const resolvedPdaAccount = resolveExtraAccountMeta(
            pdaExtraAccountMeta,
            [resolvedPlainAccount],
            instructionData,
            testProgramId
        );

        expect(resolvedPdaAccount.pubkey).to.eql(pdaPublicKey);
        expect(resolvedPdaAccount.isSigner).to.be.true;
        expect(resolvedPdaAccount.isWritable).to.be.false;

        const resolvedPdaAccountWithProgramId = resolveExtraAccountMeta(
            pdaExtraAccountMetaWithProgramId,
            [resolvedPlainAccount],
            instructionData,
            testProgramId
        );

        expect(resolvedPdaAccountWithProgramId.pubkey).to.eql(pdaPublicKeyWithProgramId);
        expect(resolvedPdaAccountWithProgramId.isSigner).to.be.false;
        expect(resolvedPdaAccountWithProgramId.isWritable).to.be.true;
    });
});
