use crate::uniswap::{
    abi::{IERC20, IUniswapV2Pair},
    types::*,
};
use ethers::{prelude::*, types::Filter};
use std::sync::Arc;

pub struct UniswapV2 {
    pub provider: Arc<Provider<Http>>,
}

impl UniswapV2 {
    pub fn new(provider_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(provider_url)
            .map_err(|e| {
                println!("error:{:?}", e);
            })
            .unwrap();
        Ok(Self {
            provider: Arc::new(provider),
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

    /// get liquid pool info
    pub async fn get_pool_info(&self, pool_address: Address) -> Result<PoolInfo> {
        let pair_contract = IUniswapV2Pair::new(pool_address, self.provider.clone());
        let token0_addr = pair_contract
            .token_0()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token0 error: {}", e)))?;
        let token1_addr = pair_contract
            .token_1()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Token1 error: {}", e)))?;
        let (reserve0, reserve1, _) = pair_contract
            .get_reserves()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Reserves error: {}", e)))?;
        let liquidity = pair_contract
            .total_supply()
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Total supply error: {}", e)))?;
        let token0 = self.get_token_info(token0_addr).await?;
        let token1 = self.get_token_info(token1_addr).await?;
        Ok(PoolInfo {
            address: pool_address,
            token0,
            token1,
            reserve0: reserve0.into(),
            reserve1: reserve1.into(),
            liquidity,
        })
    }

    /// get price
    pub async fn get_price(&self, pool_address: Address, base_token: Address) -> Result<PriceData> {
        let pool_info = self.get_pool_info(pool_address).await?;
        let (reserve_base, reserve_quote) = if pool_info.token0.address == base_token {
            (pool_info.reserve0, pool_info.reserve1)
        } else if pool_info.token1.address == base_token {
            (pool_info.reserve1, pool_info.reserve0)
        } else {
            return Err(UniswapError::PoolNotFound);
        };
        let price = if !reserve_base.is_zero() {
            reserve_quote.as_u128() as f64 / reserve_base.as_u128() as f64
        } else {
            0.0
        };
        Ok(PriceData {
            token0: pool_info.token0.address,
            token1: pool_info.token1.address,
            price,
            liquidity: pool_info.liquidity,
        })
    }

    /// monitor the latest transactions
    pub async fn listen_swaps(&self, pool_address: Address) -> Result<()> {
        let filter = Filter::new()
            .address(pool_address)
            .event("Swap(address,uint256,uint256,uint256,uint256,address)");
        let mut stream = self
            .provider
            .watch(&filter)
            .await
            .map_err(|e| UniswapError::EthersError(e))?;
        println!("Listening for swaps on pool: {:?}", pool_address);
        while let Some(log) = stream.next().await {
            println!("New swap detected: {:?}", log);
        }
        Ok(())
    }
}
