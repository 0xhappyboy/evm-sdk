use crate::uniswap::{
    abi::{IERC20, IUniswapV3Pool},
    types::*,
};
use ethers::{prelude::*, types::Filter};
use std::sync::Arc;

pub struct UniswapV3 {
    pub provider: Arc<Provider<Http>>,
}

impl UniswapV3 {
    pub fn new(provider_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(provider_url)
            .map_err(|e| {
                println!("provider connection error:{:?}", e);
            })
            .unwrap();
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    /// get price by token0 token1
    pub async fn get_price_by_token0_token1(
        &self,
        pool_address: Address,
        token_a: &TokenInfo,
        token_b: &TokenInfo,
    ) -> Result<f64> {
        let pool_contract = IUniswapV3Pool::new(pool_address, self.provider.clone());
        let (sqrt_price_x96, _, _, _, _, _, _) = pool_contract
            .slot_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Slot0 error: {}", e)))?;
        // UniswapV3 Price Calculation Formula：price = (sqrtPriceX96 / 2^96)^2
        let sqrt_price = sqrt_price_x96.as_u128() as f64;
        let price_ratio = (sqrt_price / 2.0_f64.powi(96)).powi(2);
        let token0 = pool_contract
            .token_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token0 error: {}", e)))?;
        let price = if token0 == token_a.address {
            price_ratio
        } else {
            1.0 / price_ratio
        };
        let decimals_diff = token_b.decimals as i32 - token_a.decimals as i32;
        let adjusted_price = price * 10.0_f64.powi(decimals_diff);
        Ok(adjusted_price)
    }

    /// get liquid pool info
    pub async fn get_pool_info(&self, pool_address: Address) -> Result<PoolInfo> {
        let pool_contract = IUniswapV3Pool::new(pool_address, self.provider.clone());
        let token0_addr = pool_contract
            .token_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token0 error: {}", e)))?;
        let token1_addr = pool_contract
            .token_1()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token1 error: {}", e)))?;
        let liquidity = pool_contract
            .liquidity()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Liquidity error: {}", e)))?;
        let (sqrt_price_x96, _, _, _, _, _, _) = pool_contract
            .slot_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Slot0 error: {}", e)))?;
        // price
        let price = (sqrt_price_x96.as_u128() as f64).powf(2.0) / (2.0_f64.powf(192.0));
        let token0 = self.get_token_info(token0_addr).await?;
        let token1 = self.get_token_info(token1_addr).await?;
        Ok(PoolInfo {
            address: pool_address,
            token0,
            token1,
            reserve0: U256::zero(),
            reserve1: U256::zero(),
            liquidity: liquidity.into(),
        })
    }

    /// get token info
    pub async fn get_token_info(&self, address: Address) -> Result<TokenInfo> {
        let contract = IERC20::new(address, self.provider.clone());
        let symbol = contract
            .symbol()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Symbol error: {}", e)))?;
        let name = contract
            .name()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Name error: {}", e)))?;
        let decimals = contract
            .decimals()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Decimals error: {}", e)))?;
        Ok(TokenInfo {
            address,
            symbol,
            name,
            decimals,
        })
    }

    /// get price
    pub async fn get_price(&self, pool_address: Address) -> Result<PriceData> {
        let pool_contract = IUniswapV3Pool::new(pool_address, self.provider.clone());
        let token0 = pool_contract
            .token_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token0 error: {}", e)))?;
        let token1 = pool_contract
            .token_1()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token1 error: {}", e)))?;
        let (sqrt_price_x96, _, _, _, _, _, _) = pool_contract
            .slot_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Slot0 error: {}", e)))?;
        let price = (sqrt_price_x96.as_u128() as f64).powf(2.0) / (2.0_f64.powf(192.0));
        let liquidity = pool_contract
            .liquidity()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Liquidity error: {}", e)))?;
        Ok(PriceData {
            token0,
            token1,
            price,
            liquidity: liquidity.into(),
        })
    }
}
