use std::collections::HashMap;
use std::sync::LazyLock;
use crate::{TokenType, VideoModel};

/// Model cost in USD cents (to avoid floating point)
#[derive(Clone, Debug)]
pub struct ModelCostUSD {
    pub usd_cents: u64,
}

impl Default for ModelCostUSD {
    fn default() -> Self {
        Self {
            usd_cents: 50, // 50 cents default
        }
    }
}

/// Token conversion rates from USD cents
#[derive(Clone, Debug)]
pub struct TokenConversionRates {
    /// How many SATS per USD cent (in smallest unit)
    pub usd_cents_to_sats: u64,
    /// How many DOLR units (e8s) per USD cent
    pub usd_cents_to_dolr: u64,
}

impl Default for TokenConversionRates {
    fn default() -> Self {
        Self {
            usd_cents_to_sats: 1, // 1 cent = 1 SATS (SATS has 0 decimals)
            usd_cents_to_dolr: 100_000_000, // 1 cent = 1 DOLR = 10^8 smallest units (DOLR has 8 decimals)
        }
    }
}

/// Configuration for model costs and token conversions
#[derive(Clone, Debug)]
pub struct TokenCostConfig {
    /// Model costs in USD cents by model name
    model_costs_usd: HashMap<String, ModelCostUSD>,
    /// Token conversion rates
    conversion_rates: TokenConversionRates,
    /// Default cost if model not found
    default_cost_usd: ModelCostUSD,
}

impl Default for TokenCostConfig {
    fn default() -> Self {
        let mut model_costs_usd = HashMap::new();
        
        // Load costs from VideoModel definitions
        for model in VideoModel::get_models() {
            model_costs_usd.insert(
                model.id.clone(),
                ModelCostUSD {
                    usd_cents: model.cost_usd_cents,
                },
            );
        }
        
        // Default cost for models not in the list
        let default_cost = ModelCostUSD::default();
        
        Self {
            model_costs_usd,
            conversion_rates: TokenConversionRates::default(),
            default_cost_usd: default_cost,
        }
    }
}

impl TokenCostConfig {
    /// Get the cost for a specific model in USD cents
    pub fn get_model_cost_usd(&self, model_name: &str) -> u64 {
        self.model_costs_usd
            .get(model_name)
            .map(|cost| cost.usd_cents)
            .unwrap_or(self.default_cost_usd.usd_cents)
    }
    
    /// Get the cost for a specific model in the requested token type
    pub fn get_model_cost(&self, model_name: &str, token_type: &TokenType) -> u64 {
        let usd_cents = self.get_model_cost_usd(model_name);
        self.convert_usd_to_token(usd_cents, token_type)
    }
    
    /// Convert USD cents to token amount in smallest unit
    pub fn convert_usd_to_token(&self, usd_cents: u64, token_type: &TokenType) -> u64 {
        match token_type {
            TokenType::Sats => usd_cents * self.conversion_rates.usd_cents_to_sats,
            TokenType::Dolr => usd_cents * self.conversion_rates.usd_cents_to_dolr,
        }
    }
    
    /// Convert token amount (in smallest unit) to USD cents
    pub fn convert_token_to_usd(&self, amount: u64, token_type: &TokenType) -> u64 {
        match token_type {
            TokenType::Sats => amount / self.conversion_rates.usd_cents_to_sats,
            TokenType::Dolr => amount / self.conversion_rates.usd_cents_to_dolr,
        }
    }
    
    /// Update the cost for a specific model in USD cents
    pub fn set_model_cost_usd(&mut self, model_name: String, usd_cents: u64) {
        self.model_costs_usd.insert(model_name, ModelCostUSD { usd_cents });
    }
    
    /// Update the conversion rates
    pub fn set_conversion_rates(&mut self, rates: TokenConversionRates) {
        self.conversion_rates = rates;
    }
}

// Global static configuration
pub static TOKEN_COST_CONFIG: LazyLock<TokenCostConfig> = LazyLock::new(TokenCostConfig::default);