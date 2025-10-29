/// This module provides memory pool-related functionalities.
use crate::Evm;
use crate::types::EvmError;
use ethers::providers::Middleware;
use ethers::types::Bytes;
use ethers::types::{Address, U256};
use ethers::types::{Filter, Transaction, TxHash};
use sha3::{Digest, Keccak256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

/// Represents a transaction in the mempool
#[derive(Debug, Clone)]
pub struct MempoolTransaction {
    pub hash: TxHash,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub transaction_type: Option<u64>,
    pub gas_price: Option<U256>,                // legacy trade
    pub max_fee_per_gas: Option<U256>,          // EIP-1559
    pub max_priority_fee_per_gas: Option<U256>, // EIP-1559
    pub gas: U256,
    pub input: Bytes,
    pub nonce: U256,
    pub transaction: Transaction,
    pub first_seen: u64,
    pub last_seen: u64,
    pub is_mev: bool,
    pub bundle_hash: Option<TxHash>,
    pub frontrunning_protection: bool,
}

/// Configuration for mempool monitoring
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    pub poll_interval: Duration,
    pub max_transactions: usize,
    pub track_pending: bool,
    pub enable_mev_detection: bool,
    pub max_reorg_depth: u64,
    pub simulate_transactions: bool,
    pub track_bundles: bool,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(2),
            max_transactions: 10000,
            track_pending: true,
            enable_mev_detection: false,
            max_reorg_depth: 5,
            simulate_transactions: false,
            track_bundles: false,
        }
    }
}

/// Listener for monitoring mempool transactions
#[derive(Clone)]
pub struct MempoolListener {
    evm: Arc<Evm>,
    config: MempoolConfig,
    state: Arc<RwLock<MempoolState>>,
}

/// Internal state of the mempool
#[derive(Debug)]
struct MempoolState {
    transactions: HashMap<TxHash, MempoolTransaction>,
    pending_hashes: HashSet<TxHash>,
    last_block_number: u64,
    is_running: bool,
    // Transaction package tracking
    // bundle_hash -> [tx_hashes]
    transaction_bundles: HashMap<TxHash, Vec<TxHash>>,
}

impl MempoolListener {
    /// Creates a new MempoolListener with default configuration
    pub fn new(evm: Arc<Evm>) -> Self {
        Self::with_config(evm, MempoolConfig::default())
    }

    /// Creates a new MempoolListener with custom configuration
    pub fn with_config(evm: Arc<Evm>, config: MempoolConfig) -> Self {
        Self {
            evm,
            config,
            state: Arc::new(RwLock::new(MempoolState {
                transactions: HashMap::new(),
                pending_hashes: HashSet::new(),
                last_block_number: 0,
                is_running: false,
                transaction_bundles: HashMap::new(),
            })),
        }
    }

    /// Starts the mempool listener
    ///
    /// # Example
    /// ```
    /// let evm = Arc::new(Evm::new());
    /// let listener = MempoolListener::new(evm);
    /// listener.start().await?;
    /// ```
    pub async fn start(&self) -> Result<(), EvmError> {
        let mut state = self.state.write().await;
        if state.is_running {
            return Err(EvmError::MempoolError(
                "Mempool listener is already running".to_string(),
            ));
        }
        state.is_running = true;
        drop(state);

        let listener = self.clone();
        tokio::spawn(async move {
            listener.run().await;
        });

        Ok(())
    }

    /// Stops the mempool listener
    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        state.is_running = false;
    }

    /// Main run loop
    async fn run(&self) {
        while self.is_running().await {
            if let Err(e) = self.poll_mempool().await {
                eprintln!("Error polling mempool: {}", e);
            }
            sleep(self.config.poll_interval).await;
        }
    }

    /// Checks if the listener is running
    async fn is_running(&self) -> bool {
        self.state.read().await.is_running
    }

    /// Polls the mempool for pending transactions
    async fn poll_mempool(&self) -> Result<(), EvmError> {
        let current_block = self.evm.get_block_number().await?;
        {
            let mut state = self.state.write().await;
            state.last_block_number = current_block;
        }
        let pending_txs = self.get_pending_transactions().await?;
        self.update_mempool_state(pending_txs, current_block).await;
        self.clean_confirmed_transactions().await?;
        Ok(())
    }

    /// Retrieves pending transactions from the mempool using the standard JSON-RPC method.
    /// This method queries the pending block to get transactions that are currently
    ///
    /// # Features
    /// Timeout Control: 30-second timeout for pending block retrieval
    /// Parallel Processing: Uses `tokio::spawn` to fetch transaction details concurrently
    /// Retry Mechanism: 3 retry attempts with exponential backoff for failed requests
    /// Error Resilience: Continues processing even if individual transactions fail
    ///
    /// # Workflow
    /// 1. Fetches the pending block with timeout protection
    /// 2. Spawns parallel tasks for each transaction hash
    /// 3. Each task retries up to 3 times to get transaction details
    /// 4. Filters out confirmed transactions and handles errors gracefully
    /// 5. Collects all valid pending transactions
    ///
    /// # Returns
    /// - `Ok(Vec<Transaction>)`: Vector of valid pending transactions
    /// - `Err(EvmError)`: If the initial block fetch fails or times out
    ///
    /// # Example
    /// ```rust
    /// let pending_txs = listener.get_pending_transactions().await?;
    /// for tx in pending_txs {
    ///     println!("Pending TX: {:?} from {:?}", tx.hash, tx.from);
    /// }
    /// ```
    ///
    async fn get_pending_transactions(&self) -> Result<Vec<Transaction>, EvmError> {
        // timeout control
        let block = tokio::time::timeout(
            Duration::from_secs(30),
            self.evm
                .client
                .provider
                .get_block(ethers::types::BlockId::Number(
                    ethers::types::BlockNumber::Pending,
                )),
        )
        .await
        .map_err(|_| EvmError::RpcError("Timeout getting pending block".to_string()))?
        .map_err(|e| EvmError::RpcError(format!("Failed to get pending block: {}", e)))?;
        let mut pending_txs = Vec::new();
        if let Some(block) = block {
            // parallel tasks
            let mut handles = Vec::new();
            for tx_hash in block.transactions {
                let provider = self.evm.client.provider.clone();
                let handle = tokio::spawn(async move {
                    // try again
                    for attempt in 0..3 {
                        match provider.get_transaction(tx_hash).await {
                            Ok(Some(tx)) if tx.block_number.is_none() => return Some(tx),
                            Ok(Some(_)) => return None, // confirmed transactions
                            Ok(None) => return None,    // transaction does not exist
                            Err(_) if attempt < 2 => {
                                tokio::time::sleep(Duration::from_millis(100 * (attempt + 1)))
                                    .await;
                                continue;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to get transaction {} after {} attempts: {}",
                                    tx_hash,
                                    attempt + 1,
                                    e
                                );
                                return None;
                            }
                        }
                    }
                    None
                });
                handles.push(handle);
            }
            // waiting for all tasks to complete
            for handle in handles {
                match handle.await {
                    Ok(Some(tx)) => pending_txs.push(tx),
                    Ok(None) => {} // Skip invalid transaction
                    Err(e) => eprintln!("Task failed: {}", e),
                }
            }
        }
        Ok(pending_txs)
    }

    /// Updates the mempool state with new transactions
    async fn update_mempool_state(&self, transactions: Vec<Transaction>, current_block: u64) {
        let mut state = self.state.write().await;
        // collect all new transactions for check package.
        let new_transactions: Vec<Transaction> = transactions
            .into_iter()
            .filter(|tx| !state.transactions.contains_key(&tx.hash))
            .collect();
        // detect transaction packages
        let bundles = if self.config.track_bundles {
            Self::detect_transaction_bundles(&new_transactions)
        } else {
            HashMap::new()
        };
        // update transaction packages status
        for (bundle_hash, tx_hashes) in bundles {
            state
                .transaction_bundles
                .insert(bundle_hash, tx_hashes.clone());
        }
        // handle a single transaction
        for tx in new_transactions {
            if state.transactions.len() < self.config.max_transactions {
                let is_mev = self.config.enable_mev_detection && Self::detect_mev_transaction(&tx);
                let frontrunning_protection = Self::has_frontrunning_protection(&tx);
                // Find the package to which the transaction belongs
                let bundle_hash =
                    Self::find_bundle_for_transaction(&tx, &state.transaction_bundles);
                let mempool_tx = MempoolTransaction {
                    hash: tx.hash,
                    from: tx.from,
                    to: tx.to,
                    value: tx.value,
                    transaction_type: tx.transaction_type.map(|v| v.as_u64()),
                    gas_price: tx.gas_price,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    max_priority_fee_per_gas: tx.max_priority_fee_per_gas,
                    gas: tx.gas,
                    input: tx.input.clone(),
                    nonce: tx.nonce,
                    transaction: tx.clone(),
                    first_seen: current_block,
                    last_seen: current_block,
                    is_mev,
                    bundle_hash,
                    frontrunning_protection,
                };
                state.transactions.insert(tx.hash, mempool_tx);
                state.pending_hashes.insert(tx.hash);
            }
        }
    }

    /// Remove confirmed transactions and transaction packages from the memory pool.
    async fn clean_confirmed_transactions(&self) -> Result<(), EvmError> {
        let current_block = self.evm.get_block_number().await?;
        let mut state = self.state.write().await;
        let mut to_remove = Vec::new();
        for (tx_hash, mempool_tx) in state.transactions.iter() {
            if current_block.saturating_sub(mempool_tx.last_seen) > self.config.max_reorg_depth {
                if let Ok(Some(receipt)) = self.evm.get_transaction_receipt(*tx_hash).await {
                    if receipt.block_number.is_some() {
                        to_remove.push(*tx_hash);
                    }
                }
            }
        }
        for tx_hash in to_remove {
            state.transactions.remove(&tx_hash);
            state.pending_hashes.remove(&tx_hash);
            state.transaction_bundles.retain(|_, tx_hashes| {
                tx_hashes.retain(|h| h != &tx_hash);
                !tx_hashes.is_empty()
            });
        }
        Ok(())
    }

    /// Returns all pending transactions in the mempool
    ///
    /// # Example
    /// ```
    /// let transactions = listener.get_pending_transactions_list().await;
    /// for tx in transactions {
    ///     println!("Transaction hash: {:?}", tx.hash);
    /// }
    /// ```
    pub async fn get_pending_transactions_list(&self) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state.transactions.values().cloned().collect()
    }

    /// Returns pending transactions sent by a specific address
    ///
    /// # Example
    /// ```
    /// let address: Address = "0x...".parse().unwrap();
    /// let sender_txs = listener.get_transactions_by_sender(address).await;
    /// ```
    pub async fn get_transactions_by_sender(&self, address: Address) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state
            .transactions
            .values()
            .filter(|tx| tx.from == address)
            .cloned()
            .collect()
    }

    /// Returns pending transactions sent to a specific address
    ///
    /// # Example
    /// ```
    /// let address: Address = "0x...".parse().unwrap();
    /// let receiver_txs = listener.get_transactions_by_receiver(address).await;
    /// ```
    pub async fn get_transactions_by_receiver(&self, address: Address) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state
            .transactions
            .values()
            .filter(|tx| tx.to == Some(address))
            .cloned()
            .collect()
    }

    /// Returns transactions with gas price above the specified threshold
    ///
    /// # Example
    /// ```
    /// let threshold = U256::from(100_000_000_000u64); // 100 Gwei
    /// let high_gas_txs = listener.get_high_gas_transactions(threshold).await;
    /// ```
    pub async fn get_high_gas_transactions(&self, threshold: U256) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state
            .transactions
            .values()
            .filter(|tx| tx.gas_price >= Some(threshold))
            .cloned()
            .collect()
    }

    /// Returns transactions with value above the specified threshold
    ///
    /// # Example
    /// ```
    /// let threshold = U256::from(1_000_000_000_000_000_000u64); // 1 ETH
    /// let large_txs = listener.get_large_value_transactions(threshold).await;
    /// ```
    pub async fn get_large_value_transactions(&self, threshold: U256) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state
            .transactions
            .values()
            .filter(|tx| tx.value >= threshold)
            .cloned()
            .collect()
    }

    /// Returns statistics about the current mempool state
    ///
    /// # Example
    /// ```
    /// let stats = listener.get_stats().await;
    /// println!("Total transactions: {}", stats.total_transactions);
    /// println!("Total value: {}", stats.total_value);
    /// ```
    pub async fn get_stats(&self) -> MempoolStats {
        let state = self.state.read().await;
        let total_txs = state.transactions.len();
        let total_value = state
            .transactions
            .values()
            .fold(U256::zero(), |acc, tx| acc + tx.value);
        let total_gas_limit = state
            .transactions
            .values()
            .fold(U256::zero(), |acc, tx| acc + tx.gas);
        // Calculate average gas price (supports EIP-1559)
        let (gas_sum, count) =
            state
                .transactions
                .values()
                .fold((U256::zero(), 0), |(sum, count), tx| {
                    let effective_gas_price = tx
                        .max_fee_per_gas
                        .unwrap_or_else(|| tx.gas_price.unwrap_or_default());
                    (sum + effective_gas_price, count + 1)
                });
        let avg_gas_price = if count > 0 {
            gas_sum / U256::from(count)
        } else {
            U256::zero()
        };
        MempoolStats {
            total_transactions: total_txs,
            total_value,
            total_gas: total_gas_limit,
            average_gas_price: avg_gas_price,
            last_block_number: state.last_block_number,
            // eip1559 transactions, eth2.0 support
            eip1559_transactions: state
                .transactions
                .values()
                .filter(|tx| tx.transaction_type == Some(2))
                .count(),
            mev_transactions: state.transactions.values().filter(|tx| tx.is_mev).count(),
            protected_transactions: state
                .transactions
                .values()
                .filter(|tx| tx.frontrunning_protection)
                .count(),
        }
    }

    /// Checks if a specific transaction is in the mempool
    ///
    /// # Example
    /// ```
    /// let tx_hash: TxHash = "0x...".parse().unwrap();
    /// if listener.contains_transaction(tx_hash).await {
    ///     println!("Transaction is pending");
    /// }
    /// ```
    pub async fn contains_transaction(&self, tx_hash: TxHash) -> bool {
        let state = self.state.read().await;
        state.transactions.contains_key(&tx_hash)
    }

    /// Returns details for a specific transaction
    ///
    /// # Example
    /// ```
    /// let tx_hash: TxHash = "0x...".parse().unwrap();
    /// if let Some(tx) = listener.get_transaction_details(tx_hash).await {
    ///     println!("From: {:?}, To: {:?}, Value: {}", tx.from, tx.to, tx.value);
    /// }
    /// ```
    pub async fn get_transaction_details(&self, tx_hash: TxHash) -> Option<MempoolTransaction> {
        let state = self.state.read().await;
        state.transactions.get(&tx_hash).cloned()
    }

    fn detect_mev_transaction(tx: &Transaction) -> bool {
        let input_str = hex::encode(&tx.input);
        input_str.contains("0x6a761202")
            || tx.value.is_zero() && !tx.input.is_empty()
            || tx.gas_price.unwrap_or_default() > U256::from(100_000_000_000u64)
    }

    fn has_frontrunning_protection(tx: &Transaction) -> bool {
        tx.max_priority_fee_per_gas.is_some() && // 使用 EIP-1559
        tx.input.len() > 1000
    }

    /// get eip1559 transactions
    pub async fn get_eip1559_transactions(&self) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state
            .transactions
            .values()
            .filter(|tx| tx.transaction_type == Some(2))
            .cloned()
            .collect()
    }

    /// get mev transactions
    pub async fn get_mev_transactions(&self) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        state
            .transactions
            .values()
            .filter(|tx| tx.is_mev)
            .cloned()
            .collect()
    }

    fn detect_transaction_bundles(transactions: &[Transaction]) -> HashMap<TxHash, Vec<TxHash>> {
        let mut bundles = HashMap::new();
        let mut by_sender: HashMap<Address, Vec<&Transaction>> = HashMap::new();
        for tx in transactions {
            by_sender.entry(tx.from).or_default().push(tx);
        }
        for (sender, txs) in by_sender {
            let mut sorted_txs = txs;
            sorted_txs.sort_by_key(|tx| tx.nonce);
            let mut current_bundle = Vec::new();
            for window in sorted_txs.windows(2) {
                if let [prev, current] = window {
                    if current.nonce == prev.nonce + U256::one() {
                        if current_bundle.is_empty() {
                            current_bundle.push(prev.hash);
                        }
                        current_bundle.push(current.hash);
                    } else if !current_bundle.is_empty() {
                        if current_bundle.len() > 1 {
                            let bundle_hash = Self::calculate_bundle_hash(&current_bundle);
                            bundles.insert(bundle_hash, current_bundle.clone());
                        }
                        current_bundle.clear();
                    }
                }
            }
            if current_bundle.len() > 1 {
                let bundle_hash = Self::calculate_bundle_hash(&current_bundle);
                bundles.insert(bundle_hash, current_bundle);
            }
        }
        let mut by_contract: HashMap<Address, Vec<&Transaction>> = HashMap::new();
        for tx in transactions {
            if let Some(to) = tx.to {
                by_contract.entry(to).or_default().push(tx);
            }
        }
        for (contract, txs) in by_contract {
            if txs.len() > 1 {
                let mut by_selector: HashMap<Vec<u8>, Vec<TxHash>> = HashMap::new();
                for tx in txs {
                    if tx.input.len() >= 4 {
                        let selector = tx.input.0[..4].to_vec();
                        by_selector.entry(selector).or_default().push(tx.hash);
                    }
                }
                for (_, tx_hashes) in by_selector {
                    if tx_hashes.len() > 1 {
                        let bundle_hash = Self::calculate_bundle_hash(&tx_hashes);
                        bundles.insert(bundle_hash, tx_hashes);
                    }
                }
            }
        }
        bundles
    }

    fn calculate_bundle_hash(tx_hashes: &[TxHash]) -> TxHash {
        let mut hasher = Keccak256::new();
        for tx_hash in tx_hashes {
            hasher.update(tx_hash.as_bytes());
        }
        let result = hasher.finalize();
        TxHash::from_slice(&result[..32])
    }

    fn find_bundle_for_transaction(
        tx: &Transaction,
        bundles: &HashMap<TxHash, Vec<TxHash>>,
    ) -> Option<TxHash> {
        for (bundle_hash, tx_hashes) in bundles {
            if tx_hashes.contains(&tx.hash) {
                return Some(*bundle_hash);
            }
        }
        None
    }

    pub async fn get_bundle_transactions(&self, bundle_hash: TxHash) -> Vec<MempoolTransaction> {
        let state = self.state.read().await;
        if let Some(tx_hashes) = state.transaction_bundles.get(&bundle_hash) {
            tx_hashes
                .iter()
                .filter_map(|tx_hash| state.transactions.get(tx_hash))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn get_all_bundles(&self) -> HashMap<TxHash, Vec<TxHash>> {
        let state = self.state.read().await;
        state.transaction_bundles.clone()
    }
}

/// Statistics about the mempool state
#[derive(Debug, Clone)]
pub struct MempoolStats {
    pub total_transactions: usize,
    pub total_value: U256,
    pub total_gas: U256,
    pub average_gas_price: U256,
    pub last_block_number: u64,
    pub eip1559_transactions: usize,
    pub mev_transactions: usize,
    pub protected_transactions: usize,
}

/// managing mempool service
#[derive(Clone)]
pub struct MempoolService {
    evm: Arc<Evm>,
}

impl MempoolService {
    /// Creates a new MempoolService
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm: evm }
    }

    /// Creates a new mempool listener
    ///
    /// # Example
    /// ```
    /// let service = MempoolService::new(evm);
    /// let listener = service.create_listener();
    /// ```
    pub fn create_listener(&self) -> MempoolListener {
        MempoolListener::new(self.evm.clone())
    }

    /// Creates a new mempool listener with custom configuration
    ///
    /// # Example
    /// ```
    /// let config = MempoolConfig {
    ///     poll_interval: Duration::from_secs(5),
    ///     max_transactions: 5000,
    ///     track_pending: true,
    /// };
    /// let listener = service.create_listener_with_config(config);
    /// ```
    pub fn create_listener_with_config(&self, config: MempoolConfig) -> MempoolListener {
        MempoolListener::with_config(self.evm.clone(), config)
    }

    /// Quickly gets the count of pending transactions
    ///
    /// # Example
    /// ```
    /// let count = service.get_pending_transaction_count().await?;
    /// println!("Pending transactions: {}", count);
    /// ```
    pub async fn get_pending_transaction_count(&self) -> Result<usize, EvmError> {
        let filter = Filter::new().from_block(ethers::types::BlockNumber::Latest);
        let logs = self.evm.get_logs(filter).await?;
        Ok(logs.len())
    }

    /// Gets the current suggested gas price
    ///
    /// # Example
    /// ```
    /// let gas_price = service.get_suggested_gas_price().await?;
    /// println!("Suggested gas price: {}", gas_price);
    /// ```
    pub async fn get_suggested_gas_price(&self) -> Result<U256, EvmError> {
        self.evm.get_gas_price().await
    }
}
