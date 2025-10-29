/// The abstraction layer module for smart contracts.
use crate::Evm;
use crate::EvmError;
use ethers::providers::Middleware;
use ethers::types::{Address, Bytes, H256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Basic contract information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub address: Address,
    pub bytecode: Bytes,
    pub deployed_bytecode: Bytes,
    pub is_contract: bool,
    pub creation_block: Option<u64>,
    pub creation_tx_hash: Option<H256>,
    pub storage_slots: HashMap<H256, H256>,
}

/// Contract ABI information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractABI {
    pub raw_abi: Option<String>,
    pub functions: Vec<FunctionInfo>,
    pub events: Vec<EventInfo>,
    pub errors: Vec<ErrorInfo>,
}

/// Function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub inputs: Vec<Param>,
    pub outputs: Vec<Param>,
    pub constant: bool,
    pub payable: bool,
    pub selector: Option<H256>,
}

/// Event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInfo {
    pub name: String,
    pub inputs: Vec<Param>,
    pub anonymous: bool,
    pub signature: Option<H256>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub name: String,
    pub inputs: Vec<Param>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub type_: String,
    pub indexed: bool,
}

/// Storage layout analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLayout {
    pub slots: Vec<StorageSlot>,
    pub total_size: usize,
}

/// Storage slot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSlot {
    pub slot: H256,
    pub value: H256,
    pub size: usize,
}

/// Contract analyzer for EVM-based contracts
pub struct ContractAnalyzer {
    evm: Arc<Evm>,
}

impl ContractAnalyzer {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm }
    }

    /// Retrieves comprehensive contract information
    ///
    /// # Example
    /// ```rust
    /// use ethers::types::Address;
    /// use std::str::FromStr;
    ///
    /// let analyzer = ContractAnalyzer::new(evm_client);
    /// let address = Address::from_str("0x742d35Cc6634C0532925a3b8D6B6f7C93D5A7A7A")?;
    /// let contract_info = analyzer.get_contract_info(address).await?;
    /// println!("Contract bytecode length: {}", contract_info.bytecode.len());
    /// ```
    pub async fn get_contract_info(&self, address: Address) -> Result<ContractInfo, EvmError> {
        let bytecode = self.get_contract_bytecode(address).await?;
        let is_contract = !bytecode.is_empty();
        let deployed_bytecode = self.get_deployed_bytecode(address).await?;
        let (creation_block, creation_tx_hash) = self.find_creation_info(address).await?;
        let storage_slots = self.sample_storage_slots(address, 100).await?;
        Ok(ContractInfo {
            address,
            bytecode,
            deployed_bytecode,
            is_contract,
            creation_block,
            creation_tx_hash,
            storage_slots,
        })
    }

    /// Retrieves contract bytecode from the blockchain
    ///
    /// # Example
    /// ```rust
    /// let bytecode = analyzer.get_contract_bytecode(address).await?;
    /// println!("Bytecode length: {} bytes", bytecode.len());
    /// ```
    pub async fn get_contract_bytecode(&self, address: Address) -> Result<Bytes, EvmError> {
        self.evm
            .client
            .provider
            .get_code(address, None)
            .await
            .map_err(|e| EvmError::RpcError(format!("Failed to get contract bytecode: {}", e)))
    }

    /// Retrieves deployed bytecode (runtime bytecode)
    pub async fn get_deployed_bytecode(&self, address: Address) -> Result<Bytes, EvmError> {
        self.get_contract_bytecode(address).await
    }

    /// Finds contract creation block and transaction hash
    async fn find_creation_info(
        &self,
        address: Address,
    ) -> Result<(Option<u64>, Option<H256>), EvmError> {
        let current_block = self.evm.get_block_number().await?;
        let start_block = current_block.saturating_sub(1000);
        for block_number in (start_block..=current_block).rev() {
            if let Some(block) = self
                .evm
                .client
                .provider
                .get_block(block_number)
                .await
                .map_err(|e| {
                    EvmError::RpcError(format!("Failed to get block {}: {}", block_number, e))
                })?
            {
                if let transactions = block.transactions {
                    for tx_hash in transactions {
                        if let Some(receipt) = self.evm.get_transaction_receipt(tx_hash).await? {
                            if let Some(contract_address) = receipt.contract_address {
                                if contract_address == address {
                                    return Ok((Some(block_number), Some(tx_hash)));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok((None, None))
    }

    /// Samples storage slots for analysis
    async fn sample_storage_slots(
        &self,
        address: Address,
        sample_count: usize,
    ) -> Result<HashMap<H256, H256>, EvmError> {
        let mut slots = HashMap::new();
        for i in 0..sample_count {
            let slot = H256::from_low_u64_be(i as u64);
            if let Some(value) = self.get_storage_at(address, slot).await? {
                slots.insert(slot, value);
            }
        }
        Ok(slots)
    }

    /// Retrieves storage value at specific slot
    ///
    /// # Example
    /// ```rust
    /// let slot = H256::zero();
    /// let value = analyzer.get_storage_at(address, slot).await?;
    /// if let Some(storage_value) = value {
    ///     println!("Storage value: {:?}", storage_value);
    /// }
    /// ```
    pub async fn get_storage_at(
        &self,
        address: Address,
        slot: H256,
    ) -> Result<Option<H256>, EvmError> {
        self.evm
            .client
            .provider
            .get_storage_at(address, slot, None)
            .await
            .map(Some)
            .map_err(|e| {
                EvmError::RpcError(format!("Failed to get storage at slot {:?}: {}", slot, e))
            })
    }

    /// Analyzes storage layout of a contract
    ///
    /// # Example
    /// ```rust
    /// let layout = analyzer.analyze_storage_layout(address).await?;
    /// println!("Total storage size: {} bytes", layout.total_size);
    /// for slot in layout.slots {
    ///     println!("Slot {:?}: value {:?}, size {}", slot.slot, slot.value, slot.size);
    /// }
    /// ```
    pub async fn analyze_storage_layout(
        &self,
        address: Address,
    ) -> Result<StorageLayout, EvmError> {
        let mut slots = Vec::new();
        let mut total_size = 0;
        for i in 0..50 {
            let slot = H256::from_low_u64_be(i as u64);
            if let Some(value) = self.get_storage_at(address, slot).await? {
                let size = self.calculate_storage_size(value);
                total_size += size;

                slots.push(StorageSlot { slot, value, size });
            }
        }
        Ok(StorageLayout { slots, total_size })
    }

    /// Calculates approximate storage size based on non-zero bytes
    fn calculate_storage_size(&self, value: H256) -> usize {
        // 简单的启发式方法：计算非零字节的数量
        value.as_bytes().iter().filter(|&&b| b != 0).count()
    }

    /// Extracts potential function selectors from bytecode
    ///
    /// # Example
    /// ```rust
    /// let bytecode = analyzer.get_contract_bytecode(address).await?;
    /// let selectors = analyzer.extract_function_selectors(&bytecode);
    /// println!("Found {} potential function selectors", selectors.len());
    /// for selector in selectors {
    ///     println!("Selector: {:?}", selector);
    /// }
    /// ```
    pub fn extract_function_selectors(&self, bytecode: &Bytes) -> Vec<H256> {
        let mut selectors = Vec::new();
        let code = bytecode.as_ref();
        for i in 0..code.len().saturating_sub(4) {
            if i > 0 && code[i - 1] == 0x63 {
                let selector_bytes = [code[i], code[i + 1], code[i + 2], code[i + 3]];
                let selector = H256::from_slice(&{
                    let mut full = [0u8; 32];
                    full[28..32].copy_from_slice(&selector_bytes);
                    full
                });
                selectors.push(selector);
            }
        }
        selectors.dedup();
        selectors
    }

    /// Analyzes bytecode features and characteristics
    ///
    /// # Example
    /// ```rust
    /// let features = analyzer.analyze_bytecode_features(address).await?;
    /// println!("Is proxy: {}", features.is_proxy);
    /// println!("Has selfdestruct: {}", features.has_selfdestruct);
    /// println!("Bytecode length: {}", features.bytecode_length);
    /// ```
    pub async fn analyze_bytecode_features(
        &self,
        address: Address,
    ) -> Result<BytecodeFeatures, EvmError> {
        let bytecode = self.get_contract_bytecode(address).await?;
        let function_selectors = self.extract_function_selectors(&bytecode);
        let is_proxy = self.detect_proxy_pattern(&bytecode).await;
        let has_selfdestruct = bytecode.contains(&0xff); // SELFDESTRUCT opcode
        let has_delegatecall = bytecode.contains(&0xf4); // DELEGATECALL opcode
        Ok(BytecodeFeatures {
            address,
            bytecode_length: bytecode.len(),
            function_selectors,
            is_proxy,
            has_selfdestruct,
            has_delegatecall,
            opcode_distribution: self.analyze_opcode_distribution(&bytecode),
        })
    }

    /// Detects proxy contract patterns in bytecode
    async fn detect_proxy_pattern(&self, bytecode: &Bytes) -> bool {
        let code = bytecode.as_ref();
        let has_delegatecall = code.contains(&0xf4);
        has_delegatecall
    }

    /// Analyzes opcode distribution in bytecode
    fn analyze_opcode_distribution(&self, bytecode: &Bytes) -> HashMap<u8, usize> {
        let mut distribution = HashMap::new();
        for &opcode in bytecode.as_ref() {
            *distribution.entry(opcode).or_insert(0) += 1;
        }
        distribution
    }

    /// Compares two contracts for similarity
    ///
    /// # Example
    /// ```rust
    /// async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let similarity = analyzer.compare_contracts(address1, address2).await?;
    /// println!("Bytecode similarity: {:.2}%", similarity.bytecode_similarity * 100.0);
    /// println!("Common function selectors: {}", similarity.common_function_selectors.len());
    /// Ok(())
    /// }
    /// ```
    pub async fn compare_contracts(
        &self,
        address1: Address,
        address2: Address,
    ) -> Result<ContractSimilarity, EvmError> {
        let bytecode1 = self.get_contract_bytecode(address1).await?;
        let bytecode2 = self.get_contract_bytecode(address2).await?;
        let similarity = self.calculate_bytecode_similarity(&bytecode1, &bytecode2);
        let selectors1 = self.extract_function_selectors(&bytecode1);
        let selectors2 = self.extract_function_selectors(&bytecode2);
        let common_selectors: Vec<H256> = selectors1
            .iter()
            .filter(|s| selectors2.contains(s))
            .cloned()
            .collect();
        Ok(ContractSimilarity {
            address1,
            address2,
            bytecode_similarity: similarity,
            common_function_selectors: common_selectors,
            bytecode1_length: bytecode1.len(),
            bytecode2_length: bytecode2.len(),
        })
    }

    /// Calculates similarity between two bytecodes
    fn calculate_bytecode_similarity(&self, bytecode1: &Bytes, bytecode2: &Bytes) -> f64 {
        if bytecode1.is_empty() && bytecode2.is_empty() {
            return 1.0;
        }
        if bytecode1.is_empty() || bytecode2.is_empty() {
            return 0.0;
        }
        let len1 = bytecode1.len();
        let len2 = bytecode2.len();
        let max_len = len1.max(len2) as f64;
        if max_len == 0.0 {
            return 1.0;
        }
        let common_prefix = bytecode1
            .iter()
            .zip(bytecode2.iter())
            .take_while(|(a, b)| a == b)
            .count();
        common_prefix as f64 / max_len
    }

    /// Retrieves transaction statistics for a contract
    ///
    /// # Example
    /// ```rust
    /// let stats = analyzer.get_transaction_stats(address).await?;
    /// println!("Total transactions: {}", stats.total_transactions);
    /// println!("First seen block: {}", stats.first_seen_block);
    /// println!("Last seen block: {}", stats.last_seen_block);
    /// ```
    pub async fn get_transaction_stats(
        &self,
        address: Address,
    ) -> Result<TransactionStats, EvmError> {
        let current_block = self.evm.get_block_number().await?;
        let start_block = current_block.saturating_sub(10000);
        let mut total_txs = 0;
        let mut incoming_txs = 0;
        let mut outgoing_txs = 0;
        let filter = ethers::types::Filter::new()
            .from_block(start_block)
            .to_block(current_block)
            .address(address);
        let logs = self.evm.get_logs(filter).await?;
        total_txs = logs.len();
        Ok(TransactionStats {
            address,
            total_transactions: total_txs,
            incoming_transactions: incoming_txs,
            outgoing_transactions: outgoing_txs,
            first_seen_block: start_block,
            last_seen_block: current_block,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeFeatures {
    pub address: Address,
    pub bytecode_length: usize,
    pub function_selectors: Vec<H256>,
    pub is_proxy: bool,
    pub has_selfdestruct: bool,
    pub has_delegatecall: bool,
    pub opcode_distribution: HashMap<u8, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSimilarity {
    pub address1: Address,
    pub address2: Address,
    pub bytecode_similarity: f64,
    pub common_function_selectors: Vec<H256>,
    pub bytecode1_length: usize,
    pub bytecode2_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    pub address: Address,
    pub total_transactions: usize,
    pub incoming_transactions: usize,
    pub outgoing_transactions: usize,
    pub first_seen_block: u64,
    pub last_seen_block: u64,
}

impl From<ethers::providers::ProviderError> for EvmError {
    fn from(error: ethers::providers::ProviderError) -> Self {
        EvmError::RpcError(format!("Provider error: {}", error))
    }
}
