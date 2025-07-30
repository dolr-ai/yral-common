mod error;
#[cfg(feature = "ic-agent")]
pub mod ic_agent;
#[cfg(feature = "ic-git")]
pub mod ic_git;
pub mod msg_builder;

use candid::Principal;
pub use error::*;

use serde::{Deserialize, Serialize};
use web_time::{Duration, SystemTime};

fn current_epoch() -> Duration {
    web_time::SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
}

/// A signature, interoperable with ic-agent & yral-identity
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct Signature {
    sig: Option<Vec<u8>>,
    public_key: Option<Vec<u8>>,
    ingress_expiry: Duration,
    delegations: Option<Vec<SignedDelegation>>,
    sender: Principal,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
struct Delegation {
    pub pubkey: Vec<u8>,
    pub expiration_ns: u64,
    pub targets: Option<Vec<Principal>>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
struct SignedDelegation {
    pub delegation: Delegation,
    pub signature: Vec<u8>,
}

#[cfg(test)]
mod test {
    use ic_agent::identity::{Identity, Secp256k1Identity};

    use crate::{ic_agent::sign_message, msg_builder::Message};
    use rand::rngs::OsRng;

    #[test]
    fn test_signature_should_verify() {
        let sk = k256::SecretKey::random(&mut OsRng);
        let identity = Secp256k1Identity::from_private_key(sk);

        let msg = Message::default()
            .method_name("test".into())
            .args(("test",))
            .unwrap();
        let sig = sign_message(&identity, msg).unwrap();

        let msg2 = Message::default()
            .method_name("test".into())
            .args(("test",))
            .unwrap();
        sig.verify_identity(identity.sender().unwrap(), msg2)
            .unwrap();
    }
}
