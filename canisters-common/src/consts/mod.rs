#[cfg(feature = "local")]
mod local;
#[cfg(not(feature = "local"))]
mod remote;

#[cfg(feature = "local")]
pub use local::*;
#[cfg(not(feature = "local"))]
pub use remote::*;

pub mod canister_ids;

pub const CENTS_IN_E6S: u64 = 1_000_000;
pub const GOBGOB_TOTAL_COUNT: u32 = 18557;
pub const GOBGOB_PROPIC_URL: &str = "https://imagedelivery.net/abXI9nS4DYYtyR1yFFtziA/gob.";

pub const CDAO_SWAP_PRE_READY_TIME_SECS: u64 = 150;
pub const CDAO_SWAP_TIME_SECS: u64 = CDAO_SWAP_PRE_READY_TIME_SECS + 150;

pub const CKBTC_LEDGER: &str = "mxzaz-hqaaa-aaaar-qaada-cai";
pub const CKBTC_INDEX: &str = "n5wcd-faaaa-aaaar-qaaea-cai";

pub const CKUSDC_LEDGER: &str = "xevnm-gaaaa-aaaar-qafnq-cai";
pub const CKUSDC_INDEX: &str = "xrs4b-hiaaa-aaaar-qafoa-cai";

pub const SUPPORTED_NON_YRAL_TOKENS_ROOT: &[&str] = &["67bll-riaaa-aaaaq-aaauq-cai"];
