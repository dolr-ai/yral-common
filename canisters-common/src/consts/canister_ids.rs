#[cfg(not(feature = "local"))]
pub use canisters_client::ic::*;
#[cfg(feature = "local")]
pub use canisters_client::local::*;
