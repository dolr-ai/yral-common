use ic_agent::{Identity, agent::EnvelopeContent};

use crate::{Delegation, Error, Result, Signature, SignedDelegation, msg_builder::Message};

pub fn sign_message(identity: &impl Identity, mut msg: Message) -> Result<Signature> {
    let sender = identity.sender().map_err(|_| Error::SenderNotFound)?;
    msg.sender = sender;
    let ingress_expiry = msg.ingress_expiry;
    let sig_agent = identity.sign(&msg.into()).map_err(Error::Signing)?;
    Ok(Signature {
        sig: sig_agent.signature,
        public_key: identity.public_key(),
        ingress_expiry,
        sender,
        delegations: sig_agent
            .delegations
            .map(|v| v.into_iter().map(Into::into).collect()),
    })
}

impl From<Delegation> for ic_agent::identity::Delegation {
    fn from(value: Delegation) -> Self {
        Self {
            pubkey: value.pubkey,
            expiration: value.expiration_ns,
            targets: value.targets,
        }
    }
}

impl From<SignedDelegation> for ic_agent::identity::SignedDelegation {
    fn from(value: SignedDelegation) -> Self {
        Self {
            delegation: value.delegation.into(),
            signature: value.signature,
        }
    }
}

impl From<ic_agent::identity::Delegation> for Delegation {
    fn from(value: ic_agent::identity::Delegation) -> Self {
        Self {
            pubkey: value.pubkey,
            expiration_ns: value.expiration,
            targets: value.targets,
        }
    }
}

impl From<ic_agent::identity::SignedDelegation> for SignedDelegation {
    fn from(value: ic_agent::identity::SignedDelegation) -> Self {
        Self {
            delegation: value.delegation.into(),
            signature: value.signature,
        }
    }
}

impl From<Message> for EnvelopeContent {
    fn from(value: Message) -> Self {
        let ingress_expiry_ns = value
            .ingress_expiry
            .as_nanos()
            .try_into()
            .expect("Ingress expiry overflow");
        EnvelopeContent::Call {
            canister_id: value.canister_id,
            method_name: value.method_name,
            arg: value.args,
            sender: value.sender,
            nonce: value.nonce,
            ingress_expiry: ingress_expiry_ns,
        }
    }
}
