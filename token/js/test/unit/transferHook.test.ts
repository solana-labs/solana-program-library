import {
    createTransferCheckedInstructionWithExtraMetas,
    getExtraAccountMetaAddress,
    getExtraAccountMetaList,
    resolveExtraAccountMeta,
    TOKEN_2022_PROGRAM_ID,
} from '../../src';
import { expect } from 'chai';
import type { Connection } from '@solana/web3.js';
import { Keypair, PublicKey } from '@solana/web3.js';
import { getConnection } from '../common';

describe('transferHook', () => {
    describe('validation data', () => {
        let connection: Connection;
        const testProgramId = new PublicKey('7N4HggYEJAtCLJdnHGCtFqfxcB5rhQCsQTze3ftYstVj');
        const instructionData = Buffer.from(Array.from(Array(32).keys()));
        const plainAccount = new PublicKey('6c5q79ccBTWvZTEx3JkdHThtMa2eALba5bfvHGf8kA2c');
        const seeds = [
            Buffer.from('seed'),
            Buffer.from([4, 5, 6, 7]),
            plainAccount.toBuffer(),
            Buffer.from([2, 2, 2, 2]),
        ];
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

        const accountDataSeed = Buffer.concat([
            Buffer.from([4]), // u8 discriminator
            Buffer.from([0]), // u8 account index
            Buffer.from([2]), // u8 account data offset
            Buffer.from([4]), // u8 account data length
        ]);

        const addressConfig = Buffer.concat([plainSeed, instructionDataSeed, accountKeySeed, accountDataSeed], 32);

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
            Buffer.from([109, 0, 0, 0]), // u32 length (35 * 3 + 4)
            Buffer.from([3, 0, 0, 0]), // u32 count
            plainExtraAccount,
            pdaExtraAccount,
            pdaExtraAccountWithProgramId,
        ]);

        before(async () => {
            connection = await getConnection();
            connection.getAccountInfo = async (
                _publicKey: PublicKey,
                _commitmentOrConfig?: Parameters<(typeof connection)['getAccountInfo']>[1]
            ): ReturnType<(typeof connection)['getAccountInfo']> => ({
                data: Buffer.from([0, 0, 2, 2, 2, 2]),
                owner: PublicKey.default,
                executable: false,
                lamports: 0,
            });
        });

        it('can parse extra metas', () => {
            const accountInfo = {
                data: extraAccountList,
                owner: PublicKey.default,
                executable: false,
                lamports: 0,
            };
            const parsedExtraAccounts = getExtraAccountMetaList(accountInfo);
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

        it('can resolve extra metas', async () => {
            const resolvedPlainAccount = await resolveExtraAccountMeta(
                connection,
                plainExtraAccountMeta,
                [],
                instructionData,
                testProgramId
            );

            expect(resolvedPlainAccount.pubkey).to.eql(plainAccount);
            expect(resolvedPlainAccount.isSigner).to.be.false;
            expect(resolvedPlainAccount.isWritable).to.be.false;

            const resolvedPdaAccount = await resolveExtraAccountMeta(
                connection,
                pdaExtraAccountMeta,
                [resolvedPlainAccount],
                instructionData,
                testProgramId
            );

            expect(resolvedPdaAccount.pubkey).to.eql(pdaPublicKey);
            expect(resolvedPdaAccount.isSigner).to.be.true;
            expect(resolvedPdaAccount.isWritable).to.be.false;

            const resolvedPdaAccountWithProgramId = await resolveExtraAccountMeta(
                connection,
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

    // prettier-ignore
    describe('adding to transfer instructions', () => {
        const TRANSFER_HOOK_PROGRAM_ID = new PublicKey(Buffer.from([
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ]))

        const MINT_PUBKEY = new PublicKey(Buffer.from([
            2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
        ]))

        const MOCK_MINT_STATE = [
            0, 0, 0, 0, // COption (4): None = 0
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, // Mint authority (32)
            0, 0, 0, 0, 0, 0, 0, 0, // Supply (8)
            0, // Decimals (1)
            1, // Is initialized (1)
            0, 0, 0, 0, // COption (4): None = 0
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, // Freeze authority (32)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Padding (83)
            1, // Account type (1): Mint = 1
            14, 0, // Extension type (2): Transfer hook = 14
            64, 0, // Extension length (2): 64
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, // Authority (32)
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, // Transfer hook program ID (32)
        ];

        const MOCK_EXTRA_METAS_STATE = [
            105, 37, 101, 197, 75, 251, 102, 26, // Discriminator for `ExecuteInstruction` (8)
            214, 0, 0, 0, // Length of pod slice (4): 214
            6, 0, 0, 0, // Count of account metas (4): 6
            1, // First account meta discriminator (1): PDA = 1
            3, 0, // First seed: Account key at index 0 (2)
            3, 1, // Second seed: Account key at index 1 (2)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, // No more seeds (28)
            0, // First account meta is signer (1): false = 0
            0, // First account meta is writable (1): false = 0
            1, // Second account meta discriminator (1): PDA = 1
            3, 4, // First seed: Account key at index 4 (2)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, // No more seeds (30)
            0, // Second account meta is signer (1): false = 0
            0, // Second account meta is writable (1): false = 0
            1, // Third account meta discriminator (1): PDA = 1
            1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
            2, 8, 8, // Second seed: Instruction data 8..16 (3)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // No more seeds (21)
            0, // Third account meta is signer (1): false = 0
            0, // Third account meta is writable (1): false = 0
            0, // Fourth account meta discriminator (1): Pubkey = 0
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            7, 7,   // Pubkey (32)
            0,   // Fourth account meta is signer (1): false = 0
            0,   // Fourth account meta is writable (1): false = 0
            136, // Fifth account meta discriminator (1): External PDA = 128 + index 8 = 136
            1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
            2, 8, 8, // Second seed: Instruction data 8..16 (3)
            3, 6, // Third seed: Account key at index 6 (2)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,   // No more seeds (19)
            0,   // Fifth account meta is signer (1): false = 0
            0,   // Fifth account meta is writable (1): false = 0
            136, // Sixth account meta discriminator (1): External PDA = 128 + index 8 = 136
            1, 14, 97, 110, 111, 116, 104, 101, 114, 95, 112, 114, 101, 102, 105,
            120, // First seed: Literal "another_prefix" (16)
            2, 8, 8, // Second seed: Instruction data 8..16 (3)
            3, 6, // Third seed: Account key at index 6 (2)
            3, 9, // Fourth seed: Account key at index 9 (2)
            0, 0, 0, 0, 0, 0, 0, 0, 0, // No more seeds (9)
            0, // Sixth account meta is signer (1): false = 0
            0, // Sixth account meta is writable (1): false = 0
        ];

        async function mockFetchAccountDataFn(
            publicKey: PublicKey,
            _commitmentOrConfig?: Parameters<Connection['getAccountInfo']>[1]
        ): ReturnType<Connection['getAccountInfo']> {
            if (publicKey.equals(MINT_PUBKEY)) {
                return {
                    data: Buffer.from(MOCK_MINT_STATE),
                    owner: TOKEN_2022_PROGRAM_ID,
                    executable: false,
                    lamports: 0,
                };
            };
            if (publicKey.equals(getExtraAccountMetaAddress(MINT_PUBKEY, TRANSFER_HOOK_PROGRAM_ID))) {
                return {
                    data: Buffer.from(MOCK_EXTRA_METAS_STATE),
                    owner: TRANSFER_HOOK_PROGRAM_ID,
                    executable: false,
                    lamports: 0,
                };
            };
            return {
                data: Buffer.from([]),
                owner: PublicKey.default,
                executable: false,
                lamports: 0,
            };
        }

        it('can add extra accounts to a transfer instruction', async () => {
            const amount = 2n;
            const sourcePubkey = Keypair.generate().publicKey;
            const mintPubkey = MINT_PUBKEY;
            const destinationPubkey = Keypair.generate().publicKey;
            const authorityPubkey = Keypair.generate().publicKey;
            const validateStatePubkey = getExtraAccountMetaAddress(MINT_PUBKEY, TRANSFER_HOOK_PROGRAM_ID);

            const amountInLeBytes = Buffer.alloc(8);
            amountInLeBytes.writeBigUInt64LE(amount);

            const extraMeta1Pubkey = PublicKey.findProgramAddressSync(
                [
                    sourcePubkey.toBuffer(), // Account key at index 0
                    mintPubkey.toBuffer(),   // Account key at index 1
                ],
                TRANSFER_HOOK_PROGRAM_ID,
            )[0];
            const extraMeta2Pubkey = PublicKey.findProgramAddressSync(
                [
                    validateStatePubkey.toBuffer(), // Account key at index 4
                ],
                TRANSFER_HOOK_PROGRAM_ID,
            )[0];
            const extraMeta3Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("prefix"),
                    amountInLeBytes, // Instruction data 8..16
                ],
                TRANSFER_HOOK_PROGRAM_ID,
            )[0];
            const extraMeta4Pubkey = new PublicKey(Buffer.from([
                7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
            ])); // Some arbitrary program ID
            const extraMeta5Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("prefix"),
                    amountInLeBytes, // Instruction data 8..16
                    extraMeta2Pubkey.toBuffer(),
                ],
                extraMeta4Pubkey, // PDA off of the arbitrary program ID
            )[0];
            const extraMeta6Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("another_prefix"),
                    amountInLeBytes, // Instruction data 8..16
                    extraMeta2Pubkey.toBuffer(),
                    extraMeta5Pubkey.toBuffer(),
                ],
                extraMeta4Pubkey, // PDA off of the arbitrary program ID
            )[0];


            const connection = await getConnection();
            connection.getAccountInfo = mockFetchAccountDataFn;

            const transferInstruction = await createTransferCheckedInstructionWithExtraMetas(
                connection,
                sourcePubkey,
                mintPubkey,
                destinationPubkey,
                authorityPubkey,
                amount,
                9,
                [],
                undefined,
                TOKEN_2022_PROGRAM_ID
            );

            // The validation account should not be at index 4
            expect(transferInstruction.keys[4].pubkey).to.not.eql(validateStatePubkey);

            // Verify all PDAs are correct
            expect(transferInstruction.keys[4].pubkey).to.eql(extraMeta1Pubkey);
            expect(transferInstruction.keys[5].pubkey).to.eql(extraMeta2Pubkey);
            expect(transferInstruction.keys[6].pubkey).to.eql(extraMeta3Pubkey);
            expect(transferInstruction.keys[7].pubkey).to.eql(extraMeta4Pubkey);
            expect(transferInstruction.keys[8].pubkey).to.eql(extraMeta5Pubkey);
            expect(transferInstruction.keys[9].pubkey).to.eql(extraMeta6Pubkey);
        });
    });
});
