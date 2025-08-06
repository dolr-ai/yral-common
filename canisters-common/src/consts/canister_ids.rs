#[cfg(not(feature = "local"))]
pub use canisters_client::ic::*;
#[cfg(feature = "local")]
pub use canisters_client::local::*;

#[cfg(not(feature = "local"))]
pub static FALLBACK_USER_INDEX: std::sync::LazyLock<candid::Principal> =
    std::sync::LazyLock::new(|| {
        candid::Principal::from_text("rimrc-piaaa-aaaao-aaljq-cai").unwrap()
    });
