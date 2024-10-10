use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::AppError;

use super::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    pub files: Vec<String>,
}

impl Message {
    pub fn create(input: CreateMessage, _pool: &PgPool) -> Result<Message, AppError> {
        // verify content - not empty
        if input.content.is_empty() {
            return Err(AppError::CreateMessageError(
                "Content cannot be empty".to_string(),
            ));
        }

        // verify files exist

        todo!()
    }
}
