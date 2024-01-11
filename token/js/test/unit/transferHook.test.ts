import type { ExtraAccountMeta, ExtraAccountMetaList } from '../../src';
import {
    ACCOUNT_SIZE,
    ACCOUNT_TYPE_SIZE,
    ExtensionType,
    ExtraAccountMetaAccountDataLayout,
    ExtraAccountMetaLayout,
    LENGTH_SIZE,
    MintLayout,
    TOKEN_2022_PROGRAM_ID,
    TRANSFER_HOOK_SIZE,
    TYPE_SIZE,
    TransferHookLayout,
    addExtraAccountMetasForExecute,
    createTransferCheckedWithTransferHookInstruction,
    getExtraAccountMetaAddress,
    getExtraAccountMetas,
    resolveExtraAccountMeta,
} from '../../src';
import { expect } from 'chai';
import type { Connection } from '@solana/web3.js';
import { Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
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
            Buffer.from([0, 0, 0, 0]), // u32 length
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

    describe('adding extra metas to instructions', () => {
        let connection: Connection;

        let transferHookProgramId: PublicKey;

        let sourcePubkey: PublicKey;
        let mintPubkey: PublicKey;
        let destinationPubkey: PublicKey;
        let authorityPubkey: PublicKey;
        let validateStatePubkey: PublicKey;

        const amount = 100n;
        const amountInLeBytes = Buffer.alloc(8);
        amountInLeBytes.writeBigUInt64LE(amount);
        const decimals = 0;

        // Arbitrary program ID included to test external PDAs
        let arbitraryProgramId: PublicKey;

        beforeEach(async () => {
            connection = await getConnection();

            transferHookProgramId = Keypair.generate().publicKey;

            sourcePubkey = Keypair.generate().publicKey;
            mintPubkey = Keypair.generate().publicKey;
            destinationPubkey = Keypair.generate().publicKey;
            authorityPubkey = Keypair.generate().publicKey;
            validateStatePubkey = getExtraAccountMetaAddress(mintPubkey, transferHookProgramId);

            arbitraryProgramId = Keypair.generate().publicKey;
        });

        function createMockFetchAccountDataFn(extraAccounts: ExtraAccountMeta[]) {
            return async function mockFetchAccountDataFn(
                publicKey: PublicKey,
                _commitmentOrConfig?: Parameters<Connection['getAccountInfo']>[1]
            ): ReturnType<Connection['getAccountInfo']> {
                // Mocked mint state
                if (publicKey.equals(mintPubkey)) {
                    const data = Buffer.alloc(
                        ACCOUNT_SIZE + ACCOUNT_TYPE_SIZE + TYPE_SIZE + LENGTH_SIZE + TRANSFER_HOOK_SIZE
                    );
                    MintLayout.encode(
                        {
                            mintAuthorityOption: 0,
                            mintAuthority: PublicKey.default,
                            supply: 10000n,
                            decimals,
                            isInitialized: true,
                            freezeAuthorityOption: 0,
                            freezeAuthority: PublicKey.default,
                        },
                        data,
                        0
                    );
                    data.writeUint8(1, ACCOUNT_SIZE); // Account type (1): Mint = 1
                    data.writeUint16LE(ExtensionType.TransferHook, ACCOUNT_SIZE + ACCOUNT_TYPE_SIZE);
                    data.writeUint16LE(TRANSFER_HOOK_SIZE, ACCOUNT_SIZE + ACCOUNT_TYPE_SIZE + TYPE_SIZE);
                    TransferHookLayout.encode(
                        {
                            authority: Keypair.generate().publicKey,
                            programId: transferHookProgramId,
                        },
                        data,
                        ACCOUNT_SIZE + ACCOUNT_TYPE_SIZE + TYPE_SIZE + LENGTH_SIZE
                    );
                    return {
                        data,
                        owner: TOKEN_2022_PROGRAM_ID,
                        executable: false,
                        lamports: 0,
                    };
                }

                // Mocked validate state
                if (publicKey.equals(validateStatePubkey)) {
                    const extraAccountsList: ExtraAccountMetaList = {
                        count: extraAccounts.length,
                        extraAccounts,
                    };
                    const instructionDiscriminator = Buffer.from([
                        105, 37, 101, 197, 75, 251, 102, 26,
                    ]).readBigUInt64LE();
                    const data = Buffer.alloc(8 + 4 + 4 + ExtraAccountMetaLayout.span * extraAccounts.length);
                    ExtraAccountMetaAccountDataLayout.encode(
                        {
                            instructionDiscriminator,
                            length: 4 + ExtraAccountMetaLayout.span * extraAccounts.length,
                            extraAccountsList,
                        },
                        data
                    );
                    return {
                        data,
                        owner: transferHookProgramId,
                        executable: false,
                        lamports: 0,
                    };
                }

                return {
                    data: Buffer.from([]),
                    owner: PublicKey.default,
                    executable: false,
                    lamports: 0,
                };
            };
        }

        const addressConfig = (data: Uint8Array) => {
            const addressConfig = Buffer.alloc(32);
            addressConfig.set(data, 0);
            return addressConfig;
        };

        const fixedAddress = (address: PublicKey, isSigner: boolean, isWritable: boolean) => ({
            discriminator: 0,
            addressConfig: address.toBuffer(),
            isSigner,
            isWritable,
        });

        const pda = (seeds: number[], isSigner: boolean, isWritable: boolean) => ({
            discriminator: 1,
            addressConfig: addressConfig(new Uint8Array(seeds)),
            isSigner,
            isWritable,
        });

        const externalPda = (programKeyIndex: number, seeds: number[], isSigner: boolean, isWritable: boolean) => ({
            discriminator: (1 << 7) + programKeyIndex,
            addressConfig: addressConfig(new Uint8Array(seeds)),
            isSigner,
            isWritable,
        });

        it('can add extra account metas for execute', async () => {
            const extraMeta1Pubkey = Keypair.generate().publicKey;
            const extraMeta2Pubkey = Keypair.generate().publicKey;
            const extraMeta3Pubkey = Keypair.generate().publicKey;

            // prettier-ignore
            connection.getAccountInfo = createMockFetchAccountDataFn([
                fixedAddress(extraMeta1Pubkey, false, false),
                fixedAddress(extraMeta2Pubkey, false, false),
                fixedAddress(extraMeta3Pubkey, false, false),
                pda([
                    3, 0, // First seed: Account key at index 0 (2)
                    3, 4, // Second seed: Account key at index 4 (2)
                ], false, false),
                pda([
                    3, 5, // First seed: Account key at index 5 (2)
                    3, 6, // Second seed: Account key at index 6 (2)
                ], false, false),
                pda([
                    1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
                    2, 8, 8, // Second seed: Instruction data 8..16 (3)
                ], false, false),
            ]);

            const extraMeta4Pubkey = PublicKey.findProgramAddressSync(
                [sourcePubkey.toBuffer(), validateStatePubkey.toBuffer()],
                transferHookProgramId
            )[0];
            const extraMeta5Pubkey = PublicKey.findProgramAddressSync(
                [extraMeta1Pubkey.toBuffer(), extraMeta2Pubkey.toBuffer()],
                transferHookProgramId
            )[0];
            const extraMeta6Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from('prefix'),
                    amountInLeBytes, // Instruction data 8..16
                ],
                transferHookProgramId
            )[0];

            // Fail missing key
            const rawInstructionMissingKey = new TransactionInstruction({
                keys: [
                    // source missing
                    { pubkey: mintPubkey, isSigner: false, isWritable: false },
                    { pubkey: destinationPubkey, isSigner: false, isWritable: true },
                    { pubkey: authorityPubkey, isSigner: true, isWritable: false },
                ],
                programId: transferHookProgramId,
            });
            await expect(
                addExtraAccountMetasForExecute(
                    connection,
                    rawInstructionMissingKey,
                    transferHookProgramId,
                    sourcePubkey,
                    mintPubkey,
                    destinationPubkey,
                    authorityPubkey,
                    amount
                )
            ).to.be.rejectedWith('Missing required account in instruction');

            const instruction = new TransactionInstruction({
                keys: [
                    { pubkey: sourcePubkey, isSigner: false, isWritable: true },
                    { pubkey: mintPubkey, isSigner: false, isWritable: false },
                    { pubkey: destinationPubkey, isSigner: false, isWritable: true },
                    { pubkey: authorityPubkey, isSigner: true, isWritable: false },
                ],
                programId: transferHookProgramId,
            });

            await addExtraAccountMetasForExecute(
                connection,
                instruction,
                transferHookProgramId,
                sourcePubkey,
                mintPubkey,
                destinationPubkey,
                authorityPubkey,
                amount
            );

            const checkMetas = [
                { pubkey: sourcePubkey, isSigner: false, isWritable: true },
                { pubkey: mintPubkey, isSigner: false, isWritable: false },
                { pubkey: destinationPubkey, isSigner: false, isWritable: true },
                { pubkey: authorityPubkey, isSigner: true, isWritable: false },
                { pubkey: extraMeta1Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta2Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta3Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta4Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta5Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta6Pubkey, isSigner: false, isWritable: false },
                { pubkey: transferHookProgramId, isSigner: false, isWritable: false },
                { pubkey: validateStatePubkey, isSigner: false, isWritable: false },
            ];

            expect(instruction.keys).to.eql(checkMetas);
        });

        it('can create a transfer instruction with extra metas', async () => {
            // prettier-ignore
            connection.getAccountInfo = createMockFetchAccountDataFn([
                pda([
                    3, 0, // First seed: Account key at index 0 (2)
                    3, 1, // Second seed: Account key at index 1 (2)
                ], false, false),
                pda([
                    3, 4, // First seed: Account key at index 4 (2)
                ], false, false),
                pda([
                    1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
                    2, 8, 8, // Second seed: Instruction data 8..16 (3)
                ], false, false),
                fixedAddress(arbitraryProgramId, false, false),
                externalPda(8, [
                    1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
                    2, 8, 8, // Second seed: Instruction data 8..16 (3)
                    3, 6, // Third seed: Account key at index 6 (2)
                ], false, false),
                externalPda(8, [
                    1, 14, 97, 110, 111, 116, 104, 101, 114, 95, 112, 114, 101, 102, 105,
                    120, // First seed: Literal "another_prefix" (16)
                    2, 8, 8, // Second seed: Instruction data 8..16 (3)
                    3, 6, // Third seed: Account key at index 6 (2)
                    3, 9, // Fourth seed: Account key at index 9 (2)
                ], false, false),
            ]);

            const extraMeta1Pubkey = PublicKey.findProgramAddressSync(
                [
                    sourcePubkey.toBuffer(), // Account key at index 0
                    mintPubkey.toBuffer(), // Account key at index 1
                ],
                transferHookProgramId
            )[0];
            const extraMeta2Pubkey = PublicKey.findProgramAddressSync(
                [
                    validateStatePubkey.toBuffer(), // Account key at index 4
                ],
                transferHookProgramId
            )[0];
            const extraMeta3Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from('prefix'),
                    amountInLeBytes, // Instruction data 8..16
                ],
                transferHookProgramId
            )[0];
            const extraMeta4Pubkey = arbitraryProgramId;
            const extraMeta5Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from('prefix'),
                    amountInLeBytes, // Instruction data 8..16
                    extraMeta2Pubkey.toBuffer(),
                ],
                extraMeta4Pubkey // PDA off of the arbitrary program ID
            )[0];
            const extraMeta6Pubkey = PublicKey.findProgramAddressSync(
                [
                    Buffer.from('another_prefix'),
                    amountInLeBytes, // Instruction data 8..16
                    extraMeta2Pubkey.toBuffer(),
                    extraMeta5Pubkey.toBuffer(),
                ],
                extraMeta4Pubkey // PDA off of the arbitrary program ID
            )[0];

            const instruction = await createTransferCheckedWithTransferHookInstruction(
                connection,
                sourcePubkey,
                mintPubkey,
                destinationPubkey,
                authorityPubkey,
                amount,
                decimals,
                [],
                undefined,
                TOKEN_2022_PROGRAM_ID
            );

            const checkMetas = [
                { pubkey: sourcePubkey, isSigner: false, isWritable: true },
                { pubkey: mintPubkey, isSigner: false, isWritable: false },
                { pubkey: destinationPubkey, isSigner: false, isWritable: true },
                { pubkey: authorityPubkey, isSigner: true, isWritable: false },
                { pubkey: extraMeta1Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta2Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta3Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta4Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta5Pubkey, isSigner: false, isWritable: false },
                { pubkey: extraMeta6Pubkey, isSigner: false, isWritable: false },
                { pubkey: transferHookProgramId, isSigner: false, isWritable: false },
                { pubkey: validateStatePubkey, isSigner: false, isWritable: false },
            ];

            expect(instruction.keys).to.eql(checkMetas);
        });
    });
});
