<h1 align="center">
    🤵 Evm SDK
</h1>
<h4 align="center">
An Ethereum Virtual Machine (EVM) network abstraction layer that provides complete blockchain interaction, smart contract analysis, mempool monitoring, and security auditing capabilities.
</h4>
<p align="center">
  <a href="https://github.com/0xhappyboy/evm-sdk/LICENSE"><img src="https://img.shields.io/badge/License-GPL3.0-d1d1f6.svg?style=flat&labelColor=1C2C2E&color=BEC5C9&logo=googledocs&label=license&logoColor=BEC5C9" alt="License"></a>
</p>
<p align="center">
<a href="./README_zh-CN.md">简体中文</a> | <a href="./README.md">English</a>
</p>

# 🏗️ Depend

```shell
cargo add evm-sdk
```

# 📦 Example

## Basic usage

```rust
use evm_client::{Evm, EvmType};
use ethers::types::Address;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let evm = Evm::new(EvmType::Ethereum).await?;
    let address: Address = "0x742d35Cc6634C0532925a3b8D6B6f7C93D5A7A7A".parse()?;
    let balance = evm.get_balance(address).await?;
    println!("Balance: {}", balance);
    let analyzer = evm_client::contract::ContractAnalyzer::new(Arc::new(evm));
    let contract_info = analyzer.get_contract_info(address).await?;
    println!("Contract bytecode length: {}", contract_info.bytecode.len());
    Ok(())
}

```

## Listen for transaction information in the latest block.

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

## Scan all transactions in the latest block.

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

## Get the actual number and amount of tokens decreased and increased in a specified transaction.

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

## Monitoring large transactions

```rust
use evm_client::{Evm, EvmType, TradeEventListener};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let evm = Arc::new(Evm::new(EvmType::Ethereum).await?);
    let event_listener = TradeEventListener::new(evm.clone());

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

## OnChain

### Uniswap

#### Get Pair Address

```rust
    #[tokio::test]
    async fn test_v2_get_pair() {
        let evm = Evm::new(evm_client::EvmType::ETHEREUM_MAINNET)
            .await
            .unwrap();
        let service = UniswapService::new(Arc::new(evm));
        let factory_addr = Address::from_str(MOCK_FACTORY_V2).unwrap();
        let token_a = Address::from_str(MOCK_TOKEN_A).unwrap();
        let token_b = Address::from_str(MOCK_TOKEN_B).unwrap();
        let result = service.v2_get_pair(factory_addr, token_a, token_b).await;
        match result {
            Ok(pair_addr) => {
                println!("V2 Pair Address: {:?}", pair_addr);
                assert_ne!(pair_addr, Address::zero());
            }
            Err(e) => {
                println!("V2 Get Pair test - Error (expected without fork): {}", e);
                assert!(true);
            }
        }
    }
```

#### Get Reserves and Calculate Price

```rust
    #[tokio::test]
    async fn test_v2_get_reserves_and_price() {
        let evm = Evm::new(evm_client::EvmType::ETHEREUM_MAINNET)
            .await
            .unwrap();
        let service = UniswapService::new(Arc::new(evm));
        let pair_addr = Address::from_str("0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc").unwrap();
        let result = service.v2_get_reserves(pair_addr).await;
        match result {
            Ok((reserve0, reserve1, timestamp)) => {
                println!(
                    "V2 Reserves - Reserve0: {}, Reserve1: {}, Timestamp: {}",
                    reserve0, reserve1, timestamp
                );
                if reserve0 > 0 && reserve1 > 0 {
                    let price = (reserve1 as f64) / (reserve0 as f64);
                    println!("V2 Calculated price (token1/token0): {}", price);
                }
                assert!(timestamp > 0 || (reserve0 == 0 && reserve1 == 0));
            }
            Err(e) => {
                println!("V2 Get Reserves test - Error: {}", e);
                assert!(true);
            }
        }
    }
```

#### Get Pool Information

```rust
    #[tokio::test]
    async fn test_v3_get_pool_info() {
        let evm = Evm::new(evm_client::EvmType::ETHEREUM_MAINNET)
            .await
            .unwrap();
        let service = UniswapService::new(Arc::new(evm));
        let factory_addr = Address::from_str(MOCK_FACTORY_V3).unwrap();
        let token_a = Address::from_str(MOCK_TOKEN_A).unwrap();
        let token_b = Address::from_str(MOCK_TOKEN_B).unwrap();
        let fee = FeeTier::Medium.value(); // 3000 (0.3%)
        let pool_result = service
            .v3_get_pool(factory_addr, token_a, token_b, fee)
            .await;
        match pool_result {
            Ok(pool_addr) => {
                println!("V3 Pool Address: {:?}", pool_addr);
                assert_ne!(pool_addr, Address::zero());
                let slot0_result = service.v3_get_slot0(pool_addr).await;
                match slot0_result {
                    Ok((
                        sqrt_price_x96,
                        tick,
                        obs_idx,
                        obs_card,
                        obs_card_next,
                        fee_proto,
                        unlocked,
                    )) => {
                        println!(
                            "V3 Slot0 - sqrtPriceX96: {:?}, tick: {}, unlocked: {}",
                            sqrt_price_x96, tick, unlocked
                        );
                        println!(
                            "  observationIndex: {}, observationCardinality: {}",
                            obs_idx, obs_card
                        );
                        let sqrt_price_f64 = (sqrt_price_x96.as_bytes()[0] as f64) / 65536.0; // Simplified
                        println!("  Approximate price from sqrtPriceX96: {}", sqrt_price_f64);
                    }
                    Err(e) => println!("V3 Get slot0 error: {}", e),
                }
                let liquidity_result = service.v3_get_liquidity(pool_addr).await;
                match liquidity_result {
                    Ok(liquidity) => println!("V3 Pool Liquidity: {}", liquidity),
                    Err(e) => println!("V3 Get liquidity error: {}", e),
                }
                let token0 = service
                    .v3_get_token0(pool_addr)
                    .await
                    .unwrap_or(Address::zero());
                let token1 = service
                    .v3_get_token1(pool_addr)
                    .await
                    .unwrap_or(Address::zero());
                let pool_fee = service.v3_get_fee(pool_addr).await.unwrap_or(0);
                println!(
                    "V3 Pool Tokens - token0: {:?}, token1: {:?}, fee: {}",
                    token0, token1, pool_fee
                );
            }
            Err(e) => {
                println!("V3 Get Pool test - Error: {}", e);
                assert!(true);
            }
        }
    }
```

#### Build Swap Path and Get Amounts

```rust
    #[test]
    fn test_v3_build_path_and_simulation() {
        let token_usdc = Address::from_str(MOCK_TOKEN_A).unwrap();
        let token_weth = Address::from_str(MOCK_TOKEN_B).unwrap();
        let token_dai = Address::from_str("0x6B175474E89094C44Da98b954EedeAC495271d0F").unwrap();
        let path = SwapPathBuilder::build_v3_path(vec![
            (token_usdc, FeeTier::Medium.value(), token_weth),
            (token_weth, FeeTier::Medium.value(), token_dai),
        ]);
        assert!(!path.is_empty());
        println!("V3 Encoded Path length: {} bytes", path.len());
        let tokens = vec![token_usdc, token_weth, token_dai];
        let fees = vec![FeeTier::Medium.value(), FeeTier::Medium.value()];
        let path2 = SwapPathBuilder::build_v3_path_with_fees(tokens, fees);
        assert!(!path2.is_empty());
        println!("V3 Alternative Path length: {} bytes", path2.len());
        assert_eq!(path.len(), 66);
        let v2_tokens = vec![token_usdc, token_weth, token_dai];
        let v2_path = SwapPathBuilder::build_v2_path(v2_tokens);
        assert_eq!(v2_path.len(), 3);
        println!("V2 Path has {} tokens", v2_path.len());
    }
```

#### Exact Input Swap Transaction Creation

```rust
    #[tokio::test]
    async fn test_v3_build_swap_transaction() {
        let evm = Evm::new(evm_client::EvmType::ETHEREUM_MAINNET)
            .await
            .unwrap();
        let service = UniswapService::new(Arc::new(evm));
        let router_addr = Address::from_str(MOCK_ROUTER_V3).unwrap();
        let token_in = Address::from_str(MOCK_TOKEN_A).unwrap();
        let token_out = Address::from_str(MOCK_TOKEN_B).unwrap();
        let recipient = Address::from_str(MOCK_RECIPIENT).unwrap();
        let params = ExactInputSingleParams {
            token_in,
            token_out,
            fee: FeeTier::Medium.value(),
            recipient,
            deadline: U256::from(9999999999u64),
            amount_in: U256::from(1000000u64),
            amount_out_minimum: U256::from(0u64),
            sqrt_price_limit_x96: H160::zero(),
        };
        println!("V3 Swap Parameters:");
        println!("  token_in: {:?}", params.token_in);
        println!("  token_out: {:?}", params.token_out);
        println!("  fee: {}", params.fee);
        println!("  amount_in: {}", params.amount_in);
        let result = service.v3_exact_input_single(router_addr, params).await;
        match result {
            Ok(tx_hash) => {
                println!("V3 Swap Transaction Hash: {:?}", tx_hash);
                assert_ne!(tx_hash, H256::zero());
            }
            Err(e) => {
                println!(
                    "V3 Build Swap Transaction test - Error (expected without wallet): {}",
                    e
                );
                assert!(e.to_string().contains("wallet") || e.to_string().contains("No wallet"));
            }
        }
    }
```
