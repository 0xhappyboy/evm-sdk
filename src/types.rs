use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvmType {
    Ethereum,
    Arb,
    Bsc,
    Base,
    HyperEVM,
    Plasma,
}

impl EvmType {
    pub fn name(&self) -> &'static str {
        match self {
            EvmType::Ethereum => "Ethereum",
            EvmType::Arb => "Arbitrum",
            EvmType::Bsc => "Binance Smart Chain",
            EvmType::Base => "Base",
            EvmType::HyperEVM => "HyperEVM",
            EvmType::Plasma => "Plasma",
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            EvmType::Ethereum => 1,
            EvmType::Arb => 42161,
            EvmType::Bsc => 56,
            EvmType::Base => 8453,
            EvmType::HyperEVM => 777,
            EvmType::Plasma => 94,
        }
    }
}

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
        }
    }
}

impl std::error::Error for EvmError {}

/// Price data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenPrice {
    pub symbol: String,
    pub price: f64,
    pub change_24h: f64,
    pub market_cap: f64,
    pub volume_24h: f64,
}
