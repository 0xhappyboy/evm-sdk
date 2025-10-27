pub mod rpc {
    pub const ETHEREUM_RPC: &str = "https://reth-ethereum.ithaca.xyz/rpc";
    pub const BASE_RPC: &str = "https://mainnet.base.org";
    pub const ARB_RPC: &str = "https://arb1.arbitrum.io/rpc";
    pub const BSC_RPC: &str = "https://bsc-dataseed.binance.org/";
    pub const HYPEREVM_RPC: &str = "";
    pub const PLASMA_RPC: &str = "";
}

pub mod base {
    pub mod mainnet {
        pub mod dex {
            pub mod uniswap {
                pub const ROUTER_V2_ADDRESS: &str = "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24";
                pub const ROUTER_V3_ADDRESS: &str = "0x2626664c2603336E57B271c5C0b26F421741e481";
                pub const FACTORY_V2_ADDRESS: &str = "0x8909Dc15e40173Ff4699343b6eB8132c65e18eC6";
            }
        }
        pub mod lend {
            pub mod aave {
                pub const AAVE_ADDRESS: &str = "0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb";
            }
        }
    }
}
pub mod arb {
    pub mod mainnet {
        pub mod dex {
            pub mod uniswap {
                pub const ROUTER_V2_ADDRESS: &str = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506";
                pub const ROUTER_V3_ADDRESS: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
                pub const FACTORY_V2_ADDRESS: &str = "0xc35DADB65012eC5796536bD9864eD8773aBc74C4";
            }
        }
        pub mod lend {
            pub mod aave {
                pub const AAVE_ADDRESS: &str = "0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb";
            }
        }
    }
}

pub mod ethereum {
    pub mod mainnet {
        pub mod dex {
            pub mod uniswap {
                pub const ROUTER_V2_ADDRESS: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
                pub const ROUTER_V3_ADDRESS: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
                pub const FACTORY_V2_ADDRESS: &str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
            }
        }
        pub mod lend {
            pub mod aave {
                pub const AAVE_ADDRESS: &str = "0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e";
            }
        }
        pub mod token {
            /// mainnet WETH address
            pub const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
            /// mainnet USDC address
            pub const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
            /// mainnet USDT address
            pub const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
            /// mainnet DAI address
            pub const DAI_ADDRESS: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
            /// mainnet WBTC address
            pub const WBTC_ADDRESS: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";
        }
    }
}

pub mod bsc {
    pub mod mainnet {
        pub mod launchpad {
            pub mod four_meme {
                pub const FOUR_MEME_ADDRESS: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";
                pub const TARGET_MCAP: u64 = 69_000;
                pub const CURVE_FACTOR: u64 = 1_000_000;
            }
        }
        pub mod dex {
            pub mod uniswap {
                pub const ROUTER_V2_ADDRESS: &str = "0x10ED43C718714eb63d5aA57B78B54704E256024E";
                pub const ROUTER_V3_ADDRESS: &str = "0xB971eF87ede563556b2ED4b1C0b0019111Dd85d2";
                pub const FACTORY_V2_ADDRESS: &str = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73";
            }
            pub mod pancakeswap {
                pub const PANCAKESWAP_ROUTER: &str = "0x10ED43C718714eb63d5aA57B78B54704E256024E";
                pub const PANCAKESWAP_FACTORY: &str = "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73";
            }
        }
        pub mod tokens {
            // BSC 上常用代币地址
            pub const WBNB: &str = "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c";
            pub const BUSD: &str = "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56";
            pub const USDT: &str = "0x55d398326f99059fF775485246999027B3197955";
        }
    }
}
