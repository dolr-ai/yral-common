use candid::Principal;

use crate::utils::posts::PostDetails;

use super::KeyedData;

impl KeyedData for PostDetails {
    type Key = (Principal, String);

    fn key(&self) -> Self::Key {
        (self.canister_id, self.post_id.clone())
    }
}
