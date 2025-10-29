use std::fmt;

#[derive(Debug)]
pub enum EvmError {
    ConfigError(String),
    ConnectionError(String),
    RpcError(String),
    WalletError(String),
    TransactionError(String),
    ContractError(String),
    InvalidInput(String),
    IOError(String),
    AaveError(String),
    ListenerError(String),
    ProviderError(String),
    CalculationError(String),
    MempoolError(String),
    Error(String),
}

impl fmt::Display for EvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvmError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            EvmError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            EvmError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            EvmError::WalletError(msg) => write!(f, "Wallet error: {}", msg),
            EvmError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            EvmError::ContractError(msg) => write!(f, "Contract error: {}", msg),
            EvmError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            EvmError::IOError(msg) => write!(f, "IO Error: {}", msg),
            EvmError::AaveError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::ListenerError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::ProviderError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::CalculationError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::MempoolError(msg) => write!(f, "Aave Error: {}", msg),
            EvmError::Error(msg) => write!(f, "Aave Error: {}", msg),
        }
    }
}

impl std::error::Error for EvmError {}
