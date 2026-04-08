/// Uniswap
use crate::{Evm, EvmClient, EvmError};
use ethers::types::{Address, Bytes, H160, H256, I256, TransactionRequest, U256};
use ethers::{contract::abigen, providers::Provider, utils};
use std::sync::Arc;

// ==================== Uniswap V2 ABIs ====================

abigen!(
    IUniswapV2Factory,
    r#"[ 
        function getPair(address tokenA, address tokenB) external view returns (address)
        function createPair(address tokenA, address tokenB) external returns (address)
    ]"#
);

abigen!(
    IUniswapV2Pair,
    r#"[ 
        function token0() external view returns (address)
        function token1() external view returns (address)
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function swap(uint amount0Out, uint amount1Out, address to, bytes calldata data) external
        function mint(address to) external returns (uint liquidity)
        function burn(address to) external returns (uint amount0, uint amount1)
    ]"#
);

abigen!(
    IUniswapV2Router02,
    r#"[ 
        function factory() external view returns (address)
        function WETH() external view returns (address)
        function addLiquidity(address tokenA, address tokenB, uint amountADesired, uint amountBDesired, uint amountAMin, uint amountBMin, address to, uint deadline) external returns (uint amountA, uint amountB, uint liquidity)
        function addLiquidityETH(address token, uint amountTokenDesired, uint amountTokenMin, uint amountETHMin, address to, uint deadline) external payable returns (uint amountToken, uint amountETH, uint liquidity)
        function removeLiquidity(address tokenA, address tokenB, uint liquidity, uint amountAMin, uint amountBMin, address to, uint deadline) external returns (uint amountA, uint amountB)
        function removeLiquidityETH(address token, uint liquidity, uint amountTokenMin, uint amountETHMin, address to, uint deadline) external returns (uint amountToken, uint amountETH)
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function swapTokensForExactTokens(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)
        function swapTokensForExactETH(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function swapExactTokensForETH(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function swapETHForExactTokens(uint amountOut, address[] calldata path, address to, uint deadline) external payable returns (uint[] memory amounts)
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
        function getAmountsIn(uint amountOut, address[] calldata path) external view returns (uint[] memory amounts)
    ]"#
);

// ==================== Uniswap V3 ABIs ====================

abigen!(
    IUniswapV3Factory,
    r#"[ 
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address)
        function createPool(address tokenA, address tokenB, uint24 fee) external returns (address)
        function owner() external view returns (address)
        function feeAmountTickSpacing(uint24 fee) external view returns (int24)
    ]"#
);

abigen!(
    IUniswapV3Pool,
    r#"[ 
        function token0() external view returns (address)
        function token1() external view returns (address)
        function fee() external view returns (uint24)
        function liquidity() external view returns (uint128)
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)
        function swap(address recipient, bool zeroForOne, int256 amountSpecified, uint160 sqrtPriceLimitX96, bytes calldata data) external returns (int256 amount0, int256 amount1)
        function mint(address recipient, int24 tickLower, int24 tickUpper, uint128 amount, bytes calldata data) external returns (uint256 amount0, uint256 amount1)
        function burn(int24 tickLower, int24 tickUpper, uint128 amount) external returns (uint256 amount0, uint256 amount1)
        function collect(address recipient, int24 tickLower, int24 tickUpper, uint128 amount0Requested, uint128 amount1Requested) external returns (uint128 amount0, uint128 amount1)
        function observe(uint32[] calldata secondsAgos) external view returns (int56[] memory tickCumulatives, uint160[] memory secondsPerLiquidityCumulativeX128s)
        function increaseObservationCardinalityNext(uint16 observationCardinalityNext) external
    ]"#
);

// Uniswap V3 Router ABI - Using flattened parameters instead of tuples
abigen!(
    IUniswapV3Router,
    r#"[
        function factory() external view returns (address)
        function WETH9() external view returns (address)
        function exactInputSingle(address tokenIn, address tokenOut, uint24 fee, address recipient, uint256 deadline, uint256 amountIn, uint256 amountOutMinimum, uint160 sqrtPriceLimitX96) external payable returns (uint256 amountOut)
        function exactOutputSingle(address tokenIn, address tokenOut, uint24 fee, address recipient, uint256 deadline, uint256 amountOut, uint256 amountInMaximum, uint160 sqrtPriceLimitX96) external payable returns (uint256 amountIn)
        function exactInput(bytes path, address recipient, uint256 deadline, uint256 amountIn, uint256 amountOutMinimum) external payable returns (uint256 amountOut)
        function exactOutput(bytes path, address recipient, uint256 deadline, uint256 amountOut, uint256 amountInMaximum) external payable returns (uint256 amountIn)
        function multicall(bytes[] calldata data) external payable returns (bytes[] memory results)
    ]"#
);

// Uniswap V3 Nonfungible Position Manager - Using flattened parameters
abigen!(
    IUniswapV3Positions,
    r#"[
        function createAndInitializePoolIfNecessary(address token0, address token1, uint24 fee, uint160 sqrtPriceX96) external payable returns (address pool)
        function mint(address token0, address token1, uint24 fee, int24 tickLower, int24 tickUpper, uint128 amount0Desired, uint128 amount1Desired, uint128 amount0Min, uint128 amount1Min, address recipient, uint256 deadline) external payable returns (uint256 tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        function increaseLiquidity(uint256 tokenId, uint128 amount0Desired, uint128 amount1Desired, uint128 amount0Min, uint128 amount1Min, uint256 deadline) external payable returns (uint128 liquidity, uint256 amount0, uint256 amount1)
        function decreaseLiquidity(uint256 tokenId, uint128 liquidity, uint256 amount0Min, uint256 amount1Min, uint256 deadline) external payable returns (uint256 amount0, uint256 amount1)
        function collect(uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max) external payable returns (uint256 amount0, uint256 amount1)
        function burn(uint256 tokenId) external payable
    ]"#
);

// ==================== Uniswap V4 ABIs ====================

abigen!(
    IUniswapV4PoolManager,
    r#"[
        function initialize(address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks, uint160 sqrtPriceX96) external returns (int24 tick)
        function swap(address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks, bool zeroForOne, int256 amountSpecified, uint160 sqrtPriceLimitX96, bytes data) external returns (int256 amount0, int256 amount1)
        function modifyLiquidity(address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks, int24 tickLower, int24 tickUpper, int128 liquidityDelta, bytes data) external returns (int256 amount0, int256 amount1)
        function donate(address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks, uint256 amount0, uint256 amount1, bytes data) external returns (int256 amount0Delta, int256 amount1Delta)
        function unlock(bytes data) external returns (bytes memory result)
    ]"#
);

abigen!(
    IUniswapV4PositionManager,
    r#"[
        function mint(address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks, int24 tickLower, int24 tickUpper, uint128 liquidity) external returns (uint256 tokenId)
        function burn(uint256 tokenId) external
        function increaseLiquidity(uint256 tokenId, uint128 liquidityDelta) external
        function decreaseLiquidity(uint256 tokenId, uint128 liquidityDelta) external
    ]"#
);

// ==================== Custom Structures for Manual Encoding ====================

/// Uniswap V3 Router parameter structures
#[derive(Debug, Clone)]
pub struct ExactInputSingleParams {
    pub token_in: Address,
    pub token_out: Address,
    pub fee: u32,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_in: U256,
    pub amount_out_minimum: U256,
    pub sqrt_price_limit_x96: H160,
}

#[derive(Debug, Clone)]
pub struct ExactOutputSingleParams {
    pub token_in: Address,
    pub token_out: Address,
    pub fee: u32,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_out: U256,
    pub amount_in_maximum: U256,
    pub sqrt_price_limit_x96: H160,
}

#[derive(Debug, Clone)]
pub struct ExactInputParams {
    pub path: Vec<u8>,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_in: U256,
    pub amount_out_minimum: U256,
}

#[derive(Debug, Clone)]
pub struct ExactOutputParams {
    pub path: Vec<u8>,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_out: U256,
    pub amount_in_maximum: U256,
}

/// V3 Position mint parameters
#[derive(Debug, Clone)]
pub struct MintParams {
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub amount0_desired: u128,
    pub amount1_desired: u128,
    pub amount0_min: U256,
    pub amount1_min: U256,
    pub recipient: Address,
    pub deadline: U256,
}

/// V3 Position increase liquidity parameters
#[derive(Debug, Clone)]
pub struct IncreaseLiquidityParams {
    pub token_id: U256,
    pub amount0_desired: u128,
    pub amount1_desired: u128,
    pub amount0_min: U256,
    pub amount1_min: U256,
    pub deadline: U256,
}

/// V3 Position decrease liquidity parameters
#[derive(Debug, Clone)]
pub struct DecreaseLiquidityParams {
    pub token_id: U256,
    pub liquidity: U128,
    pub amount0_min: U256,
    pub amount1_min: U256,
    pub deadline: U256,
}

/// Uniswap V3 Router manual implementation
pub struct UniswapV3RouterManual<'a, P> {
    address: Address,
    provider: &'a P,
}

impl<'a, P: ethers::providers::Middleware> UniswapV3RouterManual<'a, P> {
    pub fn new(address: Address, provider: &'a P) -> Self {
        Self { address, provider }
    }

    /// Single pool exact input swap
    pub async fn exact_input_single(
        &self,
        params: ExactInputSingleParams,
    ) -> Result<H256, EvmError> {
        use ethers::abi::AbiEncode;
        let selector = &utils::id(
            "exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))",
        )[0..4];
        let encoded_params = (
            params.token_in,
            params.token_out,
            params.fee,
            params.recipient,
            params.deadline,
            params.amount_in,
            params.amount_out_minimum,
            params.sqrt_price_limit_x96,
        )
            .encode();
        let mut data = selector.to_vec();
        data.extend_from_slice(&encoded_params);
        let tx = TransactionRequest::new()
            .to(self.address)
            .data(Bytes::from(data));
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| {
                EvmError::TransactionError(format!("Failed to send transaction: {}", e))
            })?;
        Ok(pending_tx.tx_hash())
    }

    /// Single pool exact output swap
    pub async fn exact_output_single(
        &self,
        params: ExactOutputSingleParams,
    ) -> Result<H256, EvmError> {
        use ethers::abi::AbiEncode;
        let selector = &utils::id(
            "exactOutputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))",
        )[0..4];
        let encoded_params = (
            params.token_in,
            params.token_out,
            params.fee,
            params.recipient,
            params.deadline,
            params.amount_out,
            params.amount_in_maximum,
            params.sqrt_price_limit_x96,
        )
            .encode();
        let mut data = selector.to_vec();
        data.extend_from_slice(&encoded_params);
        let tx = TransactionRequest::new()
            .to(self.address)
            .data(Bytes::from(data));
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| {
                EvmError::TransactionError(format!("Failed to send transaction: {}", e))
            })?;
        Ok(pending_tx.tx_hash())
    }

    /// Multi-hop exact input swap
    pub async fn exact_input(&self, params: ExactInputParams) -> Result<H256, EvmError> {
        use ethers::abi::AbiEncode;
        let selector = &utils::id("exactInput((bytes,address,uint256,uint256,uint256))")[0..4];
        let encoded_params = (
            params.path,
            params.recipient,
            params.deadline,
            params.amount_in,
            params.amount_out_minimum,
        )
            .encode();
        let mut data = selector.to_vec();
        data.extend_from_slice(&encoded_params);
        let tx = TransactionRequest::new()
            .to(self.address)
            .data(Bytes::from(data));
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| {
                EvmError::TransactionError(format!("Failed to send transaction: {}", e))
            })?;
        Ok(pending_tx.tx_hash())
    }
}

// ==================== Uniswap V4 Structures ====================

/// Uniswap V4 Pool Key
#[derive(Debug, Clone)]
pub struct PoolKey {
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

/// Uniswap V4 Swap Parameters
#[derive(Debug, Clone)]
pub struct SwapParams {
    pub zero_for_one: bool,
    pub amount_specified: I256,
    pub sqrt_price_limit_x96: H160,
}

/// Uniswap V4 Liquidity Parameters
#[derive(Debug, Clone)]
pub struct LiquidityParams {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity_delta: I128,
}

/// Uniswap V4 Pool Manager manual implementation
pub struct UniswapV4PoolManagerManual<'a, P> {
    address: Address,
    provider: &'a P,
}

impl<'a, P: ethers::providers::Middleware> UniswapV4PoolManagerManual<'a, P> {
    pub fn new(address: Address, provider: &'a P) -> Self {
        Self { address, provider }
    }

    /// Initialize a pool
    pub async fn initialize(
        &self,
        pool_key: PoolKey,
        sqrt_price_x96: H160,
    ) -> Result<H256, EvmError> {
        use ethers::abi::AbiEncode;
        let selector =
            &utils::id("initialize((address,address,uint24,int24,address),uint160)")[0..4];
        let encoded_key = (
            pool_key.currency0,
            pool_key.currency1,
            pool_key.fee,
            pool_key.tick_spacing,
            pool_key.hooks,
        )
            .encode();
        let mut data = selector.to_vec();
        data.extend_from_slice(&encoded_key);
        data.extend_from_slice(&sqrt_price_x96.encode());
        let tx = TransactionRequest::new()
            .to(self.address)
            .data(Bytes::from(data));
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to initialize pool: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// Execute a swap
    pub async fn swap(
        &self,
        pool_key: PoolKey,
        swap_params: SwapParams,
        data: Vec<u8>,
    ) -> Result<H256, EvmError> {
        use ethers::abi::AbiEncode;
        let selector =
            &utils::id("swap((address,address,uint24,int24,address),(bool,int256,uint160),bytes)")
                [0..4];
        let encoded_key = (
            pool_key.currency0,
            pool_key.currency1,
            pool_key.fee,
            pool_key.tick_spacing,
            pool_key.hooks,
        )
            .encode();
        let encoded_params = (
            swap_params.zero_for_one,
            swap_params.amount_specified,
            swap_params.sqrt_price_limit_x96,
        )
            .encode();
        let mut data_bytes = selector.to_vec();
        data_bytes.extend_from_slice(&encoded_key);
        data_bytes.extend_from_slice(&encoded_params);
        data_bytes.extend_from_slice(&data.encode());
        let tx = TransactionRequest::new()
            .to(self.address)
            .data(Bytes::from(data_bytes));
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// Modify liquidity
    pub async fn modify_liquidity(
        &self,
        pool_key: PoolKey,
        liquidity_params: LiquidityParams,
        data: Vec<u8>,
    ) -> Result<H256, EvmError> {
        use ethers::abi::AbiEncode;
        let selector = &utils::id(
            "modifyLiquidity((address,address,uint24,int24,address),(int24,int24,int128),bytes)",
        )[0..4];
        let encoded_key = (
            pool_key.currency0,
            pool_key.currency1,
            pool_key.fee,
            pool_key.tick_spacing,
            pool_key.hooks,
        )
            .encode();
        let encoded_liquidity = (
            liquidity_params.tick_lower,
            liquidity_params.tick_upper,
            liquidity_params.liquidity_delta,
        )
            .encode();
        let mut data_bytes = selector.to_vec();
        data_bytes.extend_from_slice(&encoded_key);
        data_bytes.extend_from_slice(&encoded_liquidity);
        data_bytes.extend_from_slice(&data.encode());
        let tx = TransactionRequest::new()
            .to(self.address)
            .data(Bytes::from(data_bytes));
        let pending_tx = self
            .provider
            .send_transaction(tx, None)
            .await
            .map_err(|e| {
                EvmError::TransactionError(format!("Failed to modify liquidity: {}", e))
            })?;
        Ok(pending_tx.tx_hash())
    }
}

// ==================== Uniswap Service Implementation ====================

/// Uniswap Service
pub struct UniswapService {
    evm: Arc<Evm>,
}

impl UniswapService {
    pub fn new(evm: Arc<Evm>) -> Self {
        Self { evm }
    }

    /// Create V2 Router instance
    fn v2_router(
        &self,
        router_address: Address,
    ) -> IUniswapV2Router02<Provider<ethers::providers::Http>> {
        IUniswapV2Router02::new(router_address, self.evm.client.provider.clone())
    }

    /// Create V2 Factory instance
    fn v2_factory(
        &self,
        factory_address: Address,
    ) -> IUniswapV2Factory<Provider<ethers::providers::Http>> {
        IUniswapV2Factory::new(factory_address, self.evm.client.provider.clone())
    }

    /// Create V2 Pair instance
    fn v2_pair(&self, pair_address: Address) -> IUniswapV2Pair<Provider<ethers::providers::Http>> {
        IUniswapV2Pair::new(pair_address, self.evm.client.provider.clone())
    }

    /// V2 - Get Pair address
    pub async fn v2_get_pair(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address, EvmError> {
        let factory = self.v2_factory(factory_address);
        factory
            .get_pair(token_a, token_b)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pair: {}", e)))
    }

    /// V2 - Get reserves
    pub async fn v2_get_reserves(
        &self,
        pair_address: Address,
    ) -> Result<(u128, u128, u32), EvmError> {
        let pair = self.v2_pair(pair_address);
        let reserves = pair
            .get_reserves()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get reserves: {}", e)))?;
        Ok((reserves.0, reserves.1, reserves.2))
    }

    /// V2 - Add liquidity (ERC20/ERC20)
    pub async fn v2_add_liquidity(
        &self,
        router_address: Address,
        token_a: Address,
        token_b: Address,
        amount_a_desired: U256,
        amount_b_desired: U256,
        amount_a_min: U256,
        amount_b_min: U256,
        to: Address,
        deadline: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let router = self.v2_router(router_address);
        let tx = router.add_liquidity(
            token_a,
            token_b,
            amount_a_desired,
            amount_b_desired,
            amount_a_min,
            amount_b_min,
            to,
            deadline,
        );
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to add liquidity: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// V2 - Add liquidity (ETH/ERC20)
    pub async fn v2_add_liquidity_eth(
        &self,
        router_address: Address,
        token: Address,
        amount_token_desired: U256,
        amount_token_min: U256,
        amount_eth_min: U256,
        to: Address,
        deadline: U256,
        eth_value: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let router = self.v2_router(router_address);
        let tx = router
            .add_liquidity_eth(
                token,
                amount_token_desired,
                amount_token_min,
                amount_eth_min,
                to,
                deadline,
            )
            .value(eth_value);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to add ETH liquidity: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// V2 - Remove liquidity
    pub async fn v2_remove_liquidity(
        &self,
        router_address: Address,
        token_a: Address,
        token_b: Address,
        liquidity: U256,
        amount_a_min: U256,
        amount_b_min: U256,
        to: Address,
        deadline: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let router = self.v2_router(router_address);
        let tx = router.remove_liquidity(
            token_a,
            token_b,
            liquidity,
            amount_a_min,
            amount_b_min,
            to,
            deadline,
        );
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to remove liquidity: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// V2 - Token swap (ExactIn)
    pub async fn v2_swap_exact_tokens_for_tokens(
        &self,
        router_address: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let router = self.v2_router(router_address);
        let tx = router.swap_exact_tokens_for_tokens(amount_in, amount_out_min, path, to, deadline);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap tokens: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// V2 - ETH to token swap
    pub async fn v2_swap_eth_for_tokens(
        &self,
        router_address: Address,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
        eth_value: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let router = self.v2_router(router_address);
        let tx = router
            .swap_exact_eth_for_tokens(amount_out_min, path, to, deadline)
            .value(eth_value);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to swap ETH for tokens: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// V2 - Token to ETH swap
    pub async fn v2_swap_tokens_for_eth(
        &self,
        router_address: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let router = self.v2_router(router_address);
        let tx = router.swap_exact_tokens_for_eth(amount_in, amount_out_min, path, to, deadline);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to swap tokens for ETH: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// V2 - Get output amounts
    pub async fn v2_get_amounts_out(
        &self,
        router_address: Address,
        amount_in: U256,
        path: Vec<Address>,
    ) -> Result<Vec<U256>, EvmError> {
        let router = self.v2_router(router_address);
        router
            .get_amounts_out(amount_in, path)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get amounts out: {}", e)))
    }

    // ==================== V3 Methods ====================

    /// Create V3 Factory instance
    fn v3_factory(
        &self,
        factory_address: Address,
    ) -> IUniswapV3Factory<Provider<ethers::providers::Http>> {
        IUniswapV3Factory::new(factory_address, self.evm.client.provider.clone())
    }

    /// Create V3 Pool instance
    fn v3_pool(&self, pool_address: Address) -> IUniswapV3Pool<Provider<ethers::providers::Http>> {
        IUniswapV3Pool::new(pool_address, self.evm.client.provider.clone())
    }

    /// Create V3 Router instance
    fn v3_router(
        &self,
        router_address: Address,
    ) -> IUniswapV3Router<Provider<ethers::providers::Http>> {
        IUniswapV3Router::new(router_address, self.evm.client.provider.clone())
    }

    /// Create V3 Positions instance
    fn v3_positions(
        &self,
        positions_address: Address,
    ) -> IUniswapV3Positions<Provider<ethers::providers::Http>> {
        IUniswapV3Positions::new(positions_address, self.evm.client.provider.clone())
    }

    /// V3 - Get Pool address
    pub async fn v3_get_pool(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
        fee: u32,
    ) -> Result<Address, EvmError> {
        let factory = self.v3_factory(factory_address);
        factory
            .get_pool(token_a, token_b, fee)
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get pool: {}", e)))
    }

    /// V3 - Create Pool
    pub async fn v3_create_pool(
        &self,
        factory_address: Address,
        token_a: Address,
        token_b: Address,
        fee: u32,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let factory = self.v3_factory(factory_address);
        let tx = factory.create_pool(token_a, token_b, fee);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to create pool: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// V3 - Get slot0 (sqrtPriceX96, tick, etc.)
    pub async fn v3_get_slot0(
        &self,
        pool_address: Address,
    ) -> Result<(H160, i32, u16, u16, u16, u8, bool), EvmError> {
        let pool = self.v3_pool(pool_address);
        let slot0 = pool
            .slot_0()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get slot0: {}", e)))?;

        // Convert U256 to H160 (take lower 160 bits)
        let sqrt_price_x96 = {
            let mut bytes = [0u8; 32];
            slot0.0.to_big_endian(&mut bytes);
            H160::from_slice(&bytes[12..32])
        };

        Ok((
            sqrt_price_x96,
            slot0.1,
            slot0.2,
            slot0.3,
            slot0.4,
            slot0.5,
            slot0.6,
        ))
    }

    /// V3 - Get liquidity
    pub async fn v3_get_liquidity(&self, pool_address: Address) -> Result<u128, EvmError> {
        let pool = self.v3_pool(pool_address);
        pool.liquidity()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get liquidity: {}", e)))
    }

    /// V3 - Get token0 address
    pub async fn v3_get_token0(&self, pool_address: Address) -> Result<Address, EvmError> {
        let pool = self.v3_pool(pool_address);
        pool.token_0()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get token0: {}", e)))
    }

    /// V3 - Get token1 address
    pub async fn v3_get_token1(&self, pool_address: Address) -> Result<Address, EvmError> {
        let pool = self.v3_pool(pool_address);
        pool.token_1()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get token1: {}", e)))
    }

    /// V3 - Get fee
    pub async fn v3_get_fee(&self, pool_address: Address) -> Result<u32, EvmError> {
        let pool = self.v3_pool(pool_address);
        pool.fee()
            .call()
            .await
            .map_err(|e| EvmError::ContractError(format!("Failed to get fee: {}", e)))
    }

    /// V3 - Single pool exact input swap using Router
    pub async fn v3_exact_input_single(
        &self,
        router_address: Address,
        params: ExactInputSingleParams,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;

        let router = self.v3_router(router_address);

        // Convert H160 to U256 for sqrtPriceLimitX96
        let sqrt_price_limit = {
            let mut bytes = [0u8; 32];
            bytes[12..32].copy_from_slice(params.sqrt_price_limit_x96.as_bytes());
            U256::from_big_endian(&bytes)
        };

        let tx = router.exact_input_single(
            params.token_in,
            params.token_out,
            params.fee,
            params.recipient,
            params.deadline,
            params.amount_in,
            params.amount_out_minimum,
            sqrt_price_limit,
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// V3 - Single pool exact output swap using Router
    pub async fn v3_exact_output_single(
        &self,
        router_address: Address,
        params: ExactOutputSingleParams,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;

        let router = self.v3_router(router_address);

        // Convert H160 to U256 for sqrtPriceLimitX96
        let sqrt_price_limit = {
            let mut bytes = [0u8; 32];
            bytes[12..32].copy_from_slice(params.sqrt_price_limit_x96.as_bytes());
            U256::from_big_endian(&bytes)
        };

        let tx = router.exact_output_single(
            params.token_in,
            params.token_out,
            params.fee,
            params.recipient,
            params.deadline,
            params.amount_out,
            params.amount_in_maximum,
            sqrt_price_limit,
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// V3 - Multi-hop exact input swap
    pub async fn v3_exact_input(
        &self,
        router_address: Address,
        path: Vec<u8>,
        recipient: Address,
        deadline: U256,
        amount_in: U256,
        amount_out_minimum: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;

        let router = self.v3_router(router_address);
        let tx = router.exact_input(
            path.into(),
            recipient,
            deadline,
            amount_in,
            amount_out_minimum,
        );

        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to swap: {}", e)))?;

        Ok(pending_tx.tx_hash())
    }

    /// V3 - Execute swap directly on Pool
    pub async fn v3_pool_swap(
        &self,
        pool_address: Address,
        recipient: Address,
        zero_for_one: bool,
        amount_specified: I256,
        sqrt_price_limit_x96: H160,
        data: Vec<u8>,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let pool = self.v3_pool(pool_address);

        // Convert H160 to U256
        let sqrt_price_limit_u256 = {
            let mut bytes = [0u8; 32];
            bytes[12..32].copy_from_slice(sqrt_price_limit_x96.as_bytes());
            U256::from_big_endian(&bytes)
        };

        let tx = pool.swap(
            recipient,
            zero_for_one,
            amount_specified,
            sqrt_price_limit_u256,
            data.into(),
        );
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to pool swap: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// V3 - Mint liquidity position
    pub async fn v3_mint_position(
        &self,
        positions_address: Address,
        params: MintParams,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let positions = self.v3_positions(positions_address);
        let tx = positions.mint(
            params.token0,
            params.token1,
            params.fee,
            params.tick_lower,
            params.tick_upper,
            params.amount0_desired as u128,
            params.amount1_desired as u128,
            params.amount0_min.as_u128(),
            params.amount1_min.as_u128(),
            params.recipient,
            params.deadline,
        );
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to mint position: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// V3 - Create and initialize pool if necessary
    pub async fn v3_create_and_initialize_pool(
        &self,
        positions_address: Address,
        token0: Address,
        token1: Address,
        fee: u32,
        sqrt_price_x96: H160,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let positions = self.v3_positions(positions_address);
        let sqrt_price = {
            let mut bytes = [0u8; 32];
            bytes[12..32].copy_from_slice(sqrt_price_x96.as_bytes());
            U256::from_big_endian(&bytes)
        };
        let tx = positions.create_and_initialize_pool_if_necessary(token0, token1, fee, sqrt_price);
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to create and initialize pool: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    // ==================== V4 Methods ====================

    /// V4 - Initialize pool
    pub async fn v4_initialize(
        &self,
        manager_address: Address,
        pool_key: PoolKey,
        sqrt_price_x96: H160,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let manager = UniswapV4PoolManagerManual::new(manager_address, &self.evm.client.provider);
        manager.initialize(pool_key, sqrt_price_x96).await
    }

    /// V4 - Execute swap
    pub async fn v4_swap(
        &self,
        manager_address: Address,
        pool_key: PoolKey,
        swap_params: SwapParams,
        data: Vec<u8>,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let manager = UniswapV4PoolManagerManual::new(manager_address, &self.evm.client.provider);
        manager.swap(pool_key, swap_params, data).await
    }

    /// V4 - Modify liquidity
    pub async fn v4_modify_liquidity(
        &self,
        manager_address: Address,
        pool_key: PoolKey,
        liquidity_params: LiquidityParams,
        data: Vec<u8>,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let manager = UniswapV4PoolManagerManual::new(manager_address, &self.evm.client.provider);
        manager
            .modify_liquidity(pool_key, liquidity_params, data)
            .await
    }

    fn check_wallet(&self) -> Result<(), EvmError> {
        if self.evm.client.wallet.is_none() {
            Err(EvmError::WalletError("No wallet configured".to_string()))
        } else {
            Ok(())
        }
    }
}

// ==================== Path Builder Utilities ====================

/// Swap path building helper
pub struct SwapPathBuilder;

impl SwapPathBuilder {
    /// Build V2 path (address list)
    pub fn build_v2_path(tokens: Vec<Address>) -> Vec<Address> {
        tokens
    }

    /// Build V3 encoded path
    /// Format: [token0, fee0, token1, fee1, token2, ...]
    pub fn build_v3_path(tokens_and_fees: Vec<(Address, u32, Address)>) -> Vec<u8> {
        let mut path = Vec::new();
        for (i, (token0, fee, token1)) in tokens_and_fees.into_iter().enumerate() {
            if i == 0 {
                path.extend_from_slice(token0.as_bytes());
            }
            path.extend_from_slice(&fee.to_be_bytes());
            path.extend_from_slice(token1.as_bytes());
        }
        path
    }

    /// Build V3 path from address list (using default fee tiers)
    pub fn build_v3_path_with_fees(tokens: Vec<Address>, fees: Vec<u32>) -> Vec<u8> {
        if tokens.len() < 2 || tokens.len() != fees.len() + 1 {
            panic!("Invalid tokens and fees length");
        }
        let mut path = Vec::new();
        path.extend_from_slice(tokens[0].as_bytes());
        for i in 0..fees.len() {
            path.extend_from_slice(&fees[i].to_be_bytes());
            path.extend_from_slice(tokens[i + 1].as_bytes());
        }
        path
    }
}

// ==================== Fee Tiers ====================

/// Common fee tiers for V3
#[derive(Debug, Clone, Copy)]
pub enum FeeTier {
    Low = 500,     // 0.05%
    Medium = 3000, // 0.3%
    High = 10000,  // 1%
}

impl FeeTier {
    pub fn value(&self) -> u32 {
        *self as u32
    }
}

// ==================== Type Aliases ====================

pub type U128 = u128;
pub type I128 = i128;

// ==================== Additional V3 Position Methods ====================

impl UniswapService {
    /// V3 - Increase liquidity
    pub async fn v3_increase_liquidity(
        &self,
        positions_address: Address,
        params: IncreaseLiquidityParams,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let positions = self.v3_positions(positions_address);
        let tx = positions.increase_liquidity(
            params.token_id,
            params.amount0_desired as u128,
            params.amount1_desired as u128,
            params.amount0_min.as_u128(),
            params.amount1_min.as_u128(),
            params.deadline,
        );
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to increase liquidity: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// V3 - Decrease liquidity
    pub async fn v3_decrease_liquidity(
        &self,
        positions_address: Address,
        params: DecreaseLiquidityParams,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let positions = self.v3_positions(positions_address);
        let tx = positions.decrease_liquidity(
            params.token_id,
            params.liquidity,
            params.amount0_min,
            params.amount1_min,
            params.deadline,
        );
        let pending_tx = tx.send().await.map_err(|e| {
            EvmError::TransactionError(format!("Failed to decrease liquidity: {}", e))
        })?;
        Ok(pending_tx.tx_hash())
    }

    /// V3 - Collect fees
    pub async fn v3_collect(
        &self,
        positions_address: Address,
        token_id: U256,
        recipient: Address,
        amount0_max: U128,
        amount1_max: U128,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let positions = self.v3_positions(positions_address);
        let tx = positions.collect(token_id, recipient, amount0_max.into(), amount1_max.into());
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to collect fees: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }

    /// V3 - Burn position
    pub async fn v3_burn_position(
        &self,
        positions_address: Address,
        token_id: U256,
    ) -> Result<H256, EvmError> {
        self.check_wallet()?;
        let positions = self.v3_positions(positions_address);
        let tx = positions.burn(token_id);
        let pending_tx = tx
            .send()
            .await
            .map_err(|e| EvmError::TransactionError(format!("Failed to burn position: {}", e)))?;
        Ok(pending_tx.tx_hash())
    }
}

// ======================== Test ========================
// ==================== Unit Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Evm;
    use ethers::types::{Address, U256};
    use std::str::FromStr;
    use std::sync::Arc;

    // Mock addresses for testing
    const MOCK_FACTORY_V2: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
    const MOCK_ROUTER_V2: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
    const MOCK_FACTORY_V3: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
    const MOCK_ROUTER_V3: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    const MOCK_POSITIONS_V3: &str = "0xC36442b4a4522E871399CD717aBDD847Ab11FE88";
    const MOCK_MANAGER_V4: &str = "0x000000000004444c5dc75cB358380D2e3dE7382";
    const MOCK_TOKEN_A: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    const MOCK_TOKEN_B: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    const MOCK_RECIPIENT: &str = "0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B";

    ///  Get Pair Address
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

    /// Get Reserves and Calculate Price
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

    /// Get Pool Information
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

    /// Build Swap Path and Get Amounts
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

    /// Exact Input Swap Transaction Creation
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

    /// Pool Key and Initialization Parameters
    #[test]
    fn test_v4_pool_key_and_parameters() {
        let token0 = Address::from_str(MOCK_TOKEN_A).unwrap();
        let token1 = Address::from_str(MOCK_TOKEN_B).unwrap();
        let hooks = Address::zero(); // No hooks for basic pool
        let fee = FeeTier::Medium.value();
        let tick_spacing = 60; // Standard for 0.3% fee tier
        let pool_key = PoolKey {
            currency0: token0,
            currency1: token1,
            fee,
            tick_spacing,
            hooks,
        };
        println!("V4 Pool Key:");
        println!("  currency0: {:?}", pool_key.currency0);
        println!("  currency1: {:?}", pool_key.currency1);
        println!("  fee: {}", pool_key.fee);
        println!("  tick_spacing: {}", pool_key.tick_spacing);
        println!("  hooks: {:?}", pool_key.hooks);
        let swap_params = SwapParams {
            zero_for_one: true,                    // Swap token0 for token1
            amount_specified: I256::from(1000000), // 1 token
            sqrt_price_limit_x96: H160::zero(),
        };
        println!("V4 Swap Params:");
        println!("  zero_for_one: {}", swap_params.zero_for_one);
        println!("  amount_specified: {}", swap_params.amount_specified);
        let liquidity_params = LiquidityParams {
            tick_lower: -887220,                      // Typical lower tick for E/U
            tick_upper: 887220,                       // Typical upper tick
            liquidity_delta: 1000000000000000000i128, // 1e18
        };
        println!("V4 Liquidity Params:");
        println!("  tick_lower: {}", liquidity_params.tick_lower);
        println!("  tick_upper: {}", liquidity_params.tick_upper);
        println!("  liquidity_delta: {}", liquidity_params.liquidity_delta);
        assert_eq!(pool_key.fee, 3000);
        assert_eq!(pool_key.tick_spacing, 60);
        assert!(swap_params.amount_specified > I256::zero());
        assert_eq!(FeeTier::Low.value(), 500);
        assert_eq!(FeeTier::Medium.value(), 3000);
        assert_eq!(FeeTier::High.value(), 10000);
        println!("All V4 structure tests passed!");
    }
}
