import * as index from './index.js';
import * as schema from './schema.js';
import BN from 'bn.js';
import assert from 'assert';

describe('schema.decode', () => {
    describe('StakePoolAccount', () => {
        it('should successfully decode StakePoolAccount account data', () => {
            const decodedData = schema.StakePoolAccount.decode(new Buffer('014f962536a1b5b3a1f1036599ca7c164d6ad0677fed896f12ecdd09ada2cd23dc4f962536a1b5b3a1f1036599ca7c164d6ad0677fed896f12ecdd09ada2cd23dcbb09705cd25f4ad3b9e2c78e3f62ba3e290f798150499edc4f851a1a45e71159ff1086831390e666d64d18cac8f0c86eb91d799eda647319634280b89548ba0371ee618745ef3ebb93164e66ac3560e934b04e49b89ad38baa3b104284abb1a574003e566772f55676cf243c647fab463afa138cb156d2080062d2957b10bf22870fdbe150fc449db6268eb9214a848a90162d9bd4bb81e722cacb41d3dcb483b706ddf6e1d765a193d9cbe146ceeb79ac1cb485ed5f5b37913a8cf5857eff00a9000000000000000000000000000000007c00000000000000e8030000000000003800000000000000', 'hex'))
            console.log(decodedData)
            assert.equal(decodedData['accountType'].enum, 'StakePool')

            // TODO
            /* 
            StakePoolAccount {
            accountType: AccountType { StakePool: AccountTypeEnum {}, enum: 'StakePool' },
            manager: PublicKey {
                value: <BN: dc23cda2ad09ddec126f89ed7f67d06a4d167cca996503f1a1b3b5a13625964f>
            },
            staker: PublicKey {
                value: <BN: dc23cda2ad09ddec126f89ed7f67d06a4d167cca996503f1a1b3b5a13625964f>
            },
            depositAuthority: PublicKey {
                value: <BN: 5911e7451a1a854fdc9e495081790f293eba623f8ec7e2b9d34a5fd25c7009bb>
            },
            withdrawBumpSeed: 255,
            validatorList: PublicKey {
                value: <BN: 7103ba4895b8804263197364da9e791db96ec8f0c8ca184dd666e69013838610>
            },
            reserveStake: PublicKey {
                value: <BN: 74a5b1ab8442103baa8bd39ab8494eb034e96035ac664e1693bb3eef458761ee>
            },
            poolMint: PublicKey {
                value: <BN: 8722bf107b95d2620008d256b18c13fa3a46ab7f643c24cf7656f57267563e00>
            },
            managerFeeAccount: PublicKey {
                value: <BN: b783b4dcd341cbca22e781bbd49b2d16908a844a21b98e26b69d44fc50e1db0f>
            },
            tokenProgramId: PublicKey {
                value: <BN: a900ff7e85f58c3a91375b5fed85b41cac79ebce46e1cbd993a165d7e1f6dd06>
            },
            totalStakeLamports: <BN: 0>,
            poolTokenSupply: <BN: 0>,
            lastUpdateEpoch: <BN: 7c>,
            fee: Fee { denominator: <BN: 3e8>, numerator: <BN: 38> }
            }
            */
        })
    })

    describe('ValidatorListAccount', () => {
        it('should successfully decode ValidatorListAccount account data', () => {
            const decodedData = schema.ValidatorListAccount.decode(new Buffer('020a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000', 'hex'))
            const expectedData = new schema.ValidatorListAccount({
                'accountType': new schema.AccountType({
                    'ValidatorList': new schema.AccountTypeEnum({})
                }),
                'maxValidators': 10,
                'validators': [],
            })

            assert.deepStrictEqual(
                decodedData,
                expectedData
            )
        })
    })
})

describe('index.ts/PrettyPrintPubkey', () => {
    it('should successfully pretty print a pubkey', () => {
        assert.equal(index.prettyPrintPubKey(new schema.PublicKey({ 'value': new BN("99572085579321386496717000324290408927851378839748241098946587626478579848783") })), "6MfzrQUzB2mozveRWU9a77zMoQzSrYa4Gq46KswjupQB");
    });
});
