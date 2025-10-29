use crate::{Evm, EvmError};
use ethers::{
    providers::Middleware,
    types::{
        Address, BlockNumber, Filter, H256, Log, Transaction, TransactionReceipt, ValueOrArray,
    },
};
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
    pub address: Address,
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
pub struct TradeService {
    evm: Arc<Evm>,
}

impl TradeService {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm: evm }
    }

    /// Get transactions for a specific address with filtering and pagination
    ///
    /// # Example
    /// ```
    /// let query = TransactionQuery {
    ///     address: "0x...".parse().unwrap(),
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
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(50);
        let mut filter = Filter::new().address(ValueOrArray::Value(query.address));
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
    /// let address_a = "0x...".parse().unwrap();
    /// let address_b = "0x...".parse().unwrap();
    /// let transactions = trade_service.get_transactions_involving_addresses(
    ///     address_a,
    ///     address_b,
    ///     Some(1000000),
    ///     Some(1001000)
    /// ).await?;
    /// ```
    pub async fn get_transactions_involving_addresses(
        &self,
        address_a: Address,
        address_b: Address,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<TransactionWithReceipt>, EvmError> {
        let mut filter = Filter::new().address(ValueOrArray::Array(vec![address_a, address_b]));
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
                    let involves_both = tx.from == address_a
                        || tx.from == address_b
                        || tx
                            .to
                            .map(|to| to == address_a || to == address_b)
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
    /// let receiver = "0x...".parse().unwrap();
    /// let sender = "0x...".parse().unwrap();
    /// let transactions = trade_service.get_transactions_from_b_to_a(
    ///     receiver,
    ///     sender,
    ///     Some(1000000),
    ///     Some(1001000)
    /// ).await?;
    /// ```
    pub async fn get_transactions_from_b_to_a(
        &self,
        receiver: Address, // receiver addresss
        sender: Address,   // sender address
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<TransactionWithReceipt>, EvmError> {
        let mut filter = Filter::new().address(ValueOrArray::Value(receiver));
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
                    if tx.from == sender && tx.to.map(|to| to == receiver).unwrap_or(false) {
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
    ///     "0x...".parse().unwrap(),
    ///     Some(1000000),
    ///     Some(1001000)
    /// ).await?;
    /// ```
    pub async fn get_transaction_stats(
        &self,
        address: Address,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<TransactionStats, EvmError> {
        let query = TransactionQuery {
            address,
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
            if tx.from == address {
                outgoing_count += 1;
                total_sent += tx.value;
            } else if tx.to.map(|to| to == address).unwrap_or(false) {
                incoming_count += 1;
                total_received += tx.value;
            }
        }
        Ok(TransactionStats {
            address,
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
    ///     "0x...".parse().unwrap(),
    ///     1000000,
    ///     1001000,
    ///     100
    /// ).await?;
    /// ```
    pub async fn get_balance_history(
        &self,
        address: Address,
        from_block: u64,
        to_block: u64,
        interval: u64,
    ) -> Result<Vec<BalanceSnapshot>, EvmError> {
        let mut snapshots = Vec::new();
        for block_number in (from_block..=to_block).step_by(interval as usize) {
            let balance = self
                .evm
                .client
                .provider
                .get_balance(address, Some(block_number.into()))
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
            return Err("Invalid Transfer event log: expected 3 topics".to_string());
        }
        let from = Address::from_slice(&log.topics[1].as_bytes()[12..]);
        let to = Address::from_slice(&log.topics[2].as_bytes()[12..]);
        let value = ethers::types::U256::from_big_endian(&log.data);
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
                    eprintln!("Error polling large transactions: {}", e);
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
    /// let token_address = "0x...".parse().unwrap();
    /// let mut receiver = event_listener.watch_large_transfers(
    ///     Some(token_address),
    ///     U256::from(1000 * 10u64.pow(18)), // 1000 tokens
    ///     3
    /// ).await?;
    /// ```
    pub async fn watch_large_transfers(
        &self,
        token_address: Option<Address>,
        min_value: ethers::types::U256,
        poll_interval_secs: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<LargeTransferEvent>, EvmError> {
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
                    Self::poll_large_transfers(&evm, &last_block, token_address, min_value, &tx)
                        .await
                {
                    eprintln!("Error polling large transfers: {}", e);
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
                    eprintln!("Failed to parse transfer event: {}", e);
                }
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    /// Watch large transfers for a specific token
    pub async fn watch_large_token_transfers(
        &self,
        token_address: Address,
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
    ///     "0x...".parse().unwrap()
    /// ).await?;
    /// ```
    pub async fn watch_address_events(
        &self,
        address: Address,
    ) -> Result<tokio::sync::mpsc::Receiver<Log>, EvmError> {
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
                if let Err(e) = Self::poll_events(&evm, &last_block, address, &tx).await {
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
        address: Address,
    ) -> Result<tokio::sync::mpsc::Receiver<TransferEvent>, EvmError> {
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
                if let Err(e) = Self::poll_transfer_events(&evm, &last_block, address, &tx).await {
                    eprintln!("Error polling transfer events: {}", e);
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
                    eprintln!("Failed to parse transfer event: {}", e);
                }
            }
        }
        last_block.store(to_block, Ordering::SeqCst);
        Ok(())
    }

    /// Watch address events with custom configuration
    pub async fn watch_address_events_with_config(
        &self,
        address: Address,
        poll_interval_secs: u64,
        max_blocks_per_poll: u64,
    ) -> Result<tokio::sync::mpsc::Receiver<Log>, EvmError> {
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
                    address,
                    &tx,
                    max_blocks_per_poll,
                )
                .await
                {
                    eprintln!("Error polling events: {}", e);
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
