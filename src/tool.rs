/// price tool module
pub mod price {}

/// address tool module
pub mod address {
    use ethaddr::Address;
    use ethers::types::H160;
    
    /// convert the address in string format to h160 format.
    pub fn str_to_h160_1(address_str: &str) -> Result<H160, String> {
        let address: Address = address_str
            .parse()
            .map_err(|e| format!("Invalid Ethereum address: {}", e))?;
        let bytes = address.as_ref();
        Ok(H160::from_slice(bytes))
    }
    pub fn str_to_h160_2(address_str: &str) -> Result<H160, String> {
        let clean = address_str.trim().trim_start_matches("0x");
        if clean.len() != 40 {
            return Err(format!("Invalid length: {}", clean.len()));
        }
        let mut bytes = [0u8; 20];
        hex::decode_to_slice(clean, &mut bytes).map_err(|e| format!("Hex decode error: {}", e))?;
        Ok(H160::from(bytes))
    }

    /// verify address format
    fn verify_address_format(address: &str) -> bool {
        let trimmed = address.trim();
        if trimmed.is_empty() {
            return false;
        }
        let without_prefix =
            if let Some(rest) = trimmed.strip_prefix("0x").or(trimmed.strip_prefix("0X")) {
                rest
            } else {
                trimmed
            };
        if without_prefix.len() != 40 {
            return false;
        }
        if !without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        true
    }
}

/// number tool module
pub mod num {
    /// format big numbers
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
}
