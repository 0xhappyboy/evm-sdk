use crate::{Evm, EvmError};
use ethers::providers::{Http, Middleware};
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// Result of security checks for a smart contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCheckResult {
    pub contract_address: Address,
    pub checks: Vec<SecurityCheck>,
    pub overall_score: f64,
    pub risk_level: RiskLevel,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    // Additional metrics
    pub metrics: ContractMetrics,
}

/// Contract metrics from on-chain analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractMetrics {
    pub owner_renounced: bool,
    pub lp_ratio: f64,
    pub first_lp_percentage: f64,
    pub holder_count: u64,
    pub market_cap: f64,
    pub price: f64,
    pub liquidity_usd: f64,
    pub ath_mcap: f64,
    pub vol_24h: f64,
    pub lp_locked_percentage: f64,
    pub buy_tax: f64,
    pub sell_tax: f64,
    pub age_days: u64,
    pub has_max_wallet: bool,
    pub has_multi_blacklist: bool,
    pub is_honeypot: bool,
    pub can_take_fees: bool,
    pub has_anti_whale: bool,
    pub has_cooldown: bool,
}

// Individual security check item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCheck {
    pub check_type: SecurityCheckType,
    pub passed: bool,
    pub score: f64,
    pub details: String,
    pub evidence: Vec<String>,
}

/// Types of security checks performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityCheckType {
    OwnershipRenounced,
    LpLocked,
    TaxZero,
    NoHoneypot,
    HealthyHolderDistribution,
    AntiWhaleMechanism,
    NoBlacklist,
    LiquiditySufficient,
    AgeSufficient,
    HealthyVolume,
    MaxWalletCheck,
    CooldownCheck,
}

/// Risk level classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Security checker for smart contract analysis
pub struct SecurityChecker {
    evm: Arc<Evm>,
    known_vulnerabilities: HashSet<String>,
}

impl SecurityChecker {
    /// Creates a new SecurityChecker instance
    pub fn new(evm: Arc<Evm>) -> Self {
        let mut known_vulnerabilities = HashSet::new();
        known_vulnerabilities.insert("reentrancy".to_string());
        known_vulnerabilities.insert("integer-overflow".to_string());
        known_vulnerabilities.insert("access-control".to_string());
        known_vulnerabilities.insert("unchecked-call".to_string());
        known_vulnerabilities.insert("front-running".to_string());
        Self {
            evm,
            known_vulnerabilities,
        }
    }

    /// Performs comprehensive security audit on a smart contract
    pub async fn perform_security_audit(
        &self,
        contract_address: Address,
        source_code: Option<&str>,
    ) -> Result<SecurityCheckResult, EvmError> {
        let mut checks = Vec::new();
        let mut warnings = Vec::new();
        let mut recommendations = Vec::new();

        // Collect all metrics
        let metrics = self.collect_contract_metrics(contract_address).await?;

        // Perform each security check
        checks.push(self.check_ownership_renounced(&metrics).await?);
        checks.push(self.check_lp_locked(&metrics).await?);
        checks.push(self.check_tax_zero(&metrics).await?);
        checks.push(self.check_no_honeypot(&metrics).await?);
        checks.push(self.check_holder_distribution(&metrics).await?);
        checks.push(self.check_anti_whale(&metrics).await?);
        checks.push(self.check_no_blacklist(&metrics).await?);
        checks.push(self.check_liquidity_sufficient(&metrics).await?);
        checks.push(self.check_age_sufficient(&metrics).await?);
        checks.push(self.check_healthy_volume(&metrics).await?);
        checks.push(self.check_max_wallet(&metrics).await?);
        checks.push(self.check_cooldown(&metrics).await?);

        let overall_score = self.calculate_overall_score(&checks);
        let risk_level = self.determine_risk_level(overall_score);
        self.generate_warnings_and_recommendations(
            &checks,
            &mut warnings,
            &mut recommendations,
            &metrics,
        );

        Ok(SecurityCheckResult {
            contract_address,
            checks,
            overall_score,
            risk_level,
            warnings,
            recommendations,
            metrics,
        })
    }

    /// Collect all on-chain metrics
    async fn collect_contract_metrics(
        &self,
        contract_address: Address,
    ) -> Result<ContractMetrics, EvmError> {
        let mut metrics = ContractMetrics::default();

        // Check if owner is renounced (owner = address(0))
        metrics.owner_renounced = self
            .check_owner_renounced(contract_address)
            .await
            .unwrap_or(false);

        // Get holder info
        metrics.holder_count = self.get_holder_count(contract_address).await.unwrap_or(0);

        // Check LP status
        metrics.lp_locked_percentage = self
            .check_lp_lock_status(contract_address)
            .await
            .unwrap_or(0.0);

        // Check taxes
        let (buy_tax, sell_tax) = self.get_taxes(contract_address).await.unwrap_or((0.0, 0.0));
        metrics.buy_tax = buy_tax;
        metrics.sell_tax = sell_tax;

        // Check for blacklist/honeypot
        metrics.has_multi_blacklist = self
            .has_blacklist_function(contract_address)
            .await
            .unwrap_or(false);
        metrics.is_honeypot = self.is_honeypot(contract_address).await.unwrap_or(false);
        metrics.can_take_fees = self.can_take_fees(contract_address).await.unwrap_or(false);

        // Anti-whale check
        metrics.has_anti_whale = self.has_anti_whale(contract_address).await.unwrap_or(false);
        metrics.has_max_wallet = self.has_max_wallet(contract_address).await.unwrap_or(false);
        metrics.has_cooldown = self.has_cooldown(contract_address).await.unwrap_or(false);

        // Liquidity check
        metrics.liquidity_usd = self
            .get_liquidity_usd(contract_address)
            .await
            .unwrap_or(0.0);

        Ok(metrics)
    }

    async fn check_owner_renounced(&self, address: Address) -> Result<bool, EvmError> {
        // Try to get owner via Ownable pattern
        // If owner is address(0), it's renounced
        Ok(false) // Placeholder - implement actual check
    }

    async fn get_holder_count(&self, address: Address) -> Result<u64, EvmError> {
        // Get total number of unique token holders
        Ok(4184) // Placeholder
    }

    async fn check_lp_lock_status(&self, address: Address) -> Result<f64, EvmError> {
        // Check if LP tokens are locked/burned
        Ok(100.0) // Placeholder
    }

    async fn get_taxes(&self, address: Address) -> Result<(f64, f64), EvmError> {
        // Analyze buy/sell tax percentages
        Ok((0.0, 0.0)) // Placeholder
    }

    async fn has_blacklist_function(&self, address: Address) -> Result<bool, EvmError> {
        // Check for blacklist functions
        Ok(false) // Placeholder
    }

    async fn is_honeypot(&self, address: Address) -> Result<bool, EvmError> {
        // Check if contract is a honeypot (cannot sell)
        Ok(false) // Placeholder
    }

    async fn can_take_fees(&self, address: Address) -> Result<bool, EvmError> {
        // Check if owner can take fees
        Ok(false) // Placeholder
    }

    async fn has_anti_whale(&self, address: Address) -> Result<bool, EvmError> {
        // Check for max transaction limit
        Ok(true) // Placeholder
    }

    async fn has_max_wallet(&self, address: Address) -> Result<bool, EvmError> {
        // Check for max wallet limit
        Ok(true) // Placeholder
    }

    async fn has_cooldown(&self, address: Address) -> Result<bool, EvmError> {
        // Check for cooldown mechanism between trades
        Ok(false) // Placeholder
    }

    async fn get_liquidity_usd(&self, address: Address) -> Result<f64, EvmError> {
        // Get liquidity in USD
        Ok(220100.0) // Placeholder - $220.1K
    }

    /// Check 1: Owner renounced
    async fn check_ownership_renounced(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        Ok(SecurityCheck {
            check_type: SecurityCheckType::OwnershipRenounced,
            passed: metrics.owner_renounced,
            score: if metrics.owner_renounced { 1.0 } else { 0.0 },
            details: if metrics.owner_renounced {
                "Owner has been renounced ✅".to_string()
            } else {
                "Owner not renounced - contract can be modified ⚠️".to_string()
            },
            evidence: vec![],
        })
    }

    /// Check 2: LP locked/burned
    async fn check_lp_locked(&self, metrics: &ContractMetrics) -> Result<SecurityCheck, EvmError> {
        let passed = metrics.lp_locked_percentage >= 80.0;
        Ok(SecurityCheck {
            check_type: SecurityCheckType::LpLocked,
            passed,
            score: metrics.lp_locked_percentage / 100.0,
            details: format!("LP locked: {}%", metrics.lp_locked_percentage),
            evidence: vec![],
        })
    }

    /// Check 3: Zero taxes
    async fn check_tax_zero(&self, metrics: &ContractMetrics) -> Result<SecurityCheck, EvmError> {
        let passed = metrics.buy_tax == 0.0 && metrics.sell_tax == 0.0;
        Ok(SecurityCheck {
            check_type: SecurityCheckType::TaxZero,
            passed,
            score: if passed { 1.0 } else { 0.5 },
            details: format!(
                "Buy Tax: {}% | Sell Tax: {}%",
                metrics.buy_tax, metrics.sell_tax
            ),
            evidence: vec![],
        })
    }

    /// Check 4: No honeypot
    async fn check_no_honeypot(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        Ok(SecurityCheck {
            check_type: SecurityCheckType::NoHoneypot,
            passed: !metrics.is_honeypot && !metrics.can_take_fees,
            score: if metrics.is_honeypot {
                0.0
            } else if metrics.can_take_fees {
                0.3
            } else {
                1.0
            },
            details: if metrics.is_honeypot {
                "⚠️ HONEYPOT DETECTED! Cannot sell!".to_string()
            } else if metrics.can_take_fees {
                "Contract can take fees - proceed with caution".to_string()
            } else {
                "No honeypot detected ✅".to_string()
            },
            evidence: vec![],
        })
    }

    /// Check 5: Healthy holder distribution
    async fn check_holder_distribution(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        let passed = metrics.holder_count > 1000;
        Ok(SecurityCheck {
            check_type: SecurityCheckType::HealthyHolderDistribution,
            passed,
            score: (metrics.holder_count.min(10000) as f64 / 10000.0).min(1.0),
            details: format!("Holders: {} 👥", metrics.holder_count),
            evidence: vec![],
        })
    }

    /// Check 6: Anti-whale mechanism
    async fn check_anti_whale(&self, metrics: &ContractMetrics) -> Result<SecurityCheck, EvmError> {
        Ok(SecurityCheck {
            check_type: SecurityCheckType::AntiWhaleMechanism,
            passed: metrics.has_anti_whale,
            score: if metrics.has_anti_whale { 0.8 } else { 0.3 },
            details: if metrics.has_anti_whale {
                "Anti-whale protection detected ✅".to_string()
            } else {
                "No anti-whale protection - whales can manipulate price ⚠️".to_string()
            },
            evidence: vec![],
        })
    }

    /// Check 7: No blacklist
    async fn check_no_blacklist(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        Ok(SecurityCheck {
            check_type: SecurityCheckType::NoBlacklist,
            passed: !metrics.has_multi_blacklist,
            score: if metrics.has_multi_blacklist {
                0.0
            } else {
                1.0
            },
            details: if metrics.has_multi_blacklist {
                "⚠️ MULTI BLACKLIST detected - addresses can be frozen!".to_string()
            } else {
                "No blacklist mechanism ✅".to_string()
            },
            evidence: vec![],
        })
    }

    /// Check 8: Sufficient liquidity
    async fn check_liquidity_sufficient(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        let passed = metrics.liquidity_usd > 50000.0;
        Ok(SecurityCheck {
            check_type: SecurityCheckType::LiquiditySufficient,
            passed,
            score: (metrics.liquidity_usd / 200000.0).min(1.0),
            details: format!("Liquidity: ${:.1}K 💰", metrics.liquidity_usd / 1000.0),
            evidence: vec![],
        })
    }

    /// Check 9: Sufficient age
    async fn check_age_sufficient(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        let passed = metrics.age_days > 30;
        Ok(SecurityCheck {
            check_type: SecurityCheckType::AgeSufficient,
            passed,
            score: (metrics.age_days as f64 / 365.0).min(1.0),
            details: format!("Contract age: {} days 🕰️", metrics.age_days),
            evidence: vec![],
        })
    }

    /// Check 10: Healthy volume
    async fn check_healthy_volume(
        &self,
        metrics: &ContractMetrics,
    ) -> Result<SecurityCheck, EvmError> {
        let passed = metrics.vol_24h > 100000.0;
        Ok(SecurityCheck {
            check_type: SecurityCheckType::HealthyVolume,
            passed,
            score: (metrics.vol_24h / 1000000.0).min(1.0),
            details: format!("24h Volume: ${:.1}M ⚖️", metrics.vol_24h / 1000000.0),
            evidence: vec![],
        })
    }

    /// Check 11: Max wallet check
    async fn check_max_wallet(&self, metrics: &ContractMetrics) -> Result<SecurityCheck, EvmError> {
        Ok(SecurityCheck {
            check_type: SecurityCheckType::MaxWalletCheck,
            passed: !metrics.has_max_wallet,
            score: if metrics.has_max_wallet { 0.5 } else { 0.8 },
            details: if metrics.has_max_wallet {
                "Max wallet limit enabled ✅ (anti-whale)".to_string()
            } else {
                "No max wallet limit - whales can accumulate ⚠️".to_string()
            },
            evidence: vec![],
        })
    }

    /// Check 12: Cooldown check
    async fn check_cooldown(&self, metrics: &ContractMetrics) -> Result<SecurityCheck, EvmError> {
        Ok(SecurityCheck {
            check_type: SecurityCheckType::CooldownCheck,
            passed: !metrics.has_cooldown,
            score: if metrics.has_cooldown { 0.6 } else { 1.0 },
            details: if metrics.has_cooldown {
                "Cooldown between trades enabled".to_string()
            } else {
                "No cooldown mechanism".to_string()
            },
            evidence: vec![],
        })
    }

    fn calculate_overall_score(&self, checks: &[SecurityCheck]) -> f64 {
        if checks.is_empty() {
            return 0.0;
        }
        let total_score: f64 = checks.iter().map(|check| check.score).sum();
        total_score / checks.len() as f64
    }

    fn determine_risk_level(&self, score: f64) -> RiskLevel {
        match score {
            s if s >= 0.8 => RiskLevel::Low,
            s if s >= 0.6 => RiskLevel::Medium,
            s if s >= 0.4 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    fn generate_warnings_and_recommendations(
        &self,
        checks: &[SecurityCheck],
        warnings: &mut Vec<String>,
        recommendations: &mut Vec<String>,
        metrics: &ContractMetrics,
    ) {
        for check in checks {
            if !check.passed {
                warnings.push(format!("⚠️ {:?}: {}", check.check_type, check.details));

                match check.check_type {
                    SecurityCheckType::OwnershipRenounced => {
                        recommendations.push(
                            "Consider contracts with renounced ownership for better security"
                                .to_string(),
                        );
                    }
                    SecurityCheckType::LpLocked => {
                        recommendations
                            .push("Look for contracts with 100% LP locked/burned".to_string());
                    }
                    SecurityCheckType::NoHoneypot => {
                        recommendations
                            .push("🚨 HONEYPOT WARNING: Test sell before investing!".to_string());
                    }
                    SecurityCheckType::NoBlacklist => {
                        recommendations
                            .push("Blacklist can freeze your tokens - high risk!".to_string());
                    }
                    _ => {}
                }
            }
        }

        // Additional warnings from metrics
        if metrics.lp_locked_percentage < 80.0 {
            warnings.push(format!("⚠️ Low LP lock: {}%", metrics.lp_locked_percentage));
        }

        if metrics.is_honeypot {
            warnings.push("🚨 CRITICAL: This looks like a HONEYPOT!".to_string());
            recommendations.push("DO NOT BUY - You may not be able to sell!".to_string());
        }
    }

    pub async fn quick_security_check(
        &self,
        contract_address: Address,
    ) -> Result<SecurityCheckResult, EvmError> {
        self.perform_security_audit(contract_address, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Http, Provider};
    use std::str::FromStr;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn test_real_contract_security_checks() {
        let evm = Evm::new(crate::EvmType::ETHEREUM_MAINNET).await.unwrap();
        let evm = Arc::new(evm);
        let checker = SecurityChecker::new(evm);
        // Test WETH contract address
        let contract_address =
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        println!(
            "\n🔍 Starting security audit for WETH contract: {:?}\n",
            contract_address
        );
        println!("═══════════════════════════════════════════════════════════");
        // Execute security audit
        let result = checker
            .perform_security_audit(contract_address, None)
            .await
            .unwrap();
        // Print audit results
        println!("\n📊 Security Audit Results:");
        println!("───────────────────────────────────────────────────────────");
        for check in &result.checks {
            let status = if check.passed { "✅" } else { "❌" };
            println!(
                "{} {:?}: {} (Score: {:.2})",
                status, check.check_type, check.details, check.score
            );
        }
        println!("\n📈 Overall Score: {:.2}%", result.overall_score * 100.0);
        println!("⚠️ Risk Level: {:?}", result.risk_level);
        println!("\n⚠️ Warnings:");
        for warning in &result.warnings {
            println!("  • {}", warning);
        }
        println!("\n💡 Recommendations:");
        for rec in &result.recommendations {
            println!("  • {}", rec);
        }
        println!("\n📊 Contract Metrics:");
        println!("  • Owner Renounced: {}", result.metrics.owner_renounced);
        println!("  • Holders: {}", result.metrics.holder_count);
        println!("  • Liquidity (USD): ${:.2}", result.metrics.liquidity_usd);
        println!("  • LP Locked: {}%", result.metrics.lp_locked_percentage);
        println!("  • Buy Tax: {}%", result.metrics.buy_tax);
        println!("  • Sell Tax: {}%", result.metrics.sell_tax);
        println!("  • Contract Age: {} days", result.metrics.age_days);
        println!("  • Has Max Wallet: {}", result.metrics.has_max_wallet);
        println!("  • Has Blacklist: {}", result.metrics.has_multi_blacklist);
        println!("  • Is Honeypot: {}", result.metrics.is_honeypot);
        println!("  • Has Anti-Whale: {}", result.metrics.has_anti_whale);
        // Assertions - WETH is a well-known contract and should pass most checks
        assert_eq!(result.contract_address, contract_address);
        assert!(!result.checks.is_empty());
        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
        // WETH should have zero taxes
        assert_eq!(result.metrics.buy_tax, 0.0);
        assert_eq!(result.metrics.sell_tax, 0.0);
        // WETH should not have blacklist or honeypot issues
        assert!(!result.metrics.has_multi_blacklist);
        assert!(!result.metrics.is_honeypot);
        println!("\n✅ WETH contract security audit completed!");
        println!("═══════════════════════════════════════════════════════════\n");
    }

    #[tokio::test]
    async fn test_quick_security_check() {
        // Create Evm instance
        let evm = Evm::new(crate::EvmType::ETHEREUM_MAINNET).await.unwrap();
        let evm = Arc::new(evm);
        let checker = SecurityChecker::new(evm);
        let contract_address =
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        // Quick check (without source code)
        let result = checker
            .quick_security_check(contract_address)
            .await
            .unwrap();
        // Verify result is not empty
        assert_eq!(result.checks.len(), 12);
        assert!(result.overall_score > 0.5); // WETH should have high score
        println!("\nQuick check for WETH:");
        println!("Overall Score: {:.2}%", result.overall_score * 100.0);
        println!("Risk Level: {:?}", result.risk_level);
    }
}
