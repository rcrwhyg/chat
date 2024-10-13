use anyhow::Result;
use chat_core::{Chat, Message};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgListener;
use tokio_stream::StreamExt;
use tracing::info;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    NewChat(Chat),
    AddToChat(Chat),
    RemoveFromChat(Chat),
    NewMessage(Message),
}

pub async fn setup_pg_listener(state: AppState) -> Result<()> {
    let mut listener = PgListener::connect(&state.config.server.db_url).await?;
    listener.listen("chat_updated").await?;
    listener.listen("chat_message_created").await?;

    let mut stream = listener.into_stream();

    tokio::spawn(async move {
        while let Some(notification) = stream.next().await {
            // TODO: parse notification and send to users
            let _users = &state.users;
            info!("Got notification: {:?}", notification);
        }
    });

    Ok(())
}
