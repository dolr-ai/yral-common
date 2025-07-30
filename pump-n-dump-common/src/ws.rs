use candid::{Nat, Principal};
use identity::msg_builder::Message;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{rest::UserBetsResponse, GameDirection};

/// Types of request that worker can handle
#[derive(Serialize, Deserialize)]
pub enum WsMessage {
    Bet {
        direction: GameDirection,
        round: u64,
    },
}

/// A complete request that the worker expects
/// for a given request_id, a corresponding [`WsResponse`]
/// is expected to be produced with the same request_id
#[derive(Serialize, Deserialize)]
pub struct WsRequest {
    pub request_id: Uuid,
    pub msg: WsMessage,
}

/// Result for a game
#[derive(Serialize, Deserialize, Clone)]
pub struct GameResult {
    pub direction: GameDirection,
    pub reward_pool: Nat,
    pub bet_count: u64,
    pub new_round: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WsError {
    Generic(String),
    BetFailure {
        message: String,
        direction: GameDirection,
    },
}

/// Types of responses from the worker
#[derive(Serialize, Deserialize, Clone)]
pub enum WsResp {
    // Request was succesful
    BetSuccesful {
        // It is still possible for the request to roll over
        // to the next round in extreme cases
        round: u64,
    },
    // Request Failed
    Error(WsError),
    // Event - Game ended
    GameResultEvent(GameResult),
    // Event - Winning Pool Changed
    WinningPoolEvent {
        new_pool: u64,
        round: u64,
    },
    // Event - The current in-game state
    // sent when ws connection
    // is estabilished
    WelcomeEvent {
        round: u64,
        pool: u64,
        player_count: u64,
        user_bets: UserBetsResponse,
    },
}

impl WsResp {
    pub fn error(e: impl Into<String>) -> Self {
        Self::Error(WsError::Generic(e.into()))
    }

    pub fn bet_failure(e: impl Into<String>, direction: GameDirection) -> Self {
        Self::Error(WsError::BetFailure {
            message: e.into(),
            direction,
        })
    }
}

/// A complete response from the worker
/// this can be an event or a response to some previous request
/// from the client
#[derive(Serialize, Deserialize)]
pub struct WsResponse {
    pub request_id: Uuid,
    pub response: WsResp,
}

impl WsResponse {
    pub fn is_event(&self) -> bool {
        self.request_id == Uuid::max()
    }
}

pub fn identify_message(game_canister: Principal, token_root: Principal) -> Message {
    Message::default()
        .method_name("pump_or_dump_worker".into())
        .args((game_canister, token_root))
        .expect("identify request should serialize")
}

/// Create a connection url for a new game
/// caller must ensure `base_url` ends with a slash (`/`)
/// e.g "https://example.com/"
#[cfg(feature = "client")]
pub fn websocket_connection_url(
    base_url: url::Url,
    sender: &impl ic_agent::Identity,
    token_creator_canister: candid::Principal,
    token_root: candid::Principal,
) -> Result<url::Url, String> {
    use identity::ic_agent::sign_message;

    let path = format!("ws/{token_creator_canister}/{token_root}");
    let mut new_url = base_url.join(&path).map_err(|e| e.to_string())?;

    let msg = identify_message(token_creator_canister, token_root);
    let sig = sign_message(sender, msg).map_err(|e| e.to_string())?;
    let sig_json = serde_json::to_string(&sig).expect("signature should serialize");

    let sender_id = sender.sender().expect("sender should have principal");

    new_url
        .query_pairs_mut()
        .append_pair("sender", &sender_id.to_text())
        .append_pair("signature", &sig_json);

    Ok(new_url)
}
