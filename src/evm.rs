/// evm network
use ethers::prelude::*;
use std::sync::Arc;

const ETHEREUM_RPC: &str = "https://reth-ethereum.ithaca.xyz/rpc";
const BASE_RPC: &str = "";
const ARB_RPC: &str = "";
const BSC_RPC: &str = "";
const HYPEREVM_RPC: &str = "";
const PLASMA_RPC: &str = "";

pub enum EvmType {
    Ethereum,
    Arb,
    Bsc,
    Base,
    HyperEVM,
    Plasma,
}

#[derive(Clone)]
pub struct Evm {
    pub provider: Arc<Provider<Http>>,
}

impl Evm {
    pub async fn new(evm_type: EvmType) -> Result<Self, String> {
        let mut rpc: &str = "";
        match evm_type {
            EvmType::Ethereum => {
                rpc = ETHEREUM_RPC;
            }
            EvmType::Arb => {
                // let provider = ProviderBuilder::new().connect(ARB_RPC).await.unwrap();
                rpc = ARB_RPC;
            }
            EvmType::Bsc => {
                rpc = BSC_RPC;
            }
            EvmType::Base => {
                rpc = BASE_RPC;
            }
            EvmType::HyperEVM => {
                rpc = HYPEREVM_RPC;
            }
            EvmType::Plasma => {
                rpc = PLASMA_RPC;
            }
        }
        match Provider::<Http>::try_from(rpc) {
            Ok(p) => {
                return Ok(Self {
                    provider: Arc::new(p),
                });
            }
            Err(e) => Err(format!("create provider error: {:?}", e).to_string()),
        }
    }
}
