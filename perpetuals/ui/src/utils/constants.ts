import * as PerpetualsJson from "@/target/idl/perpetuals.json";
import { IDL as PERPETUALS_IDL, Perpetuals } from "@/target/types/perpetuals";
import { getProvider } from "@/utils/provider";
import { AnchorProvider, Program, Wallet } from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import { WalletContextState } from "@solana/wallet-adapter-react";
import { Keypair, PublicKey, Transaction } from "@solana/web3.js";

export const PERPETUALS_PROGRAM_ID = new PublicKey(
  PerpetualsJson["metadata"]["address"]
);

class DefaultWallet implements Wallet {
  constructor(readonly payer: Keypair) {}

  static local(): NodeWallet | never {
    throw new Error("Local wallet not supported");
  }

  async signTransaction(tx: Transaction): Promise<Transaction> {
    return tx;
  }

  async signAllTransactions(txs: Transaction[]): Promise<Transaction[]> {
    return txs;
  }

  get publicKey(): PublicKey {
    return this.payer.publicKey;
  }
}

export async function getPerpetualProgramAndProvider(
  walletContextState?: WalletContextState
): Promise<{
  perpetual_program: Program<Perpetuals>;
  provider: AnchorProvider;
}> {
  let provider;

  let perpetual_program;

  if (walletContextState) {
    let wallet: Wallet = {
      // @ts-ignore
      signTransaction: walletContextState.signTransaction,
      // @ts-ignore
      signAllTransactions: walletContextState.signAllTransactions,
      // @ts-ignore
      publicKey: walletContextState.publicKey,
    };

    provider = await getProvider(wallet);
  } else {
    provider = await getProvider(new DefaultWallet(DEFAULT_PERPS_USER));
  }

  perpetual_program = new Program(
    PERPETUALS_IDL,
    PERPETUALS_PROGRAM_ID,
    provider
  );

  return { perpetual_program, provider };
}

export const TRANSFER_AUTHORITY = findProgramAddressSync(
  [Buffer.from("transfer_authority")],
  PERPETUALS_PROGRAM_ID
)[0];

export const PERPETUALS_ADDRESS = findProgramAddressSync(
  [Buffer.from("perpetuals")],
  PERPETUALS_PROGRAM_ID
)[0];

// default user to launch show basic pool data, etc
export const DEFAULT_PERPS_USER = Keypair.fromSecretKey(
  Uint8Array.from([
    130, 82, 70, 109, 220, 141, 128, 34, 238, 5, 80, 156, 116, 150, 24, 45, 33,
    132, 119, 244, 40, 40, 201, 182, 195, 179, 90, 172, 51, 27, 110, 208, 61,
    23, 43, 217, 131, 209, 127, 113, 93, 139, 35, 156, 34, 16, 94, 236, 175,
    232, 174, 79, 209, 223, 86, 131, 148, 188, 126, 217, 19, 248, 236, 107,
  ])
);
