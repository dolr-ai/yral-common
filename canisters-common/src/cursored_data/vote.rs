use std::sync::Mutex;

use candid::Principal;
use hon_worker_common::{GameRes, PaginatedGamesReq, PaginatedGamesRes, WORKER_URL};
use url::Url;

use crate::{utils::vote::VoteDetails, Error};

use super::{CursoredDataProvider, KeyedData, PageEntry};

impl KeyedData for VoteDetails {
    type Key = (Principal, u64);

    fn key(&self) -> Self::Key {
        (self.canister_id, self.post_id)
    }
}

impl KeyedData for GameRes {
    type Key = (Principal, u64);

    fn key(&self) -> Self::Key {
        (self.post_canister, self.post_id)
    }
}

/// Only goes forward and ignores start and end parameters when paginating
///
/// UB: Retrieving next page while the current page hasn't finished loading will lead to undefine behavior
pub struct VotesWithSatsProvider {
    // Mutex because we need to track next internally without mut ref.
    next: Mutex<Option<String>>,
    user_principal: Principal,
}

// impl clone by hand because Mutex<T> doesn't impl clone on its own
impl Clone for VotesWithSatsProvider {
    fn clone(&self) -> Self {
        let lock = self.next.lock().unwrap();
        let next = lock.clone();
        let user_principal = self.user_principal;

        Self {
            next: Mutex::new(next),
            user_principal,
        }
    }
}

impl CursoredDataProvider for VotesWithSatsProvider {
    type Data = GameRes;
    type Error = Error;

    async fn get_by_cursor_inner(
        &self,
        start: usize,
        end: usize,
    ) -> Result<PageEntry<Self::Data>, Self::Error> {
        // TODO make lazy static.
        // i tried earlier, but there was some wasm related error?
        // give it another shot in isloation
        let url: Url = WORKER_URL.parse().unwrap();
        let path = format!("/games/{}", self.user_principal);
        let url = url.join(&path).unwrap();
        let cursor = self.get_cursor();
        let req = PaginatedGamesReq {
            page_size: end - start,
            cursor,
        };

        let client = reqwest::Client::new();
        let PaginatedGamesRes { games, next }: PaginatedGamesRes =
            client.post(url).json(&req).send().await?.json().await?;

        let end = next.is_none();

        *self.next.lock().unwrap() = next;

        Ok(PageEntry { data: games, end })
    }
}

impl VotesWithSatsProvider {
    pub fn new(user_principal: Principal) -> Self {
        Self {
            user_principal,
            next: Mutex::new(None),
        }
    }

    fn get_cursor(&self) -> Option<String> {
        self.next.lock().unwrap().clone()
    }
}
