use candid::Principal;
use ic_types::{
    messages::{
        HttpCallContent, HttpCanisterUpdate, HttpRequest, HttpRequestEnvelope, HttpRequestError,
        SignedIngressContent,
    },
    CanisterId, PrincipalId,
};
use ic_validator_ingress_message::{
    HttpRequestVerifier, IngressMessageVerifier, RequestValidationError,
};

use crate::{msg_builder::Message, Delegation, Error, Result, Signature, SignedDelegation};

impl Signature {
    fn create_request(self, mut message: Message) -> Result<HttpRequest<SignedIngressContent>> {
        message.ingress_expiry = self.ingress_expiry;
        let envelope = HttpRequestEnvelope {
            content: HttpCallContent::Call {
                update: message.into(),
            },
            sender_pubkey: self.public_key.map(Into::into),
            sender_sig: self.sig.map(Into::into),
            sender_delegation: self
                .delegations
                .map(|sd| sd.into_iter().map(Into::into).collect()),
        };
        Ok(envelope.try_into()?)
    }

    fn verify_request(req: HttpRequest<SignedIngressContent>) -> Result<()> {
        let verifier = IngressMessageVerifier::default();
        Ok(verifier.validate_request(&req)?)
    }

    pub fn verify(self, msg: Message) -> Result<()> {
        let req = self.create_request(msg)?;
        Self::verify_request(req)
    }

    pub fn verify_identity(self, principal: Principal, mut msg: Message) -> Result<()> {
        msg.sender = self.sender;
        let req = self.create_request(msg)?;
        if req.sender().get().0 != principal {
            return Err(Error::IdentityMismatch);
        }
        Self::verify_request(req)
    }
}

impl From<RequestValidationError> for Error {
    fn from(value: RequestValidationError) -> Self {
        Self::SignatureVerification(value.to_string())
    }
}

impl From<HttpRequestError> for Error {
    fn from(value: HttpRequestError) -> Self {
        Self::InvalidMessage(value.to_string())
    }
}

impl From<Delegation> for ic_types::messages::Delegation {
    fn from(value: Delegation) -> Self {
        let pubkey = value.pubkey;
        let expiration = ic_types::Time::from_nanos_since_unix_epoch(value.expiration_ns);
        let Some(y_targets) = value.targets else {
            return Self::new(pubkey, expiration);
        };
        let targets = y_targets
            .into_iter()
            .map(|p| CanisterId::unchecked_from_principal(PrincipalId(p)))
            .collect();

        Self::new_with_targets(pubkey, expiration, targets)
    }
}

impl From<SignedDelegation> for ic_types::messages::SignedDelegation {
    fn from(value: SignedDelegation) -> Self {
        let delegation = value.delegation.into();
        let signature = value.signature;
        Self::new(delegation, signature)
    }
}

impl From<Message> for HttpCanisterUpdate {
    fn from(value: Message) -> Self {
        let ingress_expiry_ns = value
            .ingress_expiry
            .as_nanos()
            .try_into()
            .expect("Ingress expiry overflow");
        Self {
            canister_id: value.canister_id.into(),
            method_name: value.method_name,
            arg: value.args.into(),
            sender: value.sender.into(),
            ingress_expiry: ingress_expiry_ns,
            nonce: value.nonce.map(|n| n.into()),
        }
    }
}