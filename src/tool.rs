/// address tool module
pub mod address {
    use ethers::types::Address;
    use std::str::FromStr;

    /// Convert string to Address
    pub fn str_to_address(address_str: &str) -> Result<Address, String> {
        Address::from_str(address_str.trim())
            .map_err(|e| format!("Invalid Ethereum address: {}", e))
    }

    /// Convert Address to checksum format
    pub fn to_checksum(address: &Address) -> String {
        format!("{:?}", address)
    }

    /// Verify address format
    pub fn verify_address_format(address: &str) -> bool {
        str_to_address(address).is_ok()
    }

    /// Check if address is zero address
    pub fn is_zero_address(address: &Address) -> bool {
        *address == Address::zero()
    }
}

/// number tool module
pub mod num {
    /// Format big numbers
    pub fn format_big_num(value: f64) -> String {
        if value >= 1_000_000_000.0 {
            format!("{:.2}B", value / 1_000_000_000.0)
        } else if value >= 1_000_000.0 {
            format!("{:.2}M", value / 1_000_000.0)
        } else if value >= 1_000.0 {
            format!("{:.2}K", value / 1_000.0)
        } else {
            format!("{:.6}", value)
        }
    }

    /// Convert U256 to f64 with decimals
    pub fn u256_to_f64(value: ethers::types::U256, decimals: u8) -> f64 {
        let divisor = ethers::types::U256::from(10).pow(ethers::types::U256::from(decimals));
        let integer_part = value / divisor;
        let fractional_part = value % divisor;

        let integer = integer_part.as_u64() as f64;
        let fractional = fractional_part.as_u64() as f64 / 10f64.powi(decimals as i32);

        integer + fractional
    }

    /// Convert f64 to U256 with decimals
    pub fn f64_to_u256(value: f64, decimals: u8) -> Result<ethers::types::U256, String> {
        let scaled_value = (value * 10f64.powi(decimals as i32)).round();

        if scaled_value.is_nan() || scaled_value.is_infinite() {
            return Err("Invalid number value".to_string());
        }

        match ethers::types::U256::from_dec_str(&scaled_value.to_string()) {
            Ok(val) => Ok(val),
            Err(_) => Err("Number conversion failed".to_string()),
        }
    }
}

/// price tool module
pub mod price {
    use crate::EvmError;
    use ethers::types::Address;
    use std::collections::HashMap;

    /// Price oracle trait for getting token prices
    pub trait PriceOracle {
        async fn get_price(&self, token_address: Address) -> Result<f64, EvmError>;
        async fn get_prices(
            &self,
            token_addresses: Vec<Address>,
        ) -> Result<HashMap<Address, f64>, EvmError>;
    }

    /// Simple price oracle implementation
    pub struct SimplePriceOracle;

    impl SimplePriceOracle {
        pub fn new() -> Self {
            Self
        }
    }

    impl PriceOracle for SimplePriceOracle {
        async fn get_price(&self, _token_address: Address) -> Result<f64, EvmError> {
            // Implement price fetching logic here
            // This is a placeholder implementation
            Ok(1.0)
        }

        async fn get_prices(
            &self,
            token_addresses: Vec<Address>,
        ) -> Result<HashMap<Address, f64>, EvmError> {
            let mut prices = HashMap::new();
            for address in token_addresses {
                prices.insert(address, self.get_price(address).await?);
            }
            Ok(prices)
        }
    }
}
