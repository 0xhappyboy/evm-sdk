use crate::global::mainnet::{
    DAI_ADDRESS, MAINNET_UNISWAP_V2_FACTORY_ADDRESS, MAINNET_UNISWAP_V3_FACTORY_ADDRESS,
    USDC_ADDRESS, USDT_ADDRESS, WETH_ADDRESS,
};
use crate::tool::address::str_to_h160_1;
use crate::uniswap::abi::{IUniswapV2Factory, IUniswapV3Factory};
use crate::uniswap::types::*;
use crate::uniswap::types::{PoolInfo, TokenPriceInfo};
use crate::uniswap::v2::UniswapV2;
use crate::uniswap::v3::UniswapV3;
use ethers::prelude::*;
use ethers_providers::{Http, Provider};
use std::sync::Arc;

pub struct Price {
    provider: Arc<Provider<Http>>,
    v2_factory: Address,
    v3_factory: Address,
}

impl Price {
    pub fn new(provider: Arc<Provider<Http>>) -> Result<Self> {
        let v2_factory = MAINNET_UNISWAP_V2_FACTORY_ADDRESS.parse().unwrap();
        let v3_factory = MAINNET_UNISWAP_V3_FACTORY_ADDRESS.parse().unwrap();
        Ok(Self {
            provider: provider,
            v2_factory,
            v3_factory,
        })
    }

    /// get token prices by token address
    pub async fn get_token_prices_by_token_address(
        &self,
        token_address: Address,
    ) -> Result<TokenPriceInfo> {
        let token_info = self.get_token_info(token_address).await?;
        let mut price_info = TokenPriceInfo {
            token_address,
            token_symbol: token_info.symbol.clone(),
            token_name: token_info.name.clone(),
            decimals: token_info.decimals,
            eth_price: None,
            usd_price: None,
            usdc_price: None,
            usdt_price: None,
            dai_price: None,
            liquidity: U256::zero(),
            price_source: "Unknown".to_string(),
            last_updated: chrono::Utc::now().timestamp() as u64,
        };
        // get eth price
        if let Ok(eth_price) = self.get_eth_price_with_decimals(&token_info).await {
            price_info.eth_price = Some(eth_price);
            price_info.price_source = "Uniswap V2".to_string();
        } else if let Ok(eth_price) = self.get_eth_price_by_v3(&token_info).await {
            price_info.eth_price = Some(eth_price);
            price_info.price_source = "Uniswap V3".to_string();
        }
        // get stablecoin prices
        // usdc
        if let Ok(usdc_price) = self.get_stablecoin_price(&token_info, "USDC").await {
            price_info.usdc_price = Some(usdc_price);
        }
        // usdt
        if let Ok(usdt_price) = self.get_stablecoin_price(&token_info, "USDT").await {
            price_info.usdt_price = Some(usdt_price);
        }
        // dai
        if let Ok(dai_price) = self.get_stablecoin_price(&token_info, "DAI").await {
            price_info.dai_price = Some(dai_price);
        }
        // calculate USD price
        price_info.usd_price = self.calc_usd_price(&price_info).await;
        Ok(price_info)
    }

    /// get eth price from uniswap V2
    async fn get_eth_price_with_decimals(&self, token_info: &TokenInfo) -> Result<f64> {
        let weth_address = str_to_h160_1(WETH_ADDRESS).unwrap();
        let weth_info = self.get_token_info(weth_address).await.unwrap();
        let factory = IUniswapV2Factory::new(self.v2_factory, self.provider.clone());
        let pair_address = factory
            .get_pair(token_info.address, weth_address)
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Factory error: {}", e)))?;
        if pair_address == Address::zero() {
            return Err(UniswapError::PoolNotFound);
        }
        let uniswap_v2 = UniswapV2::new_with_provider(self.provider.clone());
        let pool_info = uniswap_v2.get_pool_info(pair_address).await?;
        let price = self.calc_price(&pool_info, token_info, &weth_info)?;
        Ok(price)
    }

    /// calculate price
    fn calc_price(
        &self,
        pool_info: &PoolInfo,
        token_a: &TokenInfo,
        token_b: &TokenInfo,
    ) -> Result<f64> {
        let (reserve_a, reserve_b) = if pool_info.token0.address == token_a.address {
            (pool_info.reserve0, pool_info.reserve1)
        } else if pool_info.token1.address == token_a.address {
            (pool_info.reserve1, pool_info.reserve0)
        } else {
            return Err(UniswapError::PoolNotFound);
        };
        let reserve_a_decimal = self.to_decimal(reserve_a, token_a.decimals);
        let reserve_b_decimal = self.to_decimal(reserve_b, token_b.decimals);
        if reserve_a_decimal == 0.0 {
            println!("decimal error");
        }
        let price = reserve_b_decimal / reserve_a_decimal;
        Ok(price)
    }
    fn to_decimal(&self, value: U256, decimals: u8) -> f64 {
        let value_str = value.to_string();
        let len = value_str.len();

        if decimals == 0 {
            return value.as_u128() as f64;
        }
        if len <= decimals as usize {
            let mut decimal_str = "0.".to_string();
            for _ in 0..(decimals as usize - len) {
                decimal_str.push('0');
            }
            decimal_str.push_str(&value_str);
            decimal_str.parse::<f64>().unwrap_or(0.0)
        } else {
            let split_index = len - decimals as usize;
            let integer_part = &value_str[0..split_index];
            let fractional_part = &value_str[split_index..];

            let mut decimal_str = integer_part.to_string();
            decimal_str.push('.');
            decimal_str.push_str(fractional_part);
            decimal_str.parse::<f64>().unwrap_or(0.0)
        }
    }

    /// get het price by uniswap v3
    async fn get_eth_price_by_v3(&self, token_info: &TokenInfo) -> Result<f64> {
        let weth_address = str_to_h160_1(WETH_ADDRESS).unwrap();
        let weth_info = self.get_token_info(weth_address).await?;
        let fees = [500, 3000, 10000];
        for fee in fees.iter() {
            let factory = IUniswapV3Factory::new(self.v3_factory, self.provider.clone());
            let pool_address = factory
                .get_pool(token_info.address, weth_address, *fee)
                .call()
                .await
                .map_err(|e| UniswapError::ContractError(format!("V3 Factory error: {}", e)))?;

            if pool_address != Address::zero() {
                let uniswap_v3 = UniswapV3::new_with_provider(self.provider.clone());
                if let Ok(price_data) = uniswap_v3
                    .get_price_by_token0_token1(pool_address, token_info, &weth_info)
                    .await
                {
                    return Ok(price_data);
                }
            }
        }

        Err(UniswapError::PoolNotFound)
    }

    /// get stable coin price
    async fn get_stablecoin_price(&self, token_info: &TokenInfo, stablecoin: &str) -> Result<f64> {
        let stablecoin_address: Address;
        match stablecoin {
            "USDC" => stablecoin_address = str_to_h160_1(USDC_ADDRESS).unwrap(),
            "USDT" => stablecoin_address = str_to_h160_1(USDT_ADDRESS).unwrap(),
            "DAI" => stablecoin_address = str_to_h160_1(DAI_ADDRESS).unwrap(),
            _ => stablecoin_address = str_to_h160_1(USDT_ADDRESS).unwrap(),
        }
        if token_info.address == stablecoin_address {
            return Ok(1.0);
        }
        let stablecoin_info = self.get_token_info(stablecoin_address).await.unwrap();
        let factory = IUniswapV2Factory::new(self.v2_factory, self.provider.clone());
        let pair_address = factory
            .get_pair(token_info.address, stablecoin_address)
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Factory error: {}", e)))?;
        if pair_address == Address::zero() {
            return Err(UniswapError::PoolNotFound);
        }
        let uniswap_v2 = UniswapV2::new_with_provider(self.provider.clone());
        let pool_info = uniswap_v2.get_pool_info(pair_address).await?;
        let price = self.calc_price(&pool_info, token_info, &stablecoin_info)?;
        Ok(price)
    }

    /// calculate USD price
    async fn calc_usd_price(&self, price_info: &TokenPriceInfo) -> Option<f64> {
        // usdc
        if let Some(usdc_price) = price_info.usdc_price {
            return Some(usdc_price);
        }
        // usdt
        if let Some(usdt_price) = price_info.usdt_price {
            return Some(usdt_price);
        }
        //dai
        if let Some(dai_price) = price_info.dai_price {
            return Some(dai_price);
        }
        if let Some(eth_price) = price_info.eth_price {
            if let Ok(eth_usd_price) = self.get_eth_usd_price().await {
                return Some(eth_price * eth_usd_price);
            }
        }
        None
    }

    /// get ETH/USD price
    async fn get_eth_usd_price(&self) -> Result<f64> {
        let weth_address = str_to_h160_1(WETH_ADDRESS).unwrap();
        let usdc_address = str_to_h160_1(USDC_ADDRESS).unwrap();
        let weth_info = self.get_token_info(weth_address).await.unwrap();
        let usdc_info = self.get_token_info(usdc_address).await.unwrap();
        let factory = IUniswapV2Factory::new(self.v2_factory, self.provider.clone());
        let pair_address = factory
            .get_pair(weth_address, usdc_address)
            .call()
            .await
            .map_err(|e| UniswapError::ContractError(format!("Factory error: {}", e)))?;
        if pair_address == Address::zero() {
            return Err(UniswapError::PoolNotFound);
        }
        let uniswap_v2 = UniswapV2::new_with_provider(self.provider.clone());
        let pool_info = uniswap_v2.get_pool_info(pair_address).await?;
        // 1 WETH price
        let price = self.calc_price(&pool_info, &weth_info, &usdc_info)?;
        Ok(price)
    }

    /// get token info
    async fn get_token_info(&self, address: Address) -> Result<TokenInfo> {
        let uniswap_v2 = UniswapV2::new_with_provider(self.provider.clone());
        uniswap_v2.get_token_info(address).await
    }

    /// get multiple prices by token addres
    pub async fn get_multiple_prices_by_token_address_vec(
        &self,
        token_addresses: Vec<Address>,
    ) -> Result<Vec<TokenPriceInfo>> {
        let mut results = Vec::new();

        for address in token_addresses {
            match self.get_token_prices_by_token_address(address).await {
                Ok(price_info) => results.push(price_info),
                Err(e) => {
                    eprintln!("Failed to get price for {:?}: {}", address, e);
                    // other token
                }
            }
        }
        Ok(results)
    }

    /// search for liquid trading pairs
    pub async fn search_liquid_pools(&self, token_address: Address) -> Result<Vec<PoolInfo>> {
        let mut pools = Vec::new();
        let token_info = self.get_token_info(token_address).await?;
        // check trading pairs with major coins
        let base_tokens = vec![
            str_to_h160_1(WETH_ADDRESS).unwrap(),
            str_to_h160_1(USDC_ADDRESS).unwrap(),
            str_to_h160_1(USDT_ADDRESS).unwrap(),
            str_to_h160_1(DAI_ADDRESS).unwrap(),
        ];

        for base_token in base_tokens {
            let factory = IUniswapV2Factory::new(self.v2_factory, self.provider.clone());
            let pair_address = factory
                .get_pair(token_address, base_token)
                .call()
                .await
                .map_err(|e| UniswapError::ContractError(format!("Factory error: {}", e)))?;
            if pair_address != Address::zero() {
                let uniswap_v2 = UniswapV2::new_with_provider(self.provider.clone());
                if let Ok(pool_info) = uniswap_v2.get_pool_info(pair_address).await {
                    // filter data whose liquidity pool is not zero.
                    if !pool_info.reserve0.is_zero() && !pool_info.reserve1.is_zero() {
                        pools.push(pool_info);
                    }
                }
            }
        }
        Ok(pools)
    }
}

impl UniswapV2 {
    pub fn new_with_provider(provider: Arc<Provider<Http>>) -> Self {
        Self { provider }
    }
}

impl UniswapV3 {
    pub fn new_with_provider(provider: Arc<Provider<Http>>) -> Self {
        Self { provider }
    }
}
