pub mod bench;
pub mod spl;

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::program_error::ProgramError;
    #[test]
    fn test_spl_program_error() {
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::MintHasNoMintAuthority),
            Into::<ProgramError>::into(spl::ExampleError::MintHasNoMintAuthority),
        );
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::IncorrectMintAuthority),
            Into::<ProgramError>::into(spl::ExampleError::IncorrectMintAuthority),
        );
    }
}
