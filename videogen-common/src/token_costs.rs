use crate::TokenType;
use global_constants::{
    LTX2_COST_USD_CENTS, VIDEOGEN_USD_CENTS_TO_DOLR_E8S, VIDEOGEN_USD_CENTS_TO_SATS,
};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Model cost in USD cents (to avoid floating point)
#[derive(Clone, Debug)]
pub struct ModelCostUSD {
    pub usd_cents: u64,
}

impl Default for ModelCostUSD {
    fn default() -> Self {
        Self {
            usd_cents: 50, // 50 cents ($0.5) default
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
            usd_cents_to_sats: VIDEOGEN_USD_CENTS_TO_SATS, // 1 cent = 10 SATS (50 cents = 500 SATS)
            usd_cents_to_dolr: VIDEOGEN_USD_CENTS_TO_DOLR_E8S, // 1 cent = 2 DOLR = 2×10^8 e8s (50 cents = 100 DOLR)
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
        // NOTE: Model costs are now fetched dynamically via API
        // This provides default costs as a fallback
        let mut model_costs_usd = HashMap::new();
        model_costs_usd.insert(
            "ltx2".to_string(),
            ModelCostUSD {
                usd_cents: LTX2_COST_USD_CENTS,
            },
        );

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
    pub fn get_model_cost_usd(&self, model_id: &str) -> u64 {
        self.model_costs_usd
            .get(model_id)
            .map(|cost| cost.usd_cents)
            .unwrap_or(self.default_cost_usd.usd_cents)
    }

    /// Get the cost for a specific model in the requested token type
    pub fn get_model_cost(&self, model_id: &str, token_type: &TokenType) -> u64 {
        let usd_cents = self.get_model_cost_usd(model_id);
        self.convert_usd_to_token(usd_cents, token_type)
    }

    /// Convert USD cents to token amount in smallest unit
    pub fn convert_usd_to_token(&self, usd_cents: u64, token_type: &TokenType) -> u64 {
        match token_type {
            TokenType::Sats => usd_cents * self.conversion_rates.usd_cents_to_sats,
            TokenType::Dolr => usd_cents * self.conversion_rates.usd_cents_to_dolr,
            TokenType::Free => 0,
            TokenType::YralProSubscription => 1,
        }
    }

    /// Convert token amount (in smallest unit) to USD cents
    pub fn convert_token_to_usd(&self, amount: u64, token_type: &TokenType) -> u64 {
        match token_type {
            TokenType::Sats => amount / self.conversion_rates.usd_cents_to_sats,
            TokenType::Dolr => amount / self.conversion_rates.usd_cents_to_dolr,
            TokenType::Free => 0,
            TokenType::YralProSubscription => 0,
        }
    }

    /// Update the cost for a specific model in USD cents
    pub fn set_model_cost_usd(&mut self, model_id: String, usd_cents: u64) {
        self.model_costs_usd
            .insert(model_id, ModelCostUSD { usd_cents });
    }

    /// Update the conversion rates
    pub fn set_conversion_rates(&mut self, rates: TokenConversionRates) {
        self.conversion_rates = rates;
    }
}

// Global static configuration
pub static TOKEN_COST_CONFIG: LazyLock<TokenCostConfig> = LazyLock::new(TokenCostConfig::default);
