/// abi related to uniswap interaction
use ethers_contract::abigen;

/// V2 pair ABI
abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
        function totalSupply() external view returns (uint256)
    ]"#,
);

/// ERC20 standard ABI
abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function totalSupply() external view returns (uint256)
        function decimals() external view returns (uint8)
        function symbol() external view returns (string)
        function name() external view returns (string)
        function transfer(address to, uint256 amount) external returns (bool)
        function approve(address spender, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
    ]"#,
);

/// V3 Pool ABI
abigen!(
    IUniswapV3Pool,
    r#"[
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)
        function liquidity() external view returns (uint128)
        function token0() external view returns (address)
        function token1() external view returns (address)
        function fee() external view returns (uint24)
    ]"#,
);

/// V2 Router ABI
abigen!(
    IUniswapV2Router,
    r#"[
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function swapTokensForExactTokens(uint amountOut, uint amountInMax, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)
        function getAmountsOut(uint amountIn, address[] calldata path) external view returns (uint[] memory amounts)
        function getAmountsIn(uint amountOut, address[] calldata path) external view returns (uint[] memory amounts)
    ]"#,
);

/// V2 Factory ABI
abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
    ]"#,
);

/// V3 Factory ABI
abigen!(
    IUniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)
    ]"#,
);
