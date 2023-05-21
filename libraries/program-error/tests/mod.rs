pub mod bench;
pub mod decode;
pub mod into;
pub mod print;
pub mod spl;

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::{
        decode_error::DecodeError,
        program_error::{PrintProgramError, ProgramError},
    };

    // `#[derive(IntoProgramError)]`
    #[test]
    fn test_derive_into_program_error() {
        // `Into<ProgramError>`
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::MintHasNoMintAuthority),
            Into::<ProgramError>::into(into::ExampleError::MintHasNoMintAuthority),
        );
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::IncorrectMintAuthority),
            Into::<ProgramError>::into(into::ExampleError::IncorrectMintAuthority),
        );
    }

    // `#[derive(DecodeError)]`
    #[test]
    fn test_derive_decode_error() {
        // `Into<ProgramError>`
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::MintHasNoMintAuthority),
            Into::<ProgramError>::into(decode::ExampleError::MintHasNoMintAuthority),
        );
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::IncorrectMintAuthority),
            Into::<ProgramError>::into(decode::ExampleError::IncorrectMintAuthority),
        );
        // `DecodeError<T>`
        assert_eq!(
            <bench::ExampleError as DecodeError<bench::ExampleError>>::type_of(),
            <bench::ExampleError as DecodeError<decode::ExampleError>>::type_of(),
        );
    }

    // `#[derive(PrintProgramError)]`
    #[test]
    fn test_derive_print_program_error() {
        // `Into<ProgramError>`
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::MintHasNoMintAuthority),
            Into::<ProgramError>::into(print::ExampleError::MintHasNoMintAuthority),
        );
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::IncorrectMintAuthority),
            Into::<ProgramError>::into(print::ExampleError::IncorrectMintAuthority),
        );
        // `DecodeError<T>`
        assert_eq!(
            <bench::ExampleError as DecodeError<bench::ExampleError>>::type_of(),
            <bench::ExampleError as DecodeError<print::ExampleError>>::type_of(),
        );
        // `PrintProgramError`
        // (!) Not sure how better to test this yet - thoughts?
        PrintProgramError::print::<bench::ExampleError>(
            &bench::ExampleError::MintHasNoMintAuthority,
        );
        PrintProgramError::print::<print::ExampleError>(
            &print::ExampleError::MintHasNoMintAuthority,
        );
        PrintProgramError::print::<bench::ExampleError>(
            &bench::ExampleError::IncorrectMintAuthority,
        );
        PrintProgramError::print::<print::ExampleError>(
            &print::ExampleError::IncorrectMintAuthority,
        );
    }

    // `#[spl_program_error]`
    #[test]
    fn test_spl_program_error() {
        // `Into<ProgramError>`
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::MintHasNoMintAuthority),
            Into::<ProgramError>::into(spl::ExampleError::MintHasNoMintAuthority),
        );
        assert_eq!(
            Into::<ProgramError>::into(bench::ExampleError::IncorrectMintAuthority),
            Into::<ProgramError>::into(spl::ExampleError::IncorrectMintAuthority),
        );
        // `DecodeError<T>`
        assert_eq!(
            <bench::ExampleError as DecodeError<bench::ExampleError>>::type_of(),
            <bench::ExampleError as DecodeError<spl::ExampleError>>::type_of(),
        );
        // `PrintProgramError`
        // (!) Not sure how better to test this yet - thoughts?
        PrintProgramError::print::<bench::ExampleError>(
            &bench::ExampleError::MintHasNoMintAuthority,
        );
        PrintProgramError::print::<spl::ExampleError>(&spl::ExampleError::MintHasNoMintAuthority);
        PrintProgramError::print::<bench::ExampleError>(
            &bench::ExampleError::IncorrectMintAuthority,
        );
        PrintProgramError::print::<spl::ExampleError>(&spl::ExampleError::IncorrectMintAuthority);
    }
}
