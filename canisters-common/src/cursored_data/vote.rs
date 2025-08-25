use std::sync::Mutex;

use candid::Principal;
use hon_worker_common::{
    GameRes, GameResV2, GameResV3, GameResV4WithCanister, PaginatedGamesReq, PaginatedGamesRes,
    PaginatedGamesResV2, PaginatedGamesResV3, PaginatedGamesResV4, WORKER_URL,
};
use url::Url;
use yral_metadata_client::MetadataClient;

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

impl KeyedData for GameResV4WithCanister {
    type Key = (Principal, String);

    fn key(&self) -> Self::Key {
        (self.post_creator_canister, self.post_id.clone())
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

impl KeyedData for GameResV2 {
    type Key = (Principal, u64);

    fn key(&self) -> Self::Key {
        (self.post_canister, self.post_id)
    }
}

impl KeyedData for GameResV3 {
    type Key = (Principal, u64);

    fn key(&self) -> Self::Key {
        (self.publisher_principal, self.post_id)
    }
}

/// Only goes forward and ignores start and end parameters when paginating
///
/// UB: Retrieving next page while the current page hasn't finished loading will lead to undefine behavior
pub struct VotesWithSatsProviderV2 {
    // Mutex because we need to track next internally without mut ref.
    next: Mutex<Option<String>>,
    user_principal: Principal,
}

// impl clone by hand because Mutex<T> doesn't impl clone on its own
impl Clone for VotesWithSatsProviderV2 {
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

impl CursoredDataProvider for VotesWithSatsProviderV2 {
    type Data = GameResV2;
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
        let PaginatedGamesResV2 { games, next }: PaginatedGamesResV2 =
            client.post(url).json(&req).send().await?.json().await?;

        let end = next.is_none();

        *self.next.lock().unwrap() = next;

        Ok(PageEntry { data: games, end })
    }
}

impl VotesWithSatsProviderV2 {
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

/// VotesWithSatsProviderV3 calls the /v3 API and converts GameResV3 to GameRes using metadata client
pub struct VotesWithSatsProviderV3 {
    next: Mutex<Option<String>>,
    user_principal: Principal,
    metadata_client: MetadataClient<false>,
}

impl Clone for VotesWithSatsProviderV3 {
    fn clone(&self) -> Self {
        let lock = self.next.lock().unwrap();
        let next = lock.clone();
        let user_principal = self.user_principal;
        let metadata_client = self.metadata_client.clone();

        Self {
            next: Mutex::new(next),
            user_principal,
            metadata_client,
        }
    }
}

impl CursoredDataProvider for VotesWithSatsProviderV3 {
    type Data = GameRes;
    type Error = Error;

    async fn get_by_cursor_inner(
        &self,
        start: usize,
        end: usize,
    ) -> Result<PageEntry<Self::Data>, Self::Error> {
        let url: Url = WORKER_URL.parse().unwrap();
        let path = format!("/v3/games/{}", self.user_principal);
        let url = url.join(&path).unwrap();
        let cursor = self.get_cursor();
        let req = PaginatedGamesReq {
            page_size: end - start,
            cursor,
        };

        let client = reqwest::Client::new();
        let PaginatedGamesResV3 { games, next }: PaginatedGamesResV3 =
            client.post(url).json(&req).send().await?.json().await?;

        let end = next.is_none();
        *self.next.lock().unwrap() = next;

        // Convert GameResV3 to GameRes using metadata client
        let mut converted_games = Vec::new();
        let publisher_principals: Vec<Principal> =
            games.iter().map(|g| g.publisher_principal).collect();

        // Get canister mappings in bulk
        let canister_mappings = self
            .metadata_client
            .get_user_metadata_bulk(publisher_principals)
            .await?;

        for game_v3 in games {
            if let Some(Some(metadata)) = canister_mappings.get(&game_v3.publisher_principal) {
                let game_res = GameRes {
                    post_canister: metadata.user_canister_id,
                    post_id: game_v3.post_id,
                    game_info: game_v3.game_info,
                };
                converted_games.push(game_res);
            }
        }

        Ok(PageEntry {
            data: converted_games,
            end,
        })
    }
}

pub struct VotesWithSatsProviderV4 {
    next: Mutex<Option<String>>,
    user_principal: Principal,
    metadata_client: MetadataClient<false>,
}

impl VotesWithSatsProviderV4 {
    pub fn new(user_principal: Principal, metadata_client: MetadataClient<false>) -> Self {
        Self {
            user_principal,
            metadata_client,
            next: Mutex::new(None),
        }
    }

    fn get_cursor(&self) -> Option<String> {
        self.next.lock().unwrap().clone()
    }
}

impl Clone for VotesWithSatsProviderV4 {
    fn clone(&self) -> Self {
        let lock = self.next.lock().unwrap();
        let next = lock.clone();
        let user_principal = self.user_principal;
        let metadata_client = self.metadata_client.clone();

        Self {
            next: Mutex::new(next),
            user_principal,
            metadata_client,
        }
    }
}

impl CursoredDataProvider for VotesWithSatsProviderV4 {
    type Data = GameResV4WithCanister;
    type Error = Error;

    async fn get_by_cursor_inner(
        &self,
        start: usize,
        end: usize,
    ) -> Result<PageEntry<Self::Data>, Self::Error> {
        let url: Url = WORKER_URL.parse().unwrap();
        let path = format!("/v4/games/{}", self.user_principal);
        let url = url.join(&path).unwrap();
        let cursor = self.get_cursor();
        let req = PaginatedGamesReq {
            page_size: end - start,
            cursor,
        };

        let client = reqwest::Client::new();
        let PaginatedGamesResV4 { games, next }: PaginatedGamesResV4 =
            client.post(url).json(&req).send().await?.json().await?;

        let end = next.is_none();
        *self.next.lock().unwrap() = next;

        // Convert GameResV3 to GameRes using metadata client
        let mut converted_games = Vec::new();
        let publisher_principals: Vec<Principal> =
            games.iter().map(|g| g.publisher_principal).collect();

        // Get canister mappings in bulk
        let canister_mappings = self
            .metadata_client
            .get_user_metadata_bulk(publisher_principals)
            .await?;

        for game_v4 in games {
            if let Some(Some(metadata)) = canister_mappings.get(&game_v4.publisher_principal) {
                let game_res = GameResV4WithCanister {
                    post_creator_canister: metadata.user_canister_id,
                    post_id: game_v4.post_id.clone(),
                    game_info: game_v4.game_info,
                };
                converted_games.push(game_res);
            }
        }

        Ok(PageEntry {
            data: converted_games,
            end,
        })
    }
}

impl VotesWithSatsProviderV3 {
    pub fn new(user_principal: Principal, metadata_client: MetadataClient<false>) -> Self {
        Self {
            user_principal,
            metadata_client,
            next: Mutex::new(None),
        }
    }

    fn get_cursor(&self) -> Option<String> {
        self.next.lock().unwrap().clone()
    }
}
