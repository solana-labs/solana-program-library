import { Connection } from "@solana/web3.js";

export class TokenLending {
  connection: Connection;

  constructor(connection: Connection) {
    this.connection = connection;
  }
}
