use crate::{Evm, EvmError, erc::erc20::ERC20Service, global::is_quote, types::Direction};
use ethers::{
    providers::Middleware,
    types::{
        Address, BlockNumber, Filter, H256, Log, Transaction, TransactionReceipt, U256,
        ValueOrArray,
    },
};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::time::interval;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionQuery {
    pub address: String,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWithReceipt {
    pub transaction: Transaction,
    pub receipt: Option<TransactionReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedTransactions {
    pub transactions: Vec<TransactionWithReceipt>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}

/// Service for handling transaction-related operations
pub struct Trade {
    evm: Arc<Evm>,
    erc20_service: ERC20Service,
}

impl Trade {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self {
            evm: evm.clone(),
            erc20_service: ERC20Service::new(evm.clone()),
        }
    }

    /// Retrieve transaction details based on transaction hash
    ///
    /// # Params
    /// - `tx_hash`: tx hash
    ///
    /// # Returns
    /// - `Ok(TransactionInfo)`: Transaction details
    /// - `Err(EvmError)`: Error message when retrieving failure information
    ///
    /// # Example
    /// ```
    /// let tx_info = trade_service.get_transactions_by_tx("0x1234...").await?;
    /// ```
    pub async fn get_transactions_by_tx(&self, tx_hash: &str) -> Result<TransactionInfo, EvmError> {
        let hash: H256 = tx_hash
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid transaction hash format: {}", e)))?;
        let transaction = self
            .evm
            .client
            .provider
            .get_transaction(hash)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| EvmError::RpcError("Transaction not found".to_string()))?;
        let receipt = self
            .evm
            .client
            .provider
            .get_transaction_receipt(hash)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction receipt: {}", e)))?;
        let mut timestamp: Option<U256> = None;
        let mut logs = Vec::new();
        if let Some(block_number) = transaction.block_number {
            if let Ok(Some(block)) = self
                .evm
                .client
                .provider
                .get_block(ethers::types::BlockId::Number(
                    ethers::types::BlockNumber::Number(block_number),
                ))
                .await
            {
                timestamp = Some(block.timestamp);
            }
            if let Some(ref receipt_data) = receipt {
                logs = receipt_data.logs.clone();
            }
        }
        let hash_str = format!("{:?}", hash);
        let hash_short = if hash_str.len() > 10 {
            format!("{}...{}", &hash_str[0..6], &hash_str[hash_str.len() - 4..])
        } else {
            hash_str.clone()
        };
        let is_contract_creation = transaction.to.is_none();
        let contract_address = receipt.as_ref().and_then(|r| r.contract_address);
        let status = receipt.as_ref().and_then(|r| r.status).map(|s| s.as_u64());
        let is_success = status.map(|s| s == 1).unwrap_or(false);
        let gas_used = receipt.as_ref().and_then(|r| r.gas_used);
        let total_gas_cost =
            if let (Some(gas_used_val), Some(gas_price_val)) = (gas_used, transaction.gas_price) {
                gas_used_val.checked_mul(gas_price_val)
            } else {
                None
            };
        let max_priority_fee_per_gas = transaction.max_priority_fee_per_gas;
        let max_fee_per_gas = transaction.max_fee_per_gas;
        let transaction_type = transaction.transaction_type.map(|t| t.as_u64());
        let chain_id = transaction.chain_id;
        let mut token_decimals_cache = std::collections::HashMap::new();
        for log in &logs {
            let token_address = log.address;
            match self.erc20_service.get_decimals(token_address).await {
                Ok(decimals) => {
                    token_decimals_cache.insert(token_address, decimals);
                }
                Err(e) => {
                    token_decimals_cache.insert(token_address, 18);
                }
            }
        }
        Ok(TransactionInfo {
            hash,
            from: transaction.from,
            to: transaction.to,
            value: transaction.value,
            gas_price: transaction.gas_price,
            gas: transaction.gas,
            gas_used,
            input: transaction.input.to_vec(),
            block_number: transaction.block_number.map(|n| n.as_u64()),
            transaction_index: transaction.transaction_index.map(|i| i.as_u64()),
            timestamp,
            status,
            is_contract_creation,
            hash_short,
            receipt,
            raw_transaction: transaction,
            contract_address,
            transaction_type,
            max_priority_fee_per_gas,
            max_fee_per_gas,
            chain_id,
            logs,
            is_success,
            total_gas_cost,
            token_decimals_cache,
        })
    }

    /// Get transactions for a specific address with filtering and pagination
    ///
    /// # Example
    /// ```
    /// let query = TransactionQuery {
    ///     address: "0x...".to_string(),
    ///     from_block: Some(1000000),
    ///     to_block: Some(1001000),
    ///     page: Some(1),
    ///     page_size: Some(50),
    /// };
    /// let result = trade_service.get_transactions_by_address(query).await?;
    /// ```
    pub async fn get_transactions_by_address(
        &self,
        query: TransactionQuery,
    ) -> Result<PaginatedTransactions, EvmError> {
        let address: Address = query
            .address
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address format: {}", e)))?;

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(50);
        let mut filter = Filter::new().address(ValueOrArray::Value(address));
        if let Some(from_block) = query.from_block {
            filter = filter.from_block(BlockNumber::Number(from_block.into()));
        }
        if let Some(to_block) = query.to_block {
            filter = filter.to_block(BlockNumber::Number(to_block.into()));
        }
        let logs = self
            .evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))?;

        let total = logs.len() as u64;
        let total_pages = (total as f64 / page_size as f64).ceil() as u64;
        let start_index = ((page - 1) * page_size) as usize;
        let end_index = std::cmp::min(start_index + page_size as usize, logs.len());
        let mut transactions = Vec::new();
        for log in logs
            .into_iter()
            .skip(start_index)
            .take(end_index - start_index)
        {
            if let Some(tx_hash) = log.transaction_hash {
                if let Ok(Some(tx)) = self.evm.client.provider.get_transaction(tx_hash).await {
                    let receipt = self
                        .evm
                        .client
                        .provider
                        .get_transaction_receipt(tx_hash)
                        .await
                        .map_err(|e| EvmError::RpcError(format!("Failed to get receipt: {}", e)))?;
                    transactions.push(TransactionWithReceipt {
                        transaction: tx,
                        receipt,
                    });
                }
            }
        }
        Ok(PaginatedTransactions {
            transactions,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// Get transactions involving two specific addresses
    ///
    /// # Example
    /// ```
    /// let address_a = "0x...".to_string();
    /// let address_b = "0x...".to_string();
    /// let transactions = trade_service.get_transactions_involving_addresses(
    ///     address_a,
    ///     address_b,
    ///     Some(1000000),
    ///     Some(1001000)
    /// ).await?;
    /// ```
    pub async fn get_transactions_involving_addresses(
        &self,
        address_a: String,
        address_b: String,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<TransactionWithReceipt>, EvmError> {
        let address_a_parsed: Address = address_a
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address_a format: {}", e)))?;
        let address_b_parsed: Address = address_b
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address_b format: {}", e)))?;

        let mut filter = Filter::new().address(ValueOrArray::Array(vec![
            address_a_parsed,
            address_b_parsed,
        ]));
        if let Some(from_block) = from_block {
            filter = filter.from_block(BlockNumber::Number(from_block.into()));
        }
        if let Some(to_block) = to_block {
            filter = filter.to_block(BlockNumber::Number(to_block.into()));
        }
        let logs = self
            .evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))?;
        let mut transactions = Vec::new();
        let mut processed_hashes = std::collections::HashSet::new();
        for log in logs {
            if let Some(tx_hash) = log.transaction_hash {
                if processed_hashes.contains(&tx_hash) {
                    continue;
                }
                processed_hashes.insert(tx_hash);
                if let Ok(Some(tx)) = self.evm.client.provider.get_transaction(tx_hash).await {
                    let involves_both = tx.from == address_a_parsed
                        || tx.from == address_b_parsed
                        || tx
                            .to
                            .map(|to| to == address_a_parsed || to == address_b_parsed)
                            .unwrap_or(false);
                    if involves_both {
                        let receipt = self
                            .evm
                            .client
                            .provider
                            .get_transaction_receipt(tx_hash)
                            .await
                            .map_err(|e| {
                                EvmError::RpcError(format!("Failed to get receipt: {}", e))
                            })?;
                        transactions.push(TransactionWithReceipt {
                            transaction: tx,
                            receipt,
                        });
                    }
                }
            }
        }
        Ok(transactions)
    }

    /// Get transactions where sender address sent to receiver address
    ///
    /// # Example
    /// ```
    /// let receiver = "0x...".to_string();
    /// let sender = "0x...".to_string();
    /// let transactions = trade_service.get_transactions_from_b_to_a(
    ///     receiver,
    ///     sender,
    ///     Some(1000000),
    ///     Some(1001000)
    /// ).await?;
    /// ```
    pub async fn get_transactions_from_b_to_a(
        &self,
        receiver: String, // receiver address
        sender: String,   // sender address
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<TransactionWithReceipt>, EvmError> {
        let receiver_parsed: Address = receiver
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid receiver address format: {}", e)))?;
        let sender_parsed: Address = sender
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid sender address format: {}", e)))?;

        let mut filter = Filter::new().address(ValueOrArray::Value(receiver_parsed));
        if let Some(from_block) = from_block {
            filter = filter.from_block(BlockNumber::Number(from_block.into()));
        }
        if let Some(to_block) = to_block {
            filter = filter.to_block(BlockNumber::Number(to_block.into()));
        }
        let logs = self
            .evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))?;
        let mut transactions = Vec::new();
        for log in logs {
            if let Some(tx_hash) = log.transaction_hash {
                if let Ok(Some(tx)) = self.evm.client.provider.get_transaction(tx_hash).await {
                    if tx.from == sender_parsed
                        && tx.to.map(|to| to == receiver_parsed).unwrap_or(false)
                    {
                        let receipt = self
                            .evm
                            .client
                            .provider
                            .get_transaction_receipt(tx_hash)
                            .await
                            .map_err(|e| {
                                EvmError::RpcError(format!("Failed to get receipt: {}", e))
                            })?;
                        transactions.push(TransactionWithReceipt {
                            transaction: tx,
                            receipt,
                        });
                    }
                }
            }
        }
        Ok(transactions)
    }

    /// Get transaction statistics for an address
    ///
    /// # Example
    /// ```
    /// let stats = trade_service.get_transaction_stats(
    ///     "0x...".to_string(),
    ///     Some(1000000),
    ///     Some(1001000)
    /// ).await?;
    /// ```
    pub async fn get_transaction_stats(
        &self,
        address: String,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<TransactionStats, EvmError> {
        let address_parsed: Address = address
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address format: {}", e)))?;

        let query = TransactionQuery {
            address: address.clone(),
            from_block,
            to_block,
            page: None,
            page_size: None,
        };
        let transactions = self.get_transactions_by_address(query).await?;
        let mut total_received = ethers::types::U256::zero();
        let mut total_sent = ethers::types::U256::zero();
        let mut incoming_count = 0;
        let mut outgoing_count = 0;
        for tx_with_receipt in transactions.transactions {
            let tx = tx_with_receipt.transaction;
            if tx.from == address_parsed {
                outgoing_count += 1;
                total_sent += tx.value;
            } else if tx.to.map(|to| to == address_parsed).unwrap_or(false) {
                incoming_count += 1;
                total_received += tx.value;
            }
        }
        Ok(TransactionStats {
            address: address_parsed,
            total_transactions: (incoming_count + outgoing_count) as u64,
            incoming_count: incoming_count as u64,
            outgoing_count: outgoing_count as u64,
            total_received,
            total_sent,
            first_seen_block: from_block.unwrap_or(0),
            last_seen_block: to_block.unwrap_or(0),
        })
    }

    /// Get transaction by hash
    ///
    /// # Example
    /// ```
    /// let tx_hash = "0x...".parse().unwrap();
    /// let transaction = trade_service.get_transaction_by_hash(tx_hash).await?;
    /// ```
    pub async fn get_transaction_by_hash(
        &self,
        tx_hash: H256,
    ) -> Result<Option<TransactionWithReceipt>, EvmError> {
        let tx = self
            .evm
            .client
            .provider
            .get_transaction(tx_hash)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transaction: {}", e)))?;
        if let Some(transaction) = tx {
            let receipt = self
                .evm
                .client
                .provider
                .get_transaction_receipt(tx_hash)
                .await
                .map_err(|e| EvmError::RpcError(format!("Failed to get receipt: {}", e)))?;
            Ok(Some(TransactionWithReceipt {
                transaction,
                receipt,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get balance history for an address
    ///
    /// # Example
    /// ```
    /// let history = trade_service.get_balance_history(
    ///     "0x...".to_string(),
    ///     1000000,
    ///     1001000,
    ///     100
    /// ).await?;
    /// ```
    pub async fn get_balance_history(
        &self,
        address: String,
        from_block: u64,
        to_block: u64,
        interval: u64,
    ) -> Result<Vec<BalanceSnapshot>, EvmError> {
        let address_parsed: Address = address
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address format: {}", e)))?;
        let mut snapshots = Vec::new();
        for block_number in (from_block..=to_block).step_by(interval as usize) {
            let balance = self
                .evm
                .client
                .provider
                .get_balance(address_parsed, Some(block_number.into()))
                .await
                .map_err(|e| EvmError::RpcError(format!("Failed to get balance: {}", e)))?;
            snapshots.push(BalanceSnapshot {
                block_number,
                balance,
                timestamp: 0,
            });
        }
        Ok(snapshots)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferEvent {
    pub from: Address,
    pub to: Address,
    pub value: ethers::types::U256,
    pub transaction_hash: H256,
    pub block_number: u64,
    pub log_index: u64,
}

impl TransferEvent {
    pub fn from_log(log: &Log) -> Result<Self, String> {
        if log.topics.len() != 3 {
            return Err(format!(
                "Invalid Transfer event log: expected 3 topics, got {}",
                log.topics.len()
            ));
        }
        let from_bytes = log.topics[1].as_bytes();
        if from_bytes.len() != 32 {
            return Err(format!("Invalid from topic length: {}", from_bytes.len()));
        }
        let from = Address::from_slice(&from_bytes[12..]);
        let to_bytes = log.topics[2].as_bytes();
        if to_bytes.len() != 32 {
            return Err(format!("Invalid to topic length: {}", to_bytes.len()));
        }
        let to = Address::from_slice(&to_bytes[12..]);
        let value = if log.data.is_empty() {
            ethers::types::U256::zero()
        } else {
            let mut data_bytes = [0u8; 32];
            let data_len = log.data.len();
            if data_len >= 32 {
                data_bytes.copy_from_slice(&log.data[..32]);
            } else {
                let start = 32 - data_len;
                data_bytes[start..].copy_from_slice(&log.data);
            }
            ethers::types::U256::from_big_endian(&data_bytes)
        };
        let transaction_hash = log
            .transaction_hash
            .ok_or("Missing transaction hash in log".to_string())?;
        let block_number = log
            .block_number
            .ok_or("Missing block number in log".to_string())?
            .as_u64();
        let log_index = log
            .log_index
            .ok_or("Missing log index in log".to_string())?
            .as_u64();
        Ok(TransferEvent {
            from,
            to,
            value,
            transaction_hash,
            block_number,
            log_index,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    pub address: Address,
    pub total_transactions: u64,
    pub incoming_count: u64,
    pub outgoing_count: u64,
    pub total_received: ethers::types::U256,
    pub total_sent: ethers::types::U256,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    pub block_number: u64,
    pub balance: ethers::types::U256,
    pub timestamp: u64,
}

/// Event listener for transaction monitoring
pub struct TradeEventListener {
    evm: Arc<Evm>,
}

impl TradeEventListener {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm: evm }
    }

    /// Watch for large transactions based on value threshold
    ///
    /// # Example
    /// ```
    /// let mut receiver = event_listener.watch_large_transactions(
    ///     U256::from(10u64.pow(18)), // 1 ETH
    ///     3
    /// ).await?;
    ///
    /// while let Some(tx) = receiver.recv().await {
    ///     println!("Large transaction: {:?}", tx.transaction.hash);
    /// }
    /// ```
    pub async fn watch_large_transactions(
        &self,
        min_value: ethers::types::U256,
        poll_interval_secs: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<TransactionWithReceipt>, EvmError> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let evm = self.evm.clone();
        let last_block = Arc::new(AtomicU64::new(0));
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        last_block.store(current_block.as_u64(), Ordering::SeqCst);
        tokio::spawn(async move {
            let mut poll_interval = interval(Duration::from_secs(poll_interval_secs));
            loop {
                poll_interval.tick().await;
                if let Err(e) =
                    Self::poll_large_transactions(&evm, &last_block, min_value, &tx).await
                {
                    tokio::time::sleep(Duration::from_secs(poll_interval_secs * 2)).await;
                }
            }
        });
        Ok(rx)
    }

    /// The core logic of polling large transactions
    async fn poll_large_transactions(
        evm: &Evm,
        last_block: &AtomicU64,
        min_value: ethers::types::U256,
        tx: &tokio::sync::mpsc::Sender<TransactionWithReceipt>,
    ) -> Result<(), EvmError> {
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        let current_block_num = current_block.as_u64();
        let from_block = last_block.load(Ordering::SeqCst) + 1;
        if from_block > current_block_num {
            return Ok(());
        }
        let to_block = current_block_num;
        for block_number in from_block..=to_block {
            if let Ok(Some(block)) = evm.client.provider.get_block_with_txs(block_number).await {
                for transaction in block.transactions {
                    if transaction.value >= min_value {
                        let receipt = evm
                            .client
                            .provider
                            .get_transaction_receipt(transaction.hash)
                            .await
                            .map_err(|e| {
                                EvmError::RpcError(format!("Failed to get receipt: {}", e))
                            })?;
                        let tx_with_receipt = TransactionWithReceipt {
                            transaction,
                            receipt,
                        };
                        if tx.send(tx_with_receipt).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    /// Watch for large ERC20 transfer events
    ///
    /// # Example
    /// ```
    /// let token_address = "0x...".to_string();
    /// let mut receiver = event_listener.watch_large_transfers(
    ///     Some(token_address),
    ///     U256::from(1000 * 10u64.pow(18)), // 1000 tokens
    ///     3
    /// ).await?;
    /// ```
    pub async fn watch_large_transfers(
        &self,
        token_address: Option<String>,
        min_value: ethers::types::U256,
        poll_interval_secs: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<LargeTransferEvent>, EvmError> {
        let token_address_parsed = match &token_address {
            Some(addr_str) => {
                let addr: Address = addr_str.parse().map_err(|e| {
                    EvmError::RpcError(format!("Invalid token address format: {}", e))
                })?;
                Some(addr)
            }
            None => None,
        };
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let evm = self.evm.clone();
        let last_block = Arc::new(AtomicU64::new(0));
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        last_block.store(current_block.as_u64(), Ordering::SeqCst);
        tokio::spawn(async move {
            let mut poll_interval = interval(Duration::from_secs(poll_interval_secs));
            loop {
                poll_interval.tick().await;
                if let Err(e) = Self::poll_large_transfers(
                    &evm,
                    &last_block,
                    token_address_parsed,
                    min_value,
                    &tx,
                )
                .await
                {
                    tokio::time::sleep(Duration::from_secs(poll_interval_secs * 2)).await;
                }
            }
        });
        Ok(rx)
    }

    /// The core logic of polling large transfer events
    async fn poll_large_transfers(
        evm: &Evm,
        last_block: &AtomicU64,
        token_address: Option<Address>,
        min_value: ethers::types::U256,
        tx: &tokio::sync::mpsc::Sender<LargeTransferEvent>,
    ) -> Result<(), EvmError> {
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        let current_block_num = current_block.as_u64();
        let from_block = last_block.load(Ordering::SeqCst) + 1;
        if from_block > current_block_num {
            return Ok(());
        }
        let to_block = if current_block_num - from_block > 1000 {
            from_block + 1000
        } else {
            current_block_num
        };
        // Build Transfer event filters
        let mut filter = Filter::new()
            .event("Transfer(address,address,uint256)")
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()));
        // If a token address is specified, transfer events for that token will be filtered.
        if let Some(token_addr) = token_address {
            filter = filter.address(token_addr);
        }
        let logs = evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transfer logs: {}", e)))?;
        for log in logs {
            match TransferEvent::from_log(&log) {
                Ok(transfer_event) => {
                    if transfer_event.value >= min_value {
                        let large_transfer = LargeTransferEvent {
                            token_address: log.address,
                            from: transfer_event.from,
                            to: transfer_event.to,
                            value: transfer_event.value,
                            transaction_hash: transfer_event.transaction_hash,
                            block_number: transfer_event.block_number,
                            log_index: transfer_event.log_index,
                        };
                        if tx.send(large_transfer).await.is_err() {
                            return Ok(());
                        }
                    }
                }
                Err(e) => {
                    error!(target: "[Trade Module]", "Failed to parse transfer event: {:?}", e);
                }
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    /// Watch large transfers for a specific token
    pub async fn watch_large_token_transfers(
        &self,
        token_address: String,
        min_value: ethers::types::U256,
    ) -> Result<tokio::sync::mpsc::Receiver<LargeTransferEvent>, EvmError> {
        self.watch_large_transfers(Some(token_address), min_value, 3)
            .await
    }

    /// Watch large transfers for all tokens
    pub async fn watch_all_large_transfers(
        &self,
        min_value: ethers::types::U256,
    ) -> Result<tokio::sync::mpsc::Receiver<LargeTransferEvent>, EvmError> {
        self.watch_large_transfers(None, min_value, 3).await
    }

    /// Watch all events for a specific address
    ///
    /// # Example
    /// ```
    /// let mut receiver = event_listener.watch_address_events(
    ///     "0x...".to_string()
    /// ).await?;
    /// ```
    pub async fn watch_address_events(
        &self,
        address: String,
    ) -> Result<tokio::sync::mpsc::Receiver<Log>, EvmError> {
        let address_parsed: Address = address
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address format: {}", e)))?;
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let evm = self.evm.clone();
        let last_block = Arc::new(AtomicU64::new(0));
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        last_block.store(current_block.as_u64(), Ordering::SeqCst);
        tokio::spawn(async move {
            let mut poll_interval = interval(Duration::from_secs(3));
            loop {
                poll_interval.tick().await;
                if let Err(e) = Self::poll_events(&evm, &last_block, address_parsed, &tx).await {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });
        Ok(rx)
    }

    /// The core logic of polling events
    async fn poll_events(
        evm: &Evm,
        last_block: &AtomicU64,
        address: Address,
        tx: &tokio::sync::mpsc::Sender<Log>,
    ) -> Result<(), EvmError> {
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        let current_block_num = current_block.as_u64();
        let from_block = last_block.load(Ordering::SeqCst) + 1;
        if from_block > current_block_num {
            return Ok(());
        }
        let to_block = if current_block_num - from_block > 1000 {
            from_block + 1000
        } else {
            current_block_num
        };
        let filter = Filter::new()
            .address(address)
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()));
        let logs = evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))?;
        for log in logs {
            if tx.send(log).await.is_err() {
                return Ok(());
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    /// Watch transfer events for a specific address
    pub async fn watch_transfer_events(
        &self,
        address: String,
    ) -> Result<tokio::sync::mpsc::Receiver<TransferEvent>, EvmError> {
        let address_parsed: Address = address
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address format: {}", e)))?;

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let evm = self.evm.clone();
        let last_block = Arc::new(AtomicU64::new(0));
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        last_block.store(current_block.as_u64(), Ordering::SeqCst);
        tokio::spawn(async move {
            let mut poll_interval = interval(Duration::from_secs(3));
            loop {
                poll_interval.tick().await;
                if let Err(e) =
                    Self::poll_transfer_events(&evm, &last_block, address_parsed, &tx).await
                {
                    error!(target: "[Trade Module]", "Error polling transfer events: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });
        Ok(rx)
    }

    /// The core logic of polling transfer events
    async fn poll_transfer_events(
        evm: &Evm,
        last_block: &AtomicU64,
        address: Address,
        tx: &tokio::sync::mpsc::Sender<TransferEvent>,
    ) -> Result<(), EvmError> {
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        let current_block_num = current_block.as_u64();
        let from_block = last_block.load(Ordering::SeqCst) + 1;
        if from_block > current_block_num {
            return Ok(());
        }
        let to_block = if current_block_num - from_block > 1000 {
            from_block + 1000
        } else {
            current_block_num
        };
        let filter = Filter::new()
            .address(address)
            .event("Transfer(address,address,uint256)")
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()));
        let logs = evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get transfer logs: {}", e)))?;
        for log in logs {
            match TransferEvent::from_log(&log) {
                Ok(transfer_event) => {
                    if tx.send(transfer_event).await.is_err() {
                        return Ok(());
                    }
                }
                Err(e) => {
                    error!(target: "[Trade Module]", "Failed to parse transfer event: {:?}", e);
                }
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    /// Watch address events with custom configuration
    pub async fn watch_address_events_with_config(
        &self,
        address: String,
        poll_interval_secs: u64,
        max_blocks_per_poll: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<Log>, EvmError> {
        let address_parsed: Address = address
            .parse()
            .map_err(|e| EvmError::RpcError(format!("Invalid address format: {}", e)))?;
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let evm = self.evm.clone();
        let last_block = Arc::new(AtomicU64::new(0));
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        last_block.store(current_block.as_u64(), Ordering::SeqCst);
        tokio::spawn(async move {
            let mut poll_interval = interval(Duration::from_secs(poll_interval_secs));
            loop {
                poll_interval.tick().await;
                if let Err(e) = Self::poll_events_with_config(
                    &evm,
                    &last_block,
                    address_parsed,
                    &tx,
                    max_blocks_per_poll,
                )
                .await
                {
                    error!(target: "[Trade Module]", "Error polling events: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(poll_interval_secs * 2)).await;
                }
            }
        });
        Ok(rx)
    }

    /// Polling logic with configuration
    async fn poll_events_with_config(
        evm: &Evm,
        last_block: &AtomicU64,
        address: Address,
        tx: &tokio::sync::mpsc::Sender<Log>,
        max_blocks_per_poll: u64,
    ) -> Result<(), EvmError> {
        let current_block = evm
            .client
            .provider
            .get_block_number()
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get current block: {}", e)))?;
        let current_block_num = current_block.as_u64();
        let from_block = last_block.load(Ordering::SeqCst) + 1;
        if from_block > current_block_num {
            return Ok(());
        }
        let to_block = if current_block_num - from_block > max_blocks_per_poll {
            from_block + max_blocks_per_poll
        } else {
            current_block_num
        };
        let filter = Filter::new()
            .address(address)
            .from_block(BlockNumber::Number(from_block.into()))
            .to_block(BlockNumber::Number(to_block.into()));
        let logs = evm
            .client
            .provider
            .get_logs(&filter)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get logs: {}", e)))?;
        for log in logs {
            if tx.send(log).await.is_err() {
                return Ok(());
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    pub fn stop_event_listener(receiver: tokio::sync::mpsc::Receiver<Log>) {
        drop(receiver);
    }
}

/// Large transfer event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeTransferEvent {
    pub token_address: Address,
    pub from: Address,
    pub to: Address,
    pub value: ethers::types::U256,
    pub transaction_hash: H256,
    pub block_number: u64,
    pub log_index: u64,
}

impl LargeTransferEvent {
    /// Formatted display of large transfer information
    pub fn display(&self) -> String {
        format!(
            "Large Transfer: {} {} from {:?} to {:?} in tx {:?}",
            ethers::utils::format_units(self.value, 18).unwrap_or("N/A".to_string()),
            self.token_short_name(),
            self.from,
            self.to,
            self.transaction_hash
        )
    }

    /// Get the token short name
    pub fn token_short_name(&self) -> String {
        format!("{:?}...", &self.token_address.to_string()[..8])
    }

    pub fn to_transfer_event(&self) -> TransferEvent {
        TransferEvent {
            from: self.from,
            to: self.to,
            value: self.value,
            transaction_hash: self.transaction_hash,
            block_number: self.block_number,
            log_index: self.log_index,
        }
    }
}

/// Large transaction monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeTransactionConfig {
    pub min_value: ethers::types::U256,
    pub poll_interval_secs: u64,
    pub include_failed: bool,
    pub watch_tokens: Vec<Address>,
}

impl Default for LargeTransactionConfig {
    fn default() -> Self {
        Self {
            min_value: ethers::types::U256::from(10u64.pow(18)), // 1 ETH
            poll_interval_secs: 3,
            include_failed: false,
            watch_tokens: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    pub hash: H256,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_price: Option<U256>,
    pub gas: U256,
    pub gas_used: Option<U256>,
    pub input: Vec<u8>,
    pub block_number: Option<u64>,
    pub transaction_index: Option<u64>,
    pub timestamp: Option<U256>,
    pub status: Option<u64>,
    pub is_contract_creation: bool,
    pub hash_short: String,
    pub receipt: Option<TransactionReceipt>,
    pub raw_transaction: Transaction,
    pub contract_address: Option<Address>,
    pub transaction_type: Option<u64>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub max_fee_per_gas: Option<U256>,
    pub chain_id: Option<U256>,
    pub logs: Vec<Log>,
    pub is_success: bool,
    pub total_gas_cost: Option<U256>,
    pub token_decimals_cache: std::collections::HashMap<Address, u8>,
}

impl TransactionInfo {
    pub fn get_received_token(&self) -> Option<(Address, ethers::types::U256)> {
        if !self.is_success {
            return None;
        }
        for log in &self.logs {
            if log.topics.len() == 3 {
                match TransferEvent::from_log(log) {
                    Ok(transfer) => {
                        let value = transfer.value;
                        if value > ethers::types::U256::from(1)
                            && value
                                < ethers::types::U256::from(10).pow(ethers::types::U256::from(30))
                        {
                            return Some((log.address, value));
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        None
    }

    pub fn get_spent_token(&self) -> Option<(Address, ethers::types::U256)> {
        if !self.is_success {
            return None;
        }
        let mut valid_transfers = Vec::new();
        for log in &self.logs {
            if log.topics.len() == 3 {
                match TransferEvent::from_log(log) {
                    Ok(transfer) => {
                        let value = transfer.value;
                        if value > ethers::types::U256::from(1)
                            && value
                                < ethers::types::U256::from(10).pow(ethers::types::U256::from(30))
                        {
                            valid_transfers.push((log.address, value));
                        }
                    }
                    Err(_) => continue,
                }
            }
        }
        if valid_transfers.len() >= 2 {
            return Some(valid_transfers[1]);
        }
        if valid_transfers.len() == 1 {
            return Some(valid_transfers[0]);
        }
        None
    }

    pub fn get_received_token_eth(&self) -> Option<(Address, f64)> {
        self.get_received_token().and_then(|(addr, amount)| {
            let decimals = self.get_token_decimals(&addr);
            let decimal_amount = amount.as_u128() as f64 / 10_u64.pow(decimals as u32) as f64;
            Some((addr, decimal_amount))
        })
    }

    pub fn get_spent_token_eth(&self) -> Option<(Address, f64)> {
        self.get_spent_token().and_then(|(addr, amount)| {
            let decimals = self.get_token_decimals(&addr);
            let decimal_amount = amount.as_u128() as f64 / 10_u64.pow(decimals as u32) as f64;
            Some((addr, decimal_amount))
        })
    }

    fn get_token_decimals(&self, token_address: &Address) -> u8 {
        *self.token_decimals_cache.get(token_address).unwrap_or(&18)
    }

    fn getDirection(&self) -> Direction {
        if (is_quote(&format!("{:?}", &self.get_spent_token_eth().unwrap().0))) {
            Direction::Buy
        } else {
            Direction::Sell
        }
    }
}

#[cfg(test)]
mod test {
    use evm_client::EvmType;

    use crate::{Evm, trade::Trade};
    use std::{sync::Arc, time::Duration};

    #[tokio::test]
    async fn test_get_transaction_by_tx() {
        let evm = Evm::new(evm_client::EvmType::ETHEREUM_MAINNET)
            .await
            .unwrap();
        let trade = Trade::new(Arc::new(evm));
        let t = trade
            .get_transactions_by_tx(
                "0x2c632c004c7a2c5daedf54f53a7ab424756b383bfc477bfc802e3a1d5a930a2e",
            )
            .await
            .unwrap();
        // reality received
        let received = t.get_received_token_eth();
        // reality spent
        let spent = t.get_spent_token_eth();
        println!("Actual Received {:?}", received);
        println!("Actual Spent {:?}", spent);
    }
}
