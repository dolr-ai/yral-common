use web_time::Duration;

use candid::{encode_args, utils::ArgumentEncoder, Principal};

use crate::{current_epoch, Result};

/// Signable Message
#[derive(Clone, Debug)]
pub struct Message {
    pub(crate) canister_id: Principal,
    pub(crate) method_name: String,
    pub(crate) args: Vec<u8>,
    #[allow(dead_code)]
    pub(crate) sender: Principal,
    pub(crate) ingress_expiry: Duration,
    pub(crate) nonce: Option<Vec<u8>>,
}

impl Message {
    /// Canister Id of the message receiver
    /// default: anonymous
    pub fn canister_id(mut self, canister_id: Principal) -> Self {
        self.canister_id = canister_id;
        self
    }

    /// Method name to call on the receiver
    /// default: empty string
    pub fn method_name(mut self, method_name: String) -> Self {
        self.method_name = method_name;
        self
    }

    /// Arguments to pass to the receiver
    /// default: empty
    pub fn args<Args: ArgumentEncoder>(mut self, args: Args) -> Result<Self> {
        self.args = encode_args(args)?;
        Ok(self)
    }

    /// How long the message is valid for from the current time
    /// Note: max_age is added to the current time to get the ingress_expiry
    /// default: 120 seconds
    pub fn ingress_max_age(mut self, max_age: Duration) -> Self {
        self.ingress_expiry = current_epoch() + max_age;
        self
    }

    /// Optional Nonce of the message
    /// note: its the receiver's responsibility to ensure the nonce is unique
    /// default: None
    pub fn nonce(mut self, nonce: Vec<u8>) -> Self {
        self.nonce = Some(nonce);
        self
    }
}

impl Default for Message {
    fn default() -> Self {
        Self {
            canister_id: Principal::anonymous(),
            method_name: String::new(),
            args: vec![],
            nonce: None,
            sender: Principal::anonymous(),
            ingress_expiry: current_epoch() + Duration::from_secs(120),
        }
    }
}