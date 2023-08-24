import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
import { PublicKey } from '@solana/web3.js';
import {
    AccountState,
    createCreateNativeMintInstruction,
    createEnableRequiredMemoTransfersInstruction,
    createInitializeNonTransferableMintInstruction,
    createInitializeTransferFeeConfigInstruction,
    createInitializeMintCloseAuthorityInstruction,
    createInitializeDefaultAccountStateInstruction,
    NATIVE_MINT,
    NATIVE_MINT_2022,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    TokenUnsupportedInstructionError,
    createInitializePermanentDelegateInstruction,
    createEnableCpiGuardInstruction,
    createInitializeTransferHookInstruction,
} from '../../src';
chai.use(chaiAsPromised);

describe('unsupported extensions in spl-token', () => {
    const mint = new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z');
    const account = new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z');
    const authority = new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z');
    const payer = new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z');
    const transferHookProgramId = new PublicKey('7o36UsWR1JQLpZ9PE2gn9L4SQ69CNNiWAXd4Jt7rqz9Z');
    it('initializeMintCloseAuthority', () => {
        expect(function () {
            createInitializeMintCloseAuthorityInstruction(mint, null, TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createInitializeMintCloseAuthorityInstruction(mint, null, TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('defaultAccountState', () => {
        expect(function () {
            createInitializeDefaultAccountStateInstruction(mint, AccountState.Frozen, TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createInitializeDefaultAccountStateInstruction(mint, AccountState.Frozen, TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('memoTransfer', () => {
        expect(function () {
            createEnableRequiredMemoTransfersInstruction(account, authority, [], TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createEnableRequiredMemoTransfersInstruction(account, authority, [], TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('transferFee', () => {
        expect(function () {
            createInitializeTransferFeeConfigInstruction(mint, null, null, 0, BigInt(0), TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createInitializeTransferFeeConfigInstruction(mint, null, null, 0, BigInt(0), TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('nativeMint', () => {
        expect(function () {
            createCreateNativeMintInstruction(payer, NATIVE_MINT, TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createCreateNativeMintInstruction(payer, NATIVE_MINT_2022, TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('transferHook', () => {
        expect(function () {
            createInitializeTransferHookInstruction(mint, authority, transferHookProgramId, TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createInitializeTransferHookInstruction(mint, authority, transferHookProgramId, TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('nonTransferableMint', () => {
        expect(function () {
            createInitializeNonTransferableMintInstruction(mint, TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createInitializeNonTransferableMintInstruction(mint, TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('initializePermanentDelegate', () => {
        expect(function () {
            createInitializePermanentDelegateInstruction(mint, null, TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createInitializePermanentDelegateInstruction(mint, null, TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
    it('cpiGuard', () => {
        expect(function () {
            createEnableCpiGuardInstruction(account, authority, [], TOKEN_PROGRAM_ID);
        }).to.throw(TokenUnsupportedInstructionError);
        expect(function () {
            createEnableCpiGuardInstruction(account, authority, [], TOKEN_2022_PROGRAM_ID);
        }).to.not.throw(TokenUnsupportedInstructionError);
    });
});
