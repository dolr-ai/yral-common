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
// GobGob default avatars migrated off Cloudflare Images to self-hosted Hetzner
// (prakash-yral/gobgob/). Objects are gob.<index>.png (public-read). See
// propic_from_principal for how the full URL is built.
pub const GOBGOB_PROPIC_URL: &str = "https://prakash-yral.hel1.your-objectstorage.com/gobgob/gob.";

pub const CKBTC_LEDGER: &str = "mxzaz-hqaaa-aaaar-qaada-cai";
pub const CKBTC_INDEX: &str = "n5wcd-faaaa-aaaar-qaaea-cai";

pub const CKUSDC_LEDGER: &str = "xevnm-gaaaa-aaaar-qafnq-cai";
pub const CKUSDC_INDEX: &str = "xrs4b-hiaaa-aaaar-qafoa-cai";

pub const DOLR_AI_LEDGER_CANISTER: &str = "6rdgd-kyaaa-aaaaq-aaavq-cai";

pub const SUPPORTED_NON_YRAL_TOKENS_ROOT: &[&str] = &["67bll-riaaa-aaaaq-aaauq-cai"];
