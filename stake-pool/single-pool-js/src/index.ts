import { createDefaultRpcTransport, createSolanaRpc, Base58EncodedAddress } from '@solana/web3.js';

// solana-test-validator --reset --bpf-program 3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY ~/work/solana/spl/target/deploy/spl_single_validator_pool.so --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s ~/work/solana/spl/stake-pool/program/tests/fixtures/mpl_token_metadata.so --account KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag ~/vote_account.json

async function main() {
    const localhostTransport = createDefaultRpcTransport({ url: 'http://127.0.0.1:8899' });
    const localhostRpc = createSolanaRpc({ transport: localhostTransport });

    const systemProgramAddress = '11111111111111111111111111111111' as Base58EncodedAddress;
    const balanceInLamports = await localhostRpc.getBalance(systemProgramAddress).send();
    console.log('Balance of System Program account in Lamports', balanceInLamports);
}

await main();
