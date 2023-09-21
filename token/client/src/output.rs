#![cfg(feature = "display")]

use {crate::client::RpcClientResponse, solana_cli_output::display::writeln_transaction, std::fmt};

impl fmt::Display for RpcClientResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RpcClientResponse::Signature(signature) => writeln!(f, "Signature: {}", signature),
            RpcClientResponse::Transaction(transaction) => {
                writeln!(f, "Transaction:")?;
                writeln_transaction(f, &transaction.clone().into(), None, "  ", None, None)
            }
            RpcClientResponse::Simulation(result) => {
                writeln!(f, "Simulation:")?;
                // maybe implement another formatter on simulation result?
                writeln!(f, "{result:?}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        solana_sdk::{
            hash::Hash,
            pubkey::Pubkey,
            signature::{Signature, Signer, SIGNATURE_BYTES},
            signer::keypair::Keypair,
            system_instruction,
            transaction::Transaction,
        },
    };

    #[test]
    fn display_signature() {
        let signature_bytes = [202u8; SIGNATURE_BYTES];
        let signature = RpcClientResponse::Signature(Signature::from(signature_bytes));
        println!("{}", signature);
    }

    #[test]
    fn display_transaction() {
        let payer = Keypair::new();
        let transaction = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &payer.pubkey(),
                &Pubkey::new_unique(),
                10,
            )],
            Some(&payer.pubkey()),
            &[&payer],
            Hash::default(),
        );
        let transaction = RpcClientResponse::Transaction(transaction);
        println!("{}", transaction);
    }
}
