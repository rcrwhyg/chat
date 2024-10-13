mod config;
mod error;
mod notify;
mod sse;

use anyhow::Result;
use axum::{
    middleware::from_fn_with_state,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use chat_core::{
    middlewares::{verify_token, TokenVerify},
    DecodingKey, User,
};
use dashmap::DashMap;
use sse::sse_handler;
use std::{ops::Deref, sync::Arc};
use tokio::sync::broadcast;

pub use config::AppConfig;
pub use error::AppError;
pub use notify::{setup_pg_listener, AppEvent};

const INDEX_HTML: &str = include_str!("../index.html");

pub type UserMap = Arc<DashMap<u64, broadcast::Sender<Arc<AppEvent>>>>;

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

pub struct AppStateInner {
    pub config: AppConfig,
    users: UserMap,
    dk: DecodingKey,
}

pub fn get_router() -> (Router, AppState) {
    let config = AppConfig::try_load().expect("Failed to load config");
    let state = AppState::new(config);
    let app = Router::new()
        .route("/events", get(sse_handler))
        .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
        .route("/", get(index_handler))
        .with_state(state.clone());

    (app, state)
}

async fn index_handler() -> impl IntoResponse {
    Html(INDEX_HTML)
}

impl TokenVerify for AppState {
    type Error = AppError;

    fn verify(&self, token: &str) -> Result<User, Self::Error> {
        Ok(self.dk.verify(token)?)
    }
}

impl Deref for AppState {
    type Target = Arc<AppStateInner>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AppState {
    fn new(config: AppConfig) -> Self {
        let dk = DecodingKey::load(&config.auth.pk).expect("Failed to load public key");
        let users = Arc::new(DashMap::new());
        let inner = Arc::new(AppStateInner { config, users, dk });

        Self(inner)
    }
}
