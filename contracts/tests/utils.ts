import { Provider } from "@project-serum/anchor";

export async function logTx(provider: Provider, txId: string, verbose: boolean = true) {
  await provider.connection.confirmTransaction(txId, "confirmed");
  if (verbose) {
    console.log(
      (await provider.connection.getConfirmedTransaction(txId, "confirmed")).meta
        .logMessages
    );
  }
};

