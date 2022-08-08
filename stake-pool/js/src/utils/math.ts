import BN from 'bn.js';
import { LAMPORTS_PER_SOL } from '@solana/web3.js';

const SOL_DECIMALS = Math.log10(LAMPORTS_PER_SOL);

export function solToLamports(amount: number): number {
  if (isNaN(amount)) {
    return Number(0);
  }
  return new BN(amount.toFixed(SOL_DECIMALS).replace('.', '')).toNumber();
}

export function lamportsToSol(lamports: number | BN): number {
  if (typeof lamports === 'number') {
    return Math.abs(lamports) / LAMPORTS_PER_SOL;
  }
  const absLamports = lamports.abs();
  const signMultiplier = lamports.isNeg() ? -1 : 1;
  const lamportsString = absLamports.toString(10).padStart(10, '0');
  const splitIndex = lamportsString.length - SOL_DECIMALS;
  const solString = lamportsString.slice(0, splitIndex) + '.' + lamportsString.slice(splitIndex);
  return signMultiplier * parseFloat(solString);
}
