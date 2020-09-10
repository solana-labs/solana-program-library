#![allow(missing_docs)]

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    GeneralError,
    FromHexError,
    ElGamalConversionError,
    // Web3Error
    Web3ErrorIo,
    Web3ErrorRpc,
    Web3ErrorUnreachable,
    Web3ErrorDecoder,
    Web3ErrorInvalidResponse,
    Web3ErrorTransport,
    Web3ErrorInternal,
    // Web3ErrorContract
    Web3ErrorContractInvalidOutputType,
    Web3ErrorContractAbi,
    Web3ErrorContractApi,
    // EthAbiError
    EthAbiError,
    EthAbiErrorInvalidName,
    EthAbiErrorInvalidData,
    EthAbiErrorSerdeJson,
    EthAbiErrorParseInt,
    EthAbiErrorUtf8,
    EthAbiErrorHex,
    EthAbiErrorOther,
}
