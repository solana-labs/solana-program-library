#[cfg(test)]
mod test {
    use crate::merkle_roll::{self, MerkleRoll};
    use rand::prelude::SliceRandom;
    use rand::{self, Rng};
    use rand::{rngs::ThreadRng, thread_rng};

    fn setup() -> MerkleRoll<14, 64> {
        // on-chain merkle change-record
        let merkle = MerkleRoll::<14, 64>::new();
        merkle
    }

    #[test]
    fn test_initialize() {
        println!("Hello world!");
        let mut merkle_roll = setup();
        merkle_roll.initialize().unwrap();
    }
}
