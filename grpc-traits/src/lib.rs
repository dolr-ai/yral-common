use std::future::Future;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenListItemFS {
    pub user_id: String,
    pub name: String,
    pub token_name: String,
    pub token_symbol: String,
    pub logo: String,
    pub description: String,
    pub created_at: String,
    #[serde(default)]
    pub link: String,
}

pub trait TokenInfoProvider {
    type Error;

    fn get_token_by_id(
        &self,
        token_id: String,
    ) -> impl Future<Output = Result<TokenListItemFS, Self::Error>> + Send;
}
