pub mod contract;
pub mod dapp;
pub mod erc;
pub mod global;
pub mod safe;
pub mod tool;
pub mod trade;
pub mod types;

use crate::dapp::dex::uniswap::UniswapService;
use crate::dapp::dex::uniswap::events::UniswapEventListener;
use crate::dapp::launchpad::four::FourMemeEventListener;
use crate::dapp::launchpad::four::FourMemeService;
use crate::trade::TradeEventListener;
use crate::trade::TradeService;
use ethers::providers::{Http, Provider, Ws};
use ethers::providers::{Middleware, ProviderExt};
use ethers::{
    signers::{LocalWallet, Signer},
    types::{Address, H256, TransactionRequest, U256},
};
use std::{str::FromStr, sync::Arc};
pub use types::{EvmError, EvmType};

/// EVM Client for interacting with various EVM chains
#[derive(Clone)]
pub struct EvmClient {
    pub provider: Arc<Provider<Http>>,
    pub chain: EvmType,
    pub wallet: Option<LocalWallet>,
}

impl EvmClient {
    /// Create a new EVM client without wallet
    pub async fn new(chain: EvmType) -> Result<Self, EvmError> {
        let rpc_url = match chain {
            EvmType::Ethereum => global::rpc::ETHEREUM_RPC,
            EvmType::Arb => global::rpc::ARB_RPC,
            EvmType::Bsc => global::rpc::BSC_RPC,
            EvmType::Base => global::rpc::BASE_RPC,
            EvmType::HyperEVM => global::rpc::HYPEREVM_RPC,
            EvmType::Plasma => global::rpc::PLASMA_RPC,
        };

        if rpc_url.is_empty() {
            return Err(EvmError::ConfigError("RPC URL not configured".to_string()));
        }

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| EvmError::ConnectionError(format!("Failed to connect to RPC: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            chain,
            wallet: None,
        })
    }

    /// Create a new EVM client with wallet
    pub async fn with_wallet(chain: EvmType, private_key: &str) -> Result<Self, EvmError> {
        let rpc_url = match chain {
            EvmType::Ethereum => global::rpc::ETHEREUM_RPC,
            EvmType::Arb => global::rpc::ARB_RPC,
            EvmType::Bsc => global::rpc::BSC_RPC,
            EvmType::Base => global::rpc::BASE_RPC,
            EvmType::HyperEVM => global::rpc::HYPEREVM_RPC,
            EvmType::Plasma => global::rpc::PLASMA_RPC,
        };

        if rpc_url.is_empty() {
            return Err(EvmError::ConfigError("RPC URL not configured".to_string()));
        }

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| EvmError::ConnectionError(format!("Failed to connect to RPC: {}", e)))?;

        let wallet: LocalWallet = private_key
            .parse()
            .map_err(|e| EvmError::WalletError(format!("Failed to parse private key: {}", e)))?;

        Ok(Self {
            provider: Arc::new(provider),
            chain,
            wallet: Some(wallet),
        })
    }

    /// Get chain ID
    pub async fn get_chain_id(&self) -> Result<u64, EvmError> {
        self.provider
            .get_chainid()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get chain ID: {}", e)))
            .map(|id| id.as_u64())
    }

    /// Get block number
    pub async fn get_block_number(&self) -> Result<u64, EvmError> {
        self.provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get block number: {}", e)))
            .map(|num| num.as_u64())
    }

    /// Get balance of an address
    pub async fn get_balance(&self, address: Address) -> Result<U256, EvmError> {
        self.provider
            .get_balance(address, None)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get balance: {}", e)))
    }

    /// Get transaction count (nonce) for an address
    pub async fn get_transaction_count(&self, address: Address) -> Result<u64, EvmError> {
        self.provider
            .get_transaction_count(address, None)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction count: {}", e)))
            .map(|nonce| nonce.as_u64())
    }

    /// Get gas price
    pub async fn get_gas_price(&self) -> Result<U256, EvmError> {
        self.provider
            .get_gas_price()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get gas price: {}", e)))
    }

    /// Send a raw transaction
    pub async fn send_transaction(&self, mut tx: TransactionRequest) -> Result<H256, EvmError> {
        if self.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let wallet = self.wallet.as_ref().unwrap();
        tx.from = Some(wallet.address());
        let chain_id = self.get_chain_id().await?;
        tx.chain_id = Some(chain_id.into());
        if tx.nonce.is_none() {
            let nonce = self.get_transaction_count(wallet.address()).await?;
            tx.nonce = Some(nonce.into());
        }
        if tx.gas_price.is_none() {
            let gas_price = self.get_gas_price().await?;
            tx.gas_price = Some(gas_price);
        }
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| {
                EvmError::TransactionError(format!("Failed to send transaction: {}", e))
            })?;

        Ok(pending_tx.tx_hash())
    }

    /// Get transaction receipt
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<ethers::types::TransactionReceipt>, EvmError> {
        self.provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction receipt: {}", e)))
    }

    /// Get logs by filter
    pub async fn get_logs(
        &self,
        filter: ethers::types::Filter,
    ) -> Result<Vec<ethers::types::Log>, EvmError> {
        self.provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))
    }

    /// Get native token balance for the wallet
    pub async fn get_wallet_balance(&self) -> Result<U256, EvmError> {
        if let Some(wallet) = &self.wallet {
            let address = wallet.address();
            self.get_balance(address).await
        } else {
            Err(EvmError::WalletError("No wallet configured".to_string()))
        }
    }

    /// get trade service
    pub fn get_trade_service(self: Arc<Self>) -> TradeService {
        TradeService::new(self.clone())
    }
    /// get trade listener
    pub fn get_trade_listener(self: Arc<Self>) -> TradeEventListener {
        TradeEventListener::new(self.clone())
    }

    /// get uniswap service
    pub fn get_uniswap_service(self: Arc<Self>) -> UniswapService {
        UniswapService::new(self.clone())
    }

    /// get uniswap listener
    pub fn get_uniswap_listener(self: Arc<Self>) -> UniswapEventListener {
        UniswapEventListener::new(self.clone())
    }

    /// get four service
    pub fn get_four_service(self: Arc<Self>) -> FourMemeService {
        FourMemeService::new(self.clone()).unwrap()
    }

    /// get four listener
    pub fn get_four_listener(self: Arc<Self>) -> FourMemeEventListener {
        FourMemeEventListener::new(self.clone()).unwrap()
    }
}
