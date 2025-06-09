use crate::utils::token::RootType;

use super::KeyedData;

impl KeyedData for RootType {
    type Key = RootType;

    fn key(&self) -> Self::Key {
        self.clone()
    }
}
