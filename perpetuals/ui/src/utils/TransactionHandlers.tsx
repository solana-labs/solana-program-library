import { MethodsBuilder } from "@project-serum/anchor/dist/cjs/program/namespace/methods";
import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";
import { toast } from "react-toastify";

export const TRX_URL = (txid: string) =>
  `https://explorer.solana.com/tx/${txid}?cluster=devnet`;

export const ACCOUNT_URL = (address: string) =>
  `https://explorer.solana.com/address/${address}?cluster=devnet`;

export async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export const getUnixTs = () => {
  return new Date().getTime() / 1000;
};

export async function automaticSendTransaction(
  methodBuilder: MethodsBuilder,
  connection: Connection
) {
  let successMessage = "Transaction success!";
  let failMessage = "Transaction Failed";

  await sendAnchorTransactionAndNotify({
    methodBuilder,
    connection,
    successMessage,
    failMessage,
  });
}

export async function sendAnchorTransactionAndNotify({
  methodBuilder,
  connection,
  successMessage,
  failMessage,
}: {
  methodBuilder: MethodsBuilder;
  connection: Connection;
  successMessage: string;
  failMessage: string;
}) {
  let txid = "temptx";

  await new Promise(function (resolve, reject) {
    toast.promise(
      (async () => {
        try {
          console.log("pre rpc");
          let txid = await methodBuilder.rpc();
          console.log("post rpc");
          await connection.confirmTransaction(txid, "confirmed");
          console.log(
            `XEN:: - TRX :: https://explorer.solana.com/tx/${txid}?cluster=devnet`
          );
          resolve(true);
        } catch (error) {
          reject(error);
          console.log("error", error);
          // console.log("message", error.programErrorStack);
          console.log("after trying to print");
          throw error;
        }
      })(),
      {
        pending: {
          render() {
            return (
              <div className="processing-transaction">
                <div>
                  <h2>Processing transaction {`  `}</h2>
                  <a
                    target="_blank"
                    rel="noopener noreferrer"
                    href={`${TRX_URL(txid)}`}
                    className="text-blue-500"
                  >
                    {" "}
                    View on explorer
                  </a>
                </div>
              </div>
            );
          },
        },
        success: {
          render({ data }) {
            return (
              <div className="processing-transaction">
                <div>
                  <span className="icon green">
                    <span
                      className="iconify"
                      data-icon="teenyicons:tick-circle-solid"
                    ></span>
                  </span>
                </div>
                <div>
                  <h2>{successMessage}</h2>
                  <a
                    target="_blank"
                    rel="noopener noreferrer"
                    href={`${TRX_URL(txid)}`}
                    className="text-blue-500"
                  >
                    {" "}
                    View on explorer
                  </a>
                </div>
              </div>
            );
          },
          icon: false,
        },
        error: {
          render({ data }) {
            // When the promise reject, data will contains the error
            return (
              <div className="processing-transaction">
                <div>
                  <span className="icon red">
                    <span
                      className="iconify"
                      data-icon="akar-icons:circle-x-fill"
                    ></span>
                  </span>
                </div>
                <div>
                  <h2>
                    {JSON.stringify(data?.message ?? {}).includes("timed")
                      ? data.message
                      : failMessage}
                  </h2>
                  <a
                    target="_blank"
                    rel="noopener noreferrer"
                    href={`${TRX_URL(txid)}`}
                    className="text-blue-500"
                  >
                    {" "}
                    View on explorer
                  </a>
                </div>
              </div>
            );
          },
          icon: false,
        },
      }
    );
  });

  return txid;
}

export async function manualSendTransaction(
  transaction: Transaction,
  publicKey: PublicKey,
  connection: Connection,
  signTransaction: any,
  otherSigner?: Keypair,
  successMessage?: string,
  failMessage?: string
) {
  // try {
  transaction.feePayer = publicKey;
  transaction.recentBlockhash = (
    await connection.getRecentBlockhash("finalized")
  ).blockhash;

  await sendSignedTransactionAndNotify({
    connection,
    transaction,
    successMessage: successMessage ?? "",
    failMessage: failMessage ?? "",
    signTransaction,
    enableSigning: true,
  });
}

export async function awaitTransactionSignatureConfirmation({
  connection,
  txid,
  timeout,
  confirmations = 1,
}: {
  connection: Connection;
  txid: string;
  timeout: number;
  confirmations: Number;
}) {
  let done = false;
  const result = await new Promise((resolve, reject) => {
    // eslint-disable-next-line
    (async () => {
      setTimeout(() => {
        if (done) {
          return;
        }
        done = true;
        // console.log('Timed out for txid', txid);
        reject({ timeout: true });
      }, timeout);

      while (!done) {
        // eslint-disable-next-line
        (async () => {
          try {
            const signatureStatuses = await connection.getSignatureStatuses([
              txid,
            ]);
            const result = signatureStatuses && signatureStatuses.value[0];
            if (!done) {
              if (!result) {
                // console.log('REST null result for', txid, result);
              } else if (result.err) {
                // console.log('REST error for', txid, result);
                done = true;
                reject(result.err);
              }
              // @ts-ignore
              else if (
                !(
                  result.confirmations >= confirmations ||
                  result.confirmationStatus === "finalized"
                )
              ) {
              } else {
                console.log("confirmed", txid, result);
                done = true;
                resolve(result);
              }
            }
          } catch (e) {
            if (!done) {
              // console.log('REST connection error: txid', txid, e);
            }
          }
        })();
        await sleep(1000);
      }
    })();
  });
  done = true;
  return result;
}

interface xx {
  connection: Connection;
  transaction: Transaction;
  wallet: any;
  enableSigning: boolean;
}

export const sendTransaction = async ({
  connection,
  transaction,
  wallet,
  enableSigning = true,
}: xx) => {
  if (!transaction.recentBlockhash) {
    let hash = await connection.getLatestBlockhash();
    transaction.recentBlockhash = hash.blockhash;
  }
  if (enableSigning) {
    transaction = await wallet.signTransaction(transaction);
  }
  const rawTransaction = transaction.serialize();

  const txid = await connection.sendRawTransaction(rawTransaction, {
    skipPreflight: true,
  });
  return { txid, rawTransaction };
};

export const sendRawTransaction = async ({
  connection,
  txid,
  rawTransaction,
}: {
  connection: Connection;
  txid: string;
  rawTransaction: any;
}) => {
  const timeout = 60000,
    confirmLevel = "processed";
  const startTime = getUnixTs();
  let done = false;
  (async () => {
    await sleep(1000);
    while (!done && getUnixTs() - startTime < timeout) {
      connection.sendRawTransaction(rawTransaction, {
        skipPreflight: true,
      });
      await sleep(1000);
    }
  })();
  try {
    await awaitTransactionSignatureConfirmation({
      connection,
      txid,
      timeout,
      confirmations: 10,
    });
    // notify(successMessage);
  } catch (err) {
    if (err.timeout) {
      throw new Error("Transaction timed out");
      // notify(txid+" "+ " - Timed out", "error");
    }
    throw new Error("Transaction Failed : " + err.message);
    // notify(txid+" "+failMessage, "error");
  } finally {
    done = true;
  }
};

export const sendSignedTransactionAndNotify = async ({
  connection,
  transaction,
  successMessage,
  failMessage,
  signTransaction,
  enableSigning = true,
}: {
  connection: Connection;
  transaction: Transaction;
  successMessage: string;
  failMessage: string;
  signTransaction: Function;
  enableSigning: boolean;
}) => {
  if (!transaction) {
    throw Error("no transaction");
  }
  const { txid, rawTransaction } = await sendTransaction({
    connection,
    transaction,
    wallet: { signTransaction },
    enableSigning,
  });
  console.log(
    `XEN:: - TRX :: https://explorer.solana.com/tx/${txid}?cluster=devnet`
  );

  await new Promise(function (resolve, reject) {
    toast.promise(
      (async () => {
        try {
          await sendRawTransaction({ connection, txid, rawTransaction });
          resolve(true);
        } catch (error) {
          reject(error);
          throw error;
        }
      })(),
      {
        pending: {
          render() {
            return (
              <div className="processing-transaction">
                <div>
                  <h2>Processing transaction {`  `}</h2>
                  <a
                    target="_blank"
                    rel="noopener noreferrer"
                    href={`${TRX_URL(txid)}`}
                  >
                    {" "}
                    View on explorer
                  </a>
                </div>
              </div>
            );
          },
        },
        success: {
          render({ data }) {
            return (
              <div className="processing-transaction">
                <div>
                  <span className="icon green">
                    <span
                      className="iconify"
                      data-icon="teenyicons:tick-circle-solid"
                    ></span>
                  </span>
                </div>
                <div>
                  <h2>{successMessage}</h2>
                  <a
                    target="_blank"
                    rel="noopener noreferrer"
                    href={`${TRX_URL(txid)}`}
                  >
                    {" "}
                    View on explorer
                  </a>
                </div>
              </div>
            );
          },
          icon: false,
        },
        error: {
          render({ data }) {
            // When the promise reject, data will contains the error
            return (
              <div className="processing-transaction">
                <div>
                  <span className="icon red">
                    <span
                      className="iconify"
                      data-icon="akar-icons:circle-x-fill"
                    ></span>
                  </span>
                </div>
                <div>
                  <h2>
                    {JSON.stringify(data?.message ?? {}).includes("timed")
                      ? data.message
                      : failMessage}
                  </h2>
                  <a
                    target="_blank"
                    rel="noopener noreferrer"
                    href={`${TRX_URL(txid)}`}
                  >
                    {" "}
                    View on explorer
                  </a>
                </div>
              </div>
            );
          },
          icon: false,
        },
      },
      {
        position: "bottom-left",
        autoClose: 4000,
        className: "processing-transaction",
      }
    );
  });
  return txid;
};
