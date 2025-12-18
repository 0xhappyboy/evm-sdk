<h1 align="center">
   🤵 Evm SDK
</h1>
<h4 align="center">
一个以太坊虚拟机（EVM）网络抽象层，提供完整的区块链交互、智能合约分析、内存池监控和安全审计功能。
</h4>
<p align="center">
  <a href="https://github.com/0xhappyboy/evm-sdk/LICENSE"><img src="https://img.shields.io/badge/License-GPL3.0-d1d1f6.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=license&logoColor=BEC5C9" alt="License"></a>
</p>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

# 🏗️ 依赖

```shell
cargo add evm-sdk
```

# 📦 例子

## 基础用法

```rust
use evm_client::{Evm, EvmType};
use ethers::types::Address;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 EVM 客户端
    let evm = Evm::new(EvmType::Ethereum).await?;
    // 查询余额
    let address: Address = "0x742d35Cc6634C0532925a3b8D6B6f7C93D5A7A7A".parse()?;
    let balance = evm.get_balance(address).await?;
    println!("Balance: {}", balance);
    // 分析合约
    let analyzer = evm_client::contract::ContractAnalyzer::new(Arc::new(evm));
    let contract_info = analyzer.get_contract_info(address).await?;
    println!("Contract bytecode length: {}", contract_info.bytecode.len());
    Ok(())
}

```

## 监听最新区块中的交易信息.

```rust
#[cfg(test)]
mod tests {
    use crate::trade::{self, Trade};

    use super::*;
    use ethers::types::H256;
    use evm_client::EvmType;
    use std::sync::Arc;

    #[tokio::test]
    async fn lisent_liquidity_last_transaction() {
        let evm = Arc::new(Evm::new(EvmType::ETHEREUM_MAINNET).await.unwrap());
        let trade = Trade::new(evm.clone());
        let block_service = evm.clone().get_block_service();
        loop {
            match block_service.get_latest_block().await {
                Ok(Some(block)) => {
                    for hash in block.transaction_hashes.unwrap() {
                        let trade = trade
                            .get_transactions_by_tx(&format!("{:?}", hash))
                            .await
                            .unwrap();
                        println!("transaction hash: {:?}", trade.hash);
                        println!("dex: {:?}", trade.get_dex_names());
                        println!(
                            "liquidity pool address: {:?}",
                            trade.get_liquidity_pool_addresses()
                        );
                        println!("received: {:?}", trade.get_received_token_eth());
                        println!("spent: {:?}", trade.get_spent_token_eth());
                        println!("direction: {:?}", trade.getDirection());
                    }
                }
                Ok(None) => println!("⚠️ Nont Block"),
                Err(e) => println!("❌ Error: {}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(
                evm.clone().client.get_block_interval_time().unwrap(),
            ))
            .await;
        }
    }
}
```

## 扫描最新区块中的所有交易.

```rust
#[cfg(test)]
mod tests {
    use crate::trade::{self, Trade};

    use super::*;
    use ethers::types::H256;
    use evm_client::EvmType;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_poll_latest_block_per_second() {
        let evm = Arc::new(Evm::new(EvmType::ETHEREUM_MAINNET).await.unwrap());
        let trade = Trade::new(evm.clone());
        let block_service = evm.clone().get_block_service();
        for i in 0..5 {
            match block_service.get_latest_block().await {
                Ok(Some(block)) => {
                    println!("✅ {} seconds: block #{:?}", i, block.number);
                    println!("Block hash: {:?}", block.transaction_hashes);
                    for hash in block.transaction_hashes.unwrap() {
                        let trade = trade
                            .get_transactions_by_tx(&format!("{:?}", hash))
                            .await
                            .unwrap();
                        println!("All transactions in the block: {:?}", trade);
                    }
                }
                Ok(None) => println!("⚠️  {} s: Nont Block", i),
                Err(e) => println!("❌ {} s: Error: {}", i, e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(
                evm.clone().client.get_block_interval_time().unwrap(),
            ))
            .await;
        }
    }
}
```

## 获取指定交易中实际减少代币地址与数量和实际增加代币地址与数量.

```rust
#[cfg(test)]
mod test {
    use crate::{Evm, trade::Trade};
    use std::sync::Arc;
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
```

## 监控大额交易

```rust
use evm_client::{Evm, EvmType, TradeEventListener};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let evm = Arc::new(Evm::new(EvmType::Ethereum).await?);
    let event_listener = TradeEventListener::new(evm.clone());

    // 监控大于 1 ETH 的交易
    let min_value = ethers::types::U256::from(10u64.pow(18));
    let mut receiver = event_listener.watch_large_transactions(min_value, 3).await?;

    while let Some(tx_with_receipt) = receiver.recv().await {
        println!("Large transaction detected: {:?}", tx_with_receipt.transaction.hash);
        println!("Value: {}", tx_with_receipt.transaction.value);
        println!("From: {:?}", tx_with_receipt.transaction.from);
        if let Some(to) = tx_with_receipt.transaction.to {
            println!("To: {:?}", to);
        }
    }

    Ok(())
}
```
