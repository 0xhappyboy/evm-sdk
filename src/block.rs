use ethers::types::U256;
use ethers::types::{Block as EthersBlock, H64, H256, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{Evm, types::EvmError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block number
    pub number: Option<u64>,
    /// Block hash
    pub hash: Option<H256>,
    /// Parent hash
    pub parent_hash: H256,
    /// Block timestamp in seconds since epoch
    pub timestamp: U256,
    /// Block gas limit
    pub gas_limit: U256,
    /// Block gas used
    pub gas_used: U256,
    /// Miner address (author)
    pub miner: ethers::types::Address,
    /// Block difficulty
    pub difficulty: U256,
    /// Total difficulty
    pub total_difficulty: Option<U256>,
    /// Block size in bytes
    pub size: Option<U256>,
    /// Transaction count
    pub transaction_count: usize,
    /// Transaction hashes (only available in basic block)
    pub transaction_hashes: Option<Vec<H256>>,
    /// Full transactions (only available when requested with get_block_with_txs)
    pub transactions: Option<Vec<Transaction>>,
    /// Base fee per gas (EIP-1559)
    pub base_fee_per_gas: Option<U256>,
    /// Extra data
    pub extra_data: ethers::types::Bytes,
    /// Sha3 uncles
    pub sha3_uncles: H256,
    /// Logs bloom
    pub logs_bloom: Option<ethers::types::Bloom>,
    /// Receipts root
    pub receipts_root: H256,
    /// State root
    pub state_root: H256,
    /// Transactions root
    pub transactions_root: H256,
    /// Nonce
    pub nonce: Option<H64>,
    /// Mix hash
    pub mix_hash: Option<H256>,
    /// Uncles
    pub uncles: Vec<H256>,
}

impl BlockInfo {
    /// Convert from Ethers block with transaction hashes
    pub fn from_ethers_block(block: &EthersBlock<H256>) -> Self {
        Self {
            number: block.number.map(|n| n.as_u64()),
            hash: block.hash,
            parent_hash: block.parent_hash,
            timestamp: block.timestamp,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            miner: block.author.unwrap_or(ethers::types::Address::zero()),
            difficulty: block.difficulty,
            total_difficulty: block.total_difficulty,
            size: block.size,
            transaction_count: block.transactions.len(),
            transaction_hashes: Some(block.transactions.clone()),
            transactions: None,
            base_fee_per_gas: block.base_fee_per_gas,
            extra_data: block.extra_data.clone(),
            sha3_uncles: block.uncles_hash,
            logs_bloom: block.logs_bloom,
            receipts_root: block.receipts_root,
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            nonce: block.nonce,
            mix_hash: block.mix_hash,
            uncles: block.uncles.clone(),
        }
    }

    /// Convert from Ethers block with full transactions
    pub fn from_ethers_block_with_txs(block: &EthersBlock<Transaction>) -> Self {
        let transaction_hashes: Vec<H256> = block.transactions.iter().map(|tx| tx.hash).collect();

        Self {
            number: block.number.map(|n| n.as_u64()),
            hash: block.hash,
            parent_hash: block.parent_hash,
            timestamp: block.timestamp,
            gas_limit: block.gas_limit,
            gas_used: block.gas_used,
            miner: block.author.unwrap_or(ethers::types::Address::zero()),
            difficulty: block.difficulty,
            total_difficulty: block.total_difficulty,
            size: block.size,
            transaction_count: block.transactions.len(),
            transaction_hashes: Some(transaction_hashes),
            transactions: Some(block.transactions.clone()),
            base_fee_per_gas: block.base_fee_per_gas,
            extra_data: block.extra_data.clone(),
            sha3_uncles: block.uncles_hash,
            logs_bloom: block.logs_bloom,
            receipts_root: block.receipts_root,
            state_root: block.state_root,
            transactions_root: block.transactions_root,
            nonce: block.nonce,
            mix_hash: block.mix_hash,
            uncles: block.uncles.clone(),
        }
    }

    /// Get block timestamp as u64 (if it fits)
    pub fn timestamp_u64(&self) -> Option<u64> {
        self.timestamp.try_into().ok()
    }

    /// Get block number as u64
    pub fn number_u64(&self) -> Option<u64> {
        self.number
    }

    /// Get gas limit as u64
    pub fn gas_limit_u64(&self) -> Option<u64> {
        self.gas_limit.try_into().ok()
    }

    /// Get gas used as u64
    pub fn gas_used_u64(&self) -> Option<u64> {
        self.gas_used.try_into().ok()
    }

    /// Calculate gas used percentage
    pub fn gas_used_percentage(&self) -> Option<f64> {
        let gas_used: u128 = self.gas_used.try_into().ok()?;
        let gas_limit: u128 = self.gas_limit.try_into().ok()?;
        if gas_limit == 0 {
            return None;
        }
        Some((gas_used as f64 / gas_limit as f64) * 100.0)
    }
}

pub struct BlockService {
    evm: Arc<Evm>,
}

impl BlockService {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm }
    }

    /// Get block information by block number (with transaction hashes only)
    pub async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<Option<BlockInfo>, EvmError> {
        let block = self
            .evm
            .get_block_by_number(ethers::types::BlockNumber::Number(block_number.into()))
            .await?;
        Ok(block.map(|b| BlockInfo::from_ethers_block(&b)))
    }

    /// Get block information by block hash (with transaction hashes only)
    pub async fn get_block_by_hash(&self, block_hash: H256) -> Result<Option<BlockInfo>, EvmError> {
        let block = self.evm.get_block_by_hash(block_hash).await?;
        Ok(block.map(|b| BlockInfo::from_ethers_block(&b)))
    }

    /// Get block with full transaction details by block number
    pub async fn get_block_with_txs(
        &self,
        block_number: u64,
    ) -> Result<Option<BlockInfo>, EvmError> {
        let block = self
            .evm
            .get_block_with_txs(ethers::types::BlockNumber::Number(block_number.into()))
            .await?;
        Ok(block.map(|b| BlockInfo::from_ethers_block_with_txs(&b)))
    }

    /// Get block with full transaction details by block hash
    pub async fn get_block_with_txs_by_hash(
        &self,
        block_hash: H256,
    ) -> Result<Option<BlockInfo>, EvmError> {
        let block = self.get_block_by_hash(block_hash).await?;
        if let Some(block_info) = block {
            if let Some(block_number) = block_info.number {
                return self.get_block_with_txs(block_number).await;
            }
        }
        Ok(None)
    }

    /// Get latest block information (with transaction hashes only)
    pub async fn get_latest_block(&self) -> Result<Option<BlockInfo>, EvmError> {
        let block = self
            .evm
            .get_block_by_number(ethers::types::BlockNumber::Latest)
            .await?;
        Ok(block.map(|b| BlockInfo::from_ethers_block(&b)))
    }

    /// Get latest block with full transaction details
    pub async fn get_latest_block_with_txs(&self) -> Result<Option<BlockInfo>, EvmError> {
        let block = self
            .evm
            .get_block_with_txs(ethers::types::BlockNumber::Latest)
            .await?;
        Ok(block.map(|b| BlockInfo::from_ethers_block_with_txs(&b)))
    }

    /// Get multiple blocks in a range
    pub async fn get_blocks_in_range(
        &self,
        start: u64,
        end: u64,
    ) -> Result<Vec<Option<BlockInfo>>, EvmError> {
        let mut blocks = Vec::new();
        let mut futures = Vec::new();
        for block_number in start..=end {
            let service = self.evm.clone();
            futures.push(async move {
                service
                    .get_block_by_number(ethers::types::BlockNumber::Number(block_number.into()))
                    .await
                    .ok()
                    .flatten()
                    .map(|b| BlockInfo::from_ethers_block(&b))
            });
        }
        for future in futures {
            blocks.push(tokio::spawn(future).await.ok().flatten());
        }
        Ok(blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::H256;
    use evm_client::EvmType;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_get_block_by_number_success() {
        // Test: Get an existing block
        let evm = Evm::new(EvmType::ETHEREUM_MAINNET).await.unwrap();
        let evm_arc = Arc::new(evm);
        let block_service = evm_arc.get_block_service();
        // Get the latest block number
        let latest_block = block_service.get_latest_block().await;
        match latest_block {
            Ok(Some(block_info)) => {
                println!("✅ Successfully got latest block");
                println!("   Block number: {:?}", block_info.number);
                println!("   Block hash: {:?}", block_info.hash);
                println!("   Timestamp: {}", block_info.timestamp);
                println!("   Transaction count: {}", block_info.transaction_count);
                println!("   Gas used: {}", block_info.gas_used);
                // Verify basic fields are not empty
                assert!(
                    block_info.parent_hash != H256::zero(),
                    "Parent hash should not be zero"
                );
                assert!(
                    block_info.timestamp > U256::zero(),
                    "Timestamp should be greater than zero"
                );
                assert!(
                    block_info.miner != ethers::types::Address::zero(),
                    "Miner address should not be zero"
                );
                // Test conversion functions
                if let Some(timestamp_u64) = block_info.timestamp_u64() {
                    println!("   Timestamp(u64): {}", timestamp_u64);
                }

                if let Some(gas_percentage) = block_info.gas_used_percentage() {
                    println!("   Gas usage percentage: {:.2}%", gas_percentage);
                    assert!(
                        gas_percentage >= 0.0 && gas_percentage <= 100.0,
                        "Gas usage should be between 0-100%"
                    );
                }
            }
            Ok(None) => {
                println!("⚠️  Latest block is None (possible node issue)");
            }
            Err(e) => {
                println!("❌ Failed to get latest block: {}", e);
                // Skip test if network issue
                if e.to_string().contains("Rpc Error") || e.to_string().contains("timeout") {
                    println!("   Skipping test (network issue)");
                    return;
                }
                panic!("Failed to get latest block: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_block_by_hash_and_compare() {
        // Test: Get block by number, then by hash, and compare results
        let evm = Evm::new(EvmType::ETHEREUM_MAINNET).await.unwrap();
        let evm_arc = Arc::new(evm);
        let block_service = evm_arc.get_block_service();
        // 1. First get the latest block
        let latest_block_result = block_service.get_latest_block().await;
        if let Ok(Some(latest_block)) = latest_block_result {
            if let (Some(block_number), Some(block_hash)) = (latest_block.number, latest_block.hash)
            {
                println!("✅ Got latest block #{}", block_number);
                println!("   Hash: {:?}", block_hash);
                // 2. Get by block number
                let by_number_result = block_service.get_block_by_number(block_number).await;
                assert!(
                    by_number_result.is_ok(),
                    "Should succeed when getting by block number"
                );
                if let Ok(Some(by_number)) = by_number_result {
                    // 3. Get by block hash
                    let by_hash_result = block_service.get_block_by_hash(block_hash).await;
                    assert!(
                        by_hash_result.is_ok(),
                        "Should succeed when getting by block hash"
                    );
                    if let Ok(Some(by_hash)) = by_hash_result {
                        // Compare results
                        println!("   Comparing results from block number and block hash...");
                        // Verify basic fields are consistent
                        assert_eq!(
                            by_number.number, by_hash.number,
                            "Block numbers should match"
                        );
                        assert_eq!(by_number.hash, by_hash.hash, "Block hashes should match");
                        assert_eq!(
                            by_number.parent_hash, by_hash.parent_hash,
                            "Parent hashes should match"
                        );
                        assert_eq!(
                            by_number.timestamp, by_hash.timestamp,
                            "Timestamps should match"
                        );
                        assert_eq!(
                            by_number.transaction_count, by_hash.transaction_count,
                            "Transaction counts should match"
                        );
                        // Verify transaction hash lists are consistent
                        if let (Some(number_hashes), Some(hash_hashes)) =
                            (&by_number.transaction_hashes, &by_hash.transaction_hashes)
                        {
                            assert_eq!(
                                number_hashes.len(),
                                hash_hashes.len(),
                                "Transaction hash list lengths should match"
                            );

                            // If there are transactions, compare the first one
                            if !number_hashes.is_empty() {
                                assert_eq!(
                                    number_hashes[0], hash_hashes[0],
                                    "First transaction hash should match"
                                );
                            }
                        }
                        println!("   ✅ Results from block number and block hash match");
                        // Test getting block with transactions
                        let with_txs_result = block_service.get_block_with_txs(block_number).await;
                        match with_txs_result {
                            Ok(Some(block_with_txs)) => {
                                println!("   ✅ Successfully got block with transactions");
                                println!(
                                    "      Transaction count: {}",
                                    block_with_txs.transaction_count
                                );

                                // Verify transaction hash list exists
                                assert!(
                                    block_with_txs.transaction_hashes.is_some(),
                                    "Transaction hashes should be Some"
                                );

                                // If block contains full transactions
                                if let Some(transactions) = &block_with_txs.transactions {
                                    println!(
                                        "      Full transaction count: {}",
                                        transactions.len()
                                    );
                                    assert_eq!(
                                        transactions.len(),
                                        block_with_txs.transaction_count,
                                        "Transaction counts should match"
                                    );

                                    // Verify each transaction has a hash
                                    for tx in transactions {
                                        assert_ne!(
                                            tx.hash,
                                            H256::zero(),
                                            "Transaction hash should not be zero"
                                        );
                                    }
                                }
                            }
                            Ok(None) => {
                                println!(
                                    "   ⚠️  Block with transactions is None (node might not support)"
                                );
                            }
                            Err(e) => {
                                println!("   ⚠️  Failed to get block with transactions: {}", e);
                                // Skip if node doesn't support this
                            }
                        }
                    }
                }
            } else {
                println!("⚠️  Latest block missing number or hash");
            }
        } else {
            println!("⚠️  Cannot get latest block, skipping comparison test");
        }
    }

    #[tokio::test]
    async fn test_get_blocks_in_range() {
        // Test: Get blocks in a range (only when network is available)
        let evm = Evm::new(EvmType::ETHEREUM_MAINNET).await.unwrap();
        let evm_arc = Arc::new(evm);
        let block_service = evm_arc.get_block_service();
        // Get latest block number
        let latest_block = block_service.get_latest_block().await;
        if let Ok(Some(latest_block_info)) = latest_block {
            if let Some(latest_number) = latest_block_info.number {
                if latest_number >= 2 {
                    // Get last 2 blocks
                    let start = latest_number - 1;
                    let end = latest_number;
                    println!("✅ Testing get blocks in range {}-{}", start, end);
                    let blocks_result = block_service.get_blocks_in_range(start, end).await;
                    assert!(
                        blocks_result.is_ok(),
                        "Should succeed when getting blocks in range"
                    );
                    let blocks = blocks_result.unwrap();
                    assert_eq!(blocks.len(), 2, "Should return 2 blocks");
                    let mut found_blocks = 0;
                    for (i, block_opt) in blocks.iter().enumerate() {
                        let expected_number = start + i as u64;
                        match block_opt {
                            Some(block) => {
                                found_blocks += 1;
                                if let Some(block_number) = block.number {
                                    println!("   Block #{}: found", block_number);
                                    assert_eq!(
                                        block_number, expected_number,
                                        "Block number mismatch"
                                    );
                                }
                            }
                            None => {
                                println!(
                                    "   Block #{}: not found (possible node issue)",
                                    expected_number
                                );
                            }
                        }
                    }
                    println!("   Found {}/2 blocks", found_blocks);
                } else {
                    println!("⚠️  Chain too short for range test");
                }
            }
        } else {
            println!("⚠️  Cannot get latest block, skipping range test");
        }
    }
}
