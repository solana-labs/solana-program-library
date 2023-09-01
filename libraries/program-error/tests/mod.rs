pub mod bench;
pub mod decode;
pub mod into;
pub mod print;
pub mod spl;

#[cfg(test)]
mod tests {
    use {
        super::*,
        serial_test::serial,
        solana_program::{
            decode_error::DecodeError,
            program_error::{PrintProgramError, ProgramError},
        },
        std::sync::{Arc, RwLock},
    };

    // Used to capture output for `PrintProgramError` for testing
    lazy_static::lazy_static! {
        static ref EXPECTED_DATA: Arc<RwLock<Vec<u8>>> = Arc::new(RwLock::new(Vec::new()));
    }
    fn set_expected_data(expected_data: Vec<u8>) {
        *EXPECTED_DATA.write().unwrap() = expected_data;
    }
    pub struct SyscallStubs {}
    impl solana_sdk::program_stubs::SyscallStubs for SyscallStubs {
        fn sol_log(&self, message: &str) {
            assert_eq!(
                message,
                String::from_utf8_lossy(&EXPECTED_DATA.read().unwrap())
            );
        }
    }

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
    #[serial]
    fn test_derive_print_program_error() {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            solana_sdk::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
        });
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
        set_expected_data("Mint has no mint authority".as_bytes().to_vec());
        PrintProgramError::print::<print::ExampleError>(
            &print::ExampleError::MintHasNoMintAuthority,
        );
        set_expected_data(
            "Incorrect mint authority has signed the instruction"
                .as_bytes()
                .to_vec(),
        );
        PrintProgramError::print::<print::ExampleError>(
            &print::ExampleError::IncorrectMintAuthority,
        );
    }

    // `#[spl_program_error]`
    #[test]
    #[serial]
    fn test_spl_program_error() {
        use std::sync::Once;
        static ONCE: Once = Once::new();

        ONCE.call_once(|| {
            solana_sdk::program_stubs::set_syscall_stubs(Box::new(SyscallStubs {}));
        });
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
        set_expected_data("Mint has no mint authority".as_bytes().to_vec());
        PrintProgramError::print::<spl::ExampleError>(&spl::ExampleError::MintHasNoMintAuthority);
        set_expected_data(
            "Incorrect mint authority has signed the instruction"
                .as_bytes()
                .to_vec(),
        );
        PrintProgramError::print::<spl::ExampleError>(&spl::ExampleError::IncorrectMintAuthority);
    }
}
