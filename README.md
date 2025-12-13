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
