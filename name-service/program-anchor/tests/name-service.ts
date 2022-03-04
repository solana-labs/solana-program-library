import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { NameService } from '../target/types/name_service';

describe('name-service', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.NameService as Program<NameService>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
