use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::AppError;

use super::{Chat, ChatType, ChatUser};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChat {
    pub r#type: ChatType,
    pub name: Option<String>,
    pub members: Vec<i64>,
}

#[allow(dead_code)]
impl Chat {
    pub async fn create(input: CreateChat, ws_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        let len = input.members.len();
        if len < 2 {
            return Err(AppError::CreateChatError(format!(
                "Members must be at least 2, but got {}",
                len
            )));
        }
        if len > 8 && input.name.is_none() {
            return Err(AppError::CreateChatError(
                "Group chat with more than 8 members must have a name".to_string(),
            ));
        }

        // verify if all members exist
        let users = ChatUser::fetch_by_ids(&input.members, pool).await?;
        if users.len() != len {
            return Err(AppError::CreateChatError(
                "Some of the members do not exist".to_string(),
            ));
        }

        let chat_type = match (&input.name, len) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };

        let chat = sqlx::query_as(
            r#"
            INSERT INTO chats (ws_id, name, type, members)
            VALUES ($1, $2, $3, $4)
            RETURNING id, ws_id, name, type, members, created_at
            "#,
        )
        .bind(ws_id as i64)
        .bind(input.name)
        .bind(chat_type)
        .bind(input.members)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_all(ws_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let chats = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE ws_id = $1
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let chat = sqlx::query_as(
            r#"
            SELECT id, ws_id, name, type, members, created_at
            FROM chats
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;

        Ok(chat)
    }

    pub async fn update_by_id(id: u64, input: UpdateChat, pool: &PgPool) -> Result<Self, AppError> {
        let len = input.members.len();

        if len < 2 {
            return Err(AppError::UpdateChatError(format!(
                "Members must be at least 2, but got {}",
                len
            )));
        }
        if len > 8 && input.name.is_none() {
            return Err(AppError::UpdateChatError(
                "Group chat with more than 8 members must have a name".to_string(),
            ));
        }

        if input.r#type == ChatType::Single && input.members.len() != 2 {
            return Err(AppError::UpdateChatError(
                "Chat type cannot be changed for [single] with {len} members (must 2)".to_string(),
            ));
        }

        // verify if all members exist
        let users = ChatUser::fetch_by_ids(&input.members, pool).await?;
        if users.len() != len {
            return Err(AppError::UpdateChatError(
                "Some of the members do not exist".to_string(),
            ));
        }

        let chat = sqlx::query_as(
            r#"
            UPDATE chats
            SET type = $1, name = $2, members = $3
            WHERE id = $4
            RETURNING id, ws_id, name, type, members, created_at
            "#,
        )
        .bind(input.r#type)
        .bind(input.name)
        .bind(input.members)
        .bind(id as i64)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }

    pub async fn delete_by_id(id: u64, pool: &PgPool) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM chats
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
impl CreateChat {
    pub fn new(name: &str, members: &[i64], public: bool) -> Self {
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
impl UpdateChat {
    pub fn new(r#type: ChatType, name: &str, members: &[i64]) -> Self {
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };
        Self {
            r#type,
            name,
            members: members.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::get_test_pool;

    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_create_single_chat_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateChat::new("", &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("Failed to create chat");

        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 2);
        assert_eq!(chat.r#type, ChatType::Single);

        Ok(())
    }

    #[tokio::test]
    async fn test_create_public_named_chat_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateChat::new("general", &[1, 2, 3, 4], true);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("Failed to create chat");

        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 4);
        assert_eq!(chat.r#type, ChatType::PublicChannel);

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_get_by_id_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let chat = Chat::get_by_id(1, &pool)
            .await
            .expect("Failed to get chat by id")
            .unwrap();

        assert_eq!(chat.id, 1);
        assert_eq!(chat.name.unwrap(), "general");
        assert_eq!(chat.ws_id, 1);
        assert_eq!(chat.members.len(), 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_fetch_all_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let chats = Chat::fetch_all(1, &pool)
            .await
            .expect("Failed to fetch all chats");

        assert_eq!(chats.len(), 4);

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_update_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateChat::new("test_update_single", &[1, 2], false);
        let chat1 = Chat::create(input, 1, &pool)
            .await
            .expect("Failed to create chat");

        let update = UpdateChat::new(ChatType::Group, "test_update_group", &[1, 2, 3]);
        let chat2 = Chat::update_by_id(chat1.id as _, update, &pool).await?;

        assert_eq!(chat1.id, chat2.id);
        assert_eq!(chat2.name.unwrap(), "test_update_group");
        assert_eq!(chat2.members.len(), 3);

        let update = UpdateChat::new(
            ChatType::PublicChannel,
            "test_update_public_channel",
            &[1, 2, 3, 4],
        );
        let chat3 = Chat::update_by_id(chat1.id as _, update, &pool).await?;

        assert_eq!(chat1.id, chat3.id);
        assert_eq!(chat3.name.unwrap(), "test_update_public_channel");
        assert_eq!(chat3.members.len(), 4);

        Ok(())
    }

    #[tokio::test]
    async fn test_chat_delete_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let input = CreateChat::new("test_delete", &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("Failed to create chat");

        Chat::delete_by_id(chat.id as _, &pool).await?;

        assert!(Chat::get_by_id(chat.id as _, &pool).await?.is_none());

        Ok(())
    }
}
