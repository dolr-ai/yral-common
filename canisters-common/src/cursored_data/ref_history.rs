use candid::Principal;


use super::{ KeyedData };

#[derive(Clone, Copy)]
pub struct HistoryDetails {
    pub epoch_secs: u64,
    pub referee: Principal,
    pub amount: u64,
}

impl KeyedData for HistoryDetails {
    type Key = Principal;

    fn key(&self) -> Self::Key {
        self.referee
    }
}


