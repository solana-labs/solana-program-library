import {
  Keypair,
  Connection,
  PublicKey,
  LAMPORTS_PER_SOL,
} from '@solana/web3.js';
import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { assert } from 'chai';

import { emptyNode, MerkleTree } from '../../src/merkle-tree';
import { createTreeOnChain, createEmptyTreeOnChain } from '../utils';
import {
  ALL_DEPTH_SIZE_PAIRS,
  ConcurrentMerkleTreeAccount,
  getConcurrentMerkleTreeAccountSize,
} from '../../src';

async function assertCMTProperties(
  onChainCMT: ConcurrentMerkleTreeAccount,
  expectedMaxDepth: number,
  expectedMaxBufferSize: number,
  expectedAuthority: PublicKey,
  expectedRoot: Buffer,
  expectedCanopyDepth?: number
) {
  assert(
    onChainCMT.getMaxDepth() === expectedMaxDepth,
    `Max depth does not match ${onChainCMT.getMaxDepth()}, expected ${expectedMaxDepth}`
  );
  assert(
    onChainCMT.getMaxBufferSize() === expectedMaxBufferSize,
    `Max buffer size does not match ${onChainCMT.getMaxBufferSize()}, expected ${expectedMaxBufferSize}`
  );
  assert(
    onChainCMT.getAuthority().equals(expectedAuthority),
    'Failed to write auth pubkey'
  );
  assert(
    onChainCMT.getCurrentRoot().equals(expectedRoot),
    'On chain root does not match root passed in instruction'
  );
  if (expectedCanopyDepth) {
    assert(
      onChainCMT.getCanopyDepth() === expectedCanopyDepth,
      'On chain canopy depth does not match expected canopy depth'
    );
  }
}

describe('ConcurrentMerkleTreeAccount tests', () => {
  // Configure the client to use the local cluster.
  let offChainTree: MerkleTree;
  let cmtKeypair: Keypair;
  let payerKeypair: Keypair;
  let payer: PublicKey;
  let connection: Connection;
  let provider: AnchorProvider;

  beforeEach(async () => {
    payerKeypair = Keypair.generate();
    payer = payerKeypair.publicKey;
    connection = new Connection('http://localhost:8899', {
      commitment: 'confirmed',
    });
    const wallet = new NodeWallet(payerKeypair);
    provider = new AnchorProvider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    });

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer, 1e10),
      'confirmed'
    );
  });

  describe('Can deserialize a CMTAccount from an on-chain CMT with a single leaf', () => {
    const MAX_SIZE = 64;
    const MAX_DEPTH = 14;

    beforeEach(async () => {
      [cmtKeypair, offChainTree] = await createTreeOnChain(
        provider,
        payerKeypair,
        1,
        { maxDepth: MAX_DEPTH, maxBufferSize: MAX_SIZE }
      );
    });

    it('Interpreted on-chain fields correctly', async () => {
      const cmt = await ConcurrentMerkleTreeAccount.fromAccountAddress(
        connection,
        cmtKeypair.publicKey,
        'confirmed'
      );

      await assertCMTProperties(
        cmt,
        MAX_DEPTH,
        MAX_SIZE,
        payer,
        offChainTree.root
      );
    });
  });

  describe('Test deserialization for available depth-size pairs', () => {
    it('Test all pairs', async () => {
      for (const depthSizePair of ALL_DEPTH_SIZE_PAIRS) {
        // Airdrop enough SOL to cover tree creation
        const size = getConcurrentMerkleTreeAccountSize(
          depthSizePair.maxDepth,
          depthSizePair.maxBufferSize
        );
        const rent = await connection.getMinimumBalanceForRentExemption(
          size,
          'confirmed'
        );
        const airdropId = await connection.requestAirdrop(
          payer,
          rent + 5000 * 2
        );
        await connection.confirmTransaction(airdropId, 'confirmed');

        // Create on chain tree
        cmtKeypair = await createEmptyTreeOnChain(
          provider,
          payerKeypair,
          depthSizePair
        );
        const cmt = await ConcurrentMerkleTreeAccount.fromAccountAddress(
          connection,
          cmtKeypair.publicKey,
          'confirmed'
        );

        // Verify it was initialized correctly
        await assertCMTProperties(
          cmt,
          depthSizePair.maxDepth,
          depthSizePair.maxBufferSize,
          payer,
          emptyNode(depthSizePair.maxDepth)
        );
      }
    });
  });

  describe('Test deserialization for canopy size for depth 30 tree', () => {
    it('Test all pairs', async () => {
      const maxDepth = 30;
      const maxBufferSize = 2048;

      for (let canopyDepth = 1; canopyDepth <= 14; canopyDepth++) {
        // Airdrop enough SOL to cover tree creation
        const size = getConcurrentMerkleTreeAccountSize(
          maxDepth,
          maxBufferSize,
          canopyDepth
        );
        const rent = await connection.getMinimumBalanceForRentExemption(
          size,
          'confirmed'
        );
        const airdropId = await connection.requestAirdrop(
          payer,
          rent + 5000 * 2
        );
        await connection.confirmTransaction(airdropId, 'confirmed');

        // Create on chain tree
        cmtKeypair = await createEmptyTreeOnChain(
          provider,
          payerKeypair,
          { maxDepth, maxBufferSize },
          canopyDepth
        );
        const cmt = await ConcurrentMerkleTreeAccount.fromAccountAddress(
          connection,
          cmtKeypair.publicKey,
          'confirmed'
        );

        // Verify it was initialized correctly
        await assertCMTProperties(
          cmt,
          maxDepth,
          maxBufferSize,
          payer,
          emptyNode(maxDepth),
          canopyDepth
        );
      }
    });
  });
});
