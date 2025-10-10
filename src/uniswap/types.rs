use ethers::abi::Error;
use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// uniswap error type
pub type Result<T> = std::result::Result<T, UniswapError>;

#[derive(Error, Debug)]
pub enum UniswapError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] Error),
    #[error("Ethers error: {0}")]
    EthersError(#[from] ethers::providers::ProviderError),
    #[error("Contract error: {0}")]
    ContractError(String),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Pool not found")]
    PoolNotFound,
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    #[error("Insufficient balance")]
    InsufficientBalance,
    #[error("Wallet error: {0}")]
    WalletError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: Address,
    pub symbol: String,
    pub decimals: u8,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Address,
    pub token0: TokenInfo,
    pub token1: TokenInfo,
    pub reserve0: U256,
    pub reserve1: U256,
    pub liquidity: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub tx_hash: TxHash,
    pub sender: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
    pub to: Address,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct PriceData {
    pub token0: Address,
    pub token1: Address,
    pub price: f64,
    pub liquidity: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub tx_hash: TxHash,
    pub status: bool,
    pub gas_used: U256,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub token_address: Address,
    pub to: Address,
    pub amount: U256,
    pub gas_limit: Option<U256>,
    pub gas_price: Option<U256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRequest {
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub recipient: Address,
    pub deadline: u64,
    pub slippage: f64, // slippage percentage
}

// new price query related types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPriceInfo {
    pub token_address: Address,
    pub token_symbol: String,
    pub token_name: String,
    pub decimals: u8,
    pub eth_price: Option<f64>,  // 1 token = n ETH
    pub usd_price: Option<f64>,  // 1 token = n USD
    pub usdc_price: Option<f64>, // 1 token = n USDC
    pub usdt_price: Option<f64>, // 1 token = n USDT
    pub dai_price: Option<f64>,  // 1 token = n DAI
    pub liquidity: U256,
    pub price_source: String,
    pub last_updated: u64,
}

#[derive(Debug, Clone)]
pub struct PriceFinder {
    pub weth_address: Address,
    pub usdc_address: Address,
    pub usdt_address: Address,
    pub dai_address: Address,
    pub factory_v2: Address,
    pub factory_v3: Address,
}
