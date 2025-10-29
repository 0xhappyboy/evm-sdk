/// This module is the EVM network abstraction layer.
pub mod contract;
pub mod erc;
pub mod global;
pub mod mempool;
pub mod safe;
pub mod tool;
pub mod trade;
pub mod types;

use std::sync::Arc;

use crate::mempool::MempoolListener;
use crate::mempool::MempoolService;
use crate::trade::TradeEventListener;
use crate::trade::TradeService;
use crate::types::EvmError;
use ethers::providers::Middleware;
use ethers::{
    signers::Signer,
    types::{Address, H256, TransactionRequest, U256},
};
use evm_client::EvmClient;
use evm_client::EvmType;

/// EVM Client for interacting with various EVM chains
#[derive(Clone)]
pub struct Evm {
    pub client: EvmClient,
}

impl Evm {
    /// Create a new EVM client without wallet
    ///
    /// # Example
    /// ```
    /// use evm_utils::Evm;
    /// use evm_client::EvmType;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let evm = Evm::new(EvmType::Ethereum).await?;
    /// let chain_id = evm.get_chain_id().await?;
    /// println!("Connected to chain ID: {}", chain_id);
    /// Ok(())
    /// }
    /// ```
    pub async fn new(evm_type: EvmType) -> Result<Self, EvmError> {
        match EvmClient::from_type(evm_type).await {
            Ok(client) => Ok(Self { client: client }),
            Err(e) => Err(EvmError::RpcError(format!("Rpc Error:{:?}", e))),
        }
    }

    /// Create a new EVM client with wallet
    ///
    /// # Example
    /// ```
    /// use evm_utils::Evm;
    /// use evm_client::EvmType;
    ///
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let private_key = "your_private_key_here";
    /// let evm = Evm::with_wallet(EvmType::Ethereum, private_key).await?;
    /// let balance = evm.get_wallet_balance().await?;
    /// println!("Wallet balance: {}", balance);
    /// Ok(())
    /// }
    /// ```
    pub async fn with_wallet(evm_type: EvmType, private_key: &str) -> Result<Self, EvmError> {
        match EvmClient::from_wallet(evm_type, private_key).await {
            Ok(client) => Ok(Self { client: client }),
            Err(e) => Err(EvmError::RpcError(format!("Rpc Error:{:?}", e))),
        }
    }

    /// Get chain ID
    ///
    /// # Example
    /// ```
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let chain_id = evm.get_chain_id().await?;
    /// println!("Chain ID: {}", chain_id);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_chain_id(&self) -> Result<u64, EvmError> {
        self.client
            .provider
            .get_chainid()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get chain ID: {}", e)))
            .map(|id| id.as_u64())
    }

    /// Get block number
    ///
    /// # Example
    /// ```
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let block_number = evm.get_block_number().await?;
    /// println!("Current block number: {}", block_number);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_block_number(&self) -> Result<u64, EvmError> {
        self.client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get block number: {}", e)))
            .map(|num| num.as_u64())
    }

    /// Get balance of an address
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let address: Address = "0x742d35Cc6634C0532925a3b8D6B5d7a4C03a3a7d".parse()?;
    /// let balance = evm.get_balance(address).await?;
    /// println!("Balance: {}", balance);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_balance(&self, address: Address) -> Result<U256, EvmError> {
        self.client
            .provider
            .get_balance(address, None)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get balance: {}", e)))
    }

    /// Get transaction count (nonce) for an address
    ///
    /// # Example
    /// ```
    /// use ethers::types::Address;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let address: Address = "0x742d35Cc6634C0532925a3b8D6B5d7a4C03a3a7d".parse()?;
    /// let nonce = evm.get_transaction_count(address).await?;
    /// println!("Nonce: {}", nonce);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_transaction_count(&self, address: Address) -> Result<u64, EvmError> {
        self.client
            .provider
            .get_transaction_count(address, None)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction count: {}", e)))
            .map(|nonce| nonce.as_u64())
    }

    /// Get gas price
    ///
    /// # Example
    /// ```
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let gas_price = evm.get_gas_price().await?;
    /// println!("Gas price: {}", gas_price);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_gas_price(&self) -> Result<U256, EvmError> {
        self.client
            .provider
            .get_gas_price()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get gas price: {}", e)))
    }

    /// Send a raw transaction
    ///
    /// # Example
    /// ```
    /// use ethers::types::{TransactionRequest, Address, U256};
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let to_address: Address = "0x742d35Cc6634C0532925a3b8D6B5d7a4C03a3a7d".parse()?;
    /// let tx = TransactionRequest::new()
    ///     .to(to_address)
    ///     .value(U256::from(1000000000000000u64));
    ///     
    /// let tx_hash = evm.send_transaction(tx).await?;
    /// println!("Transaction sent: {:?}", tx_hash);
    /// Ok(())
    /// }
    /// ```
    pub async fn send_transaction(&self, mut tx: TransactionRequest) -> Result<H256, EvmError> {
        if self.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let wallet = self.client.wallet.as_ref().unwrap();
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
            .client
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| {
                EvmError::TransactionError(format!("Failed to send transaction: {}", e))
            })?;
        Ok(pending_tx.tx_hash())
    }

    /// Get transaction receipt
    ///
    /// # Example
    /// ```
    /// use ethers::types::H256;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let tx_hash: H256 = "0x...".parse()?;
    /// let receipt = evm.get_transaction_receipt(tx_hash).await?;
    /// if let Some(receipt) = receipt {
    ///     println!("Transaction status: {}", receipt.status);
    /// }
    /// Ok(())
    /// }
    /// ```
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> Result<Option<ethers::types::TransactionReceipt>, EvmError> {
        self.client
            .provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction receipt: {}", e)))
    }

    /// Get logs by filter
    ///
    /// # Example
    /// ```
    /// use ethers::types::Filter;
    /// use ethers::types::Address;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let contract_address: Address = "0x...".parse()?;
    /// let filter = Filter::new()
    ///     .address(contract_address)
    ///     .from_block(1000000)
    ///     .to_block(1000100);
    ///     
    /// let logs = evm.get_logs(filter).await?;
    /// println!("Found {} logs", logs.len());
    /// Ok(())
    /// }
    /// ```
    pub async fn get_logs(
        &self,
        filter: ethers::types::Filter,
    ) -> Result<Vec<ethers::types::Log>, EvmError> {
        self.client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))
    }

    /// Get native token balance for the wallet
    ///
    /// # Example
    /// ```
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let balance = evm.get_wallet_balance().await?;
    /// println!("Wallet balance: {}", balance);
    /// Ok(())
    /// }
    /// ```
    pub async fn get_wallet_balance(&self) -> Result<U256, EvmError> {
        if let Some(wallet) = &self.client.wallet {
            let address = wallet.address();
            self.get_balance(address).await
        } else {
            Err(EvmError::WalletError("No wallet configured".to_string()))
        }
    }

    /// Get trade service for executing trades
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let evm_arc = Arc::new(evm);
    /// let trade_service = evm_arc.clone().get_trade_service();
    /// Ok(())
    /// }
    /// ```
    pub fn get_trade_service(self: Arc<Self>) -> TradeService {
        TradeService::new(self.clone())
    }

    /// Get trade event listener for monitoring trade events
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let evm_arc = Arc::new(evm);
    /// let trade_listener = evm_arc.clone().get_trade_listener();
    /// Ok(())
    /// }
    /// ```
    pub fn get_trade_listener(self: Arc<Self>) -> TradeEventListener {
        TradeEventListener::new(self.clone())
    }

    /// Get mempool service for mempool interactions
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let evm_arc = Arc::new(evm);
    /// let mempool_service = evm_arc.clone().get_mempool_service();
    /// Ok(())
    /// }
    /// ```
    pub fn get_mempool_service(self: Arc<Self>) -> MempoolService {
        MempoolService::new(self.clone())
    }

    /// Get mempool listener for monitoring mempool activities
    ///
    /// # Example
    /// ```
    /// use std::sync::Arc;
    ///
    /// async fn example(evm: Evm) -> Result<(), Box<dyn std::error::Error>> {
    /// let evm_arc = Arc::new(evm);
    /// let mempool_listener = evm_arc.clone().get_mempool_listener();
    /// Ok(())
    /// }
    /// ```
    pub fn get_mempool_listener(self: Arc<Self>) -> MempoolListener {
        MempoolListener::new(self.clone())
    }
}
