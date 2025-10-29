use crate::{EvmClient, EvmError};
use ethers::{
    contract::abigen,
    providers::Provider,
    types::{Address, H256, U256},
};
use std::sync::Arc;

abigen!(
    IERC20,
    r#"[
        function totalSupply() external view returns (uint256)
        function balanceOf(address account) external view returns (uint256)
        function transfer(address to, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transferFrom(address from, address to, uint256 amount) external returns (bool)
        event Transfer(address indexed from, address indexed to, uint256 value)
        event Approval(address indexed owner, address indexed spender, uint256 value)
    ]"#
);

/// ERC20 Service for interacting with ERC20 tokens
pub struct ERC20Service {
    client: Arc<EvmClient>,
}

impl ERC20Service {
    pub fn new(client: Arc<EvmClient>) -> Self {
        Self { client }
    }

    /// Create ERC20 token instance
    fn erc20(&self, token_address: Address) -> IERC20<Provider<ethers::providers::Http>> {
        IERC20::new(token_address, self.client.provider.clone())
    }

    /// Get ERC20 token balance
    pub async fn get_balance(
        &self,
        token_address: Address,
        owner: Address,
    ) -> Result<U256, EvmError> {
        let erc20 = self.erc20(token_address);
        erc20
            .balance_of(owner)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get ERC20 balance: {}", e)))
    }

    /// Get ERC20 token total supply
    pub async fn get_total_supply(&self, token_address: Address) -> Result<U256, EvmError> {
        let erc20 = self.erc20(token_address);
        erc20.total_supply().call().await.map_err(|e| {
            EvmError::ContractError(format!("Failed to get ERC20 total supply: {}", e))
        })
    }

    /// Transfer ERC20 tokens
    pub async fn transfer(
        &self,
        token_address: Address,
        to: Address,
        amount: U256,
    ) -> Result<H256, EvmError> {
        if self.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let erc20 = self.erc20(token_address);
        let tx = erc20.transfer(to, amount);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to transfer ERC20: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// Get ERC20 token allowance
    pub async fn get_allowance(
        &self,
        token_address: Address,
        owner: Address,
        spender: Address,
    ) -> Result<U256, EvmError> {
        let erc20 = self.erc20(token_address);
        erc20
            .allowance(owner, spender)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get allowance: {}", e)))
    }

    /// Approve spender to spend tokens
    pub async fn approve(
        &self,
        token_address: Address,
        spender: Address,
        amount: U256,
    ) -> Result<H256, EvmError> {
        if self.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let erc20 = self.erc20(token_address);
        let tx = erc20.approve(spender, amount);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to approve: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// Transfer from (requires allowance)
    pub async fn transfer_from(
        &self,
        token_address: Address,
        from: Address,
        to: Address,
        amount: U256,
    ) -> Result<H256, EvmError> {
        if self.client.wallet.is_none() {
            return Err(EvmError::WalletError("No wallet configured".to_string()));
        }
        let erc20 = self.erc20(token_address);
        let tx = erc20.transfer_from(from, to, amount);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to transfer from: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }
}

/// ERC20 Token Metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ERCTokenMetadata {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}
