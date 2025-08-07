use ic_agent::identity::{DelegatedIdentity, Secp256k1Identity, SignedDelegation};
use k256::elliptic_curve::JwkEcKey;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Delegated identity that can be serialized over the wire
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct DelegatedIdentityWire {
    /// raw bytes of delegated identity's public key
    pub from_key: Vec<u8>,
    /// JWK(JSON Web Key) encoded Secp256k1 secret key
    /// identity allowed to sign on behalf of `from_key`
    #[schema(value_type = Object)]
    pub to_secret: JwkEcKey,
    /// Proof of delegation
    /// connecting from_key to `to_secret`
    #[schema(value_type = Vec<Object>)]
    pub delegation_chain: Vec<SignedDelegation>,
}

impl std::fmt::Debug for DelegatedIdentityWire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelegatedIdentityWire").finish()
    }
}

impl TryFrom<DelegatedIdentityWire> for DelegatedIdentity {
    type Error = k256::elliptic_curve::Error;

    fn try_from(value: DelegatedIdentityWire) -> Result<Self, Self::Error> {
        let to_secret = k256::SecretKey::from_jwk(&value.to_secret)?;
        let to_identity = Secp256k1Identity::from_private_key(to_secret);
        Ok(Self::new_unchecked(
            value.from_key,
            Box::new(to_identity),
            value.delegation_chain,
        ))
    }
}
