use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{AppError, AppState, ChatFile};

use super::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    pub files: Vec<String>,
}

#[allow(dead_code)]
impl AppState {
    pub async fn create_message(
        &self,
        input: CreateMessage,
        chat_id: u64,
        user_id: u64,
    ) -> Result<Message, AppError> {
        let base_dir = &self.config.server.base_dir;
        // verify content - not empty
        if input.content.is_empty() {
            return Err(AppError::CreateMessageError(
                "Content cannot be empty".to_string(),
            ));
        }

        // verify files exist
        for s in &input.files {
            let file = ChatFile::from_str(s)?;
            if !file.path(base_dir).exists() {
                return Err(AppError::CreateMessageError(format!(
                    "File {} not found",
                    s
                )));
            }
        }

        // verify if user_id is a member of chat_id
        if !self.is_chat_member(chat_id, user_id).await? {
            return Err(AppError::CreateMessageError(format!(
                "User {} is not a member of chat {}",
                user_id, chat_id
            )));
        }

        // create message
        let message: Message = sqlx::query_as(
            r#"
            INSERT INTO messages (chat_id, sender_id, content, files)
            VALUES ($1, $2, $3, $4)
            RETURNING id, chat_id, sender_id, content, files, created_at
            "#,
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .bind(input.content)
        .bind(input.files)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_create_message_should_work() -> Result<()> {
        let (_tdb, state) = AppState::try_new_for_test().await?;

        let input = CreateMessage {
            content: "Hello World".to_string(),
            files: vec![],
        };

        let message = state
            .create_message(input, 1, 1)
            .await
            .expect("create message failed");
        assert_eq!(message.content, "Hello World");

        // invalid files should fail
        let input = CreateMessage {
            content: "Hello World".to_string(),
            files: vec!["invalid_file".to_string()],
        };
        assert!(state.create_message(input, 1, 1).await.is_err());

        // invalid files should work
        let url = upload_dummy_file(&state)?;
        let input = CreateMessage {
            content: "Hello World".to_string(),
            files: vec![url],
        };
        let message = state
            .create_message(input, 1, 1)
            .await
            .expect("create message failed");
        assert_eq!(message.content, "Hello World");
        assert_eq!(message.files.len(), 1);

        Ok(())
    }

    fn upload_dummy_file(state: &AppState) -> Result<String> {
        let file = ChatFile::new(1, "dummy.txt", b"Hello World");
        let file_path = file.path(&state.config.server.base_dir);
        std::fs::create_dir_all(file_path.parent().expect("parent dir should exists"))?;
        std::fs::write(&file_path, b"Hello World")?;

        Ok(file.url())
    }
}
