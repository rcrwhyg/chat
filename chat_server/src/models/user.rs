use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, PasswordHash,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{AppError, User};

use super::{ChatUser, Workspace};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub full_name: String,
    pub email: String,
    pub workspace: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigninUser {
    pub email: String,
    pub password: String,
}

impl User {
    /// Find a user by email
    pub async fn find_by_email(email: &str, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as(
            "SELECT id, ws_id, full_name, email, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// Create a new user
    // TODO: use transaction for workspace creation and user creation
    pub async fn create(input: &CreateUser, pool: &PgPool) -> Result<Self, AppError> {
        // check if email exists
        let user = Self::find_by_email(&input.email, pool).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }

        // check if workspace exists, if not create one
        let ws = match Workspace::find_by_name(&input.workspace, pool).await? {
            Some(ws) => ws,
            None => Workspace::create(&input.workspace, 0, pool).await?,
        };

        let password_hash = hash_password(&input.password)?;
        let user: User = sqlx::query_as(
            r#"
            INSERT INTO users (ws_id, email, full_name, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING id, ws_id, full_name, email, created_at
            "#,
        )
        .bind(ws.id)
        .bind(&input.email)
        .bind(&input.full_name)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;

        if ws.owner_id == 0 {
            ws.update_owner(user.id as _, pool).await?;
        }

        Ok(user)
    }

    /// add user to workspace
    pub async fn add_to_workspace(&self, ws_id: i64, pool: &PgPool) -> Result<User, AppError> {
        let user = sqlx::query_as(
            r#"
            UPDATE users
            SET ws_id = $1
            WHERE id = $2 and ws_id = 0
            RETURNING id, ws_id, full_name, email, created_at
            "#,
        )
        .bind(ws_id)
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// Verify email and password
    pub async fn verify(input: &SigninUser, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, ws_id, full_name, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(&input.email)
        .fetch_optional(pool)
        .await?;

        match user {
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let is_valid =
                    verify_password(&input.password, &password_hash.unwrap_or_default())?;
                if is_valid {
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }
}

#[allow(dead_code)]
impl ChatUser {
    pub async fn fetch_by_ids(ids: &[i64], pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let users = sqlx::query_as(
            r#"
            SELECT id, full_name, email
            FROM users
            WHERE id = ANY($1)
            "#,
        )
        .bind(ids)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    pub async fn fetch_all(ws_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let users = sqlx::query_as(
            r#"
            SELECT id, full_name, email
            FROM users
            WHERE ws_id = $1
            "#,
        )
        .bind(ws_id as i64)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    // Hash password to PHC string ($argon2id$v=19$...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}

fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(password_hash)?;

    // Verify password against PHC string
    let is_valid = argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();

    Ok(is_valid)
}

#[cfg(test)]
impl User {
    pub fn new(id: i64, full_name: String, email: String) -> Self {
        use chrono::Utc;
        Self {
            id,
            ws_id: 0,
            full_name,
            email,
            password_hash: None,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
impl CreateUser {
    pub fn new(ws: &str, email: &str, full_name: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            full_name: full_name.to_string(),
            workspace: ws.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
impl SigninUser {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::Result;

    use crate::test_util::get_test_pool;

    #[test]
    fn test_hash_password_and_verify_should_work() -> Result<()> {
        let password = "hunter42";
        let password_hash = hash_password(password)?;
        assert_eq!(password_hash.len(), 97);
        assert!(verify_password(password, &password_hash)?);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_duplicate_user_should_fail() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let email = "tchen@acme.org";
        let full_name = "Tyr Chen";
        let password = "hunter42";
        let input = CreateUser::new("Default Workspace", email, full_name, password);

        let ret = User::create(&input, &pool).await;
        match ret {
            Err(AppError::EmailAlreadyExists(email)) => {
                assert_eq!(email, input.email);
            }
            _ => {
                panic!("Expecting EmailAlreadyExists error");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_create_and_verify_user_should_work() -> Result<()> {
        let (_tdb, pool) = get_test_pool(None).await;

        let email = "rcrwhyg@sina.com";
        let full_name = "Lyn Wong";
        let password = "hunter42";
        let input = CreateUser::new("Default Workspace", email, full_name, password);
        let user = User::create(&input, &pool).await?;
        assert_eq!(user.email, email);
        assert_eq!(user.full_name, full_name);
        assert!(user.id > 0);

        let user = User::find_by_email(email, &pool).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, email);
        assert_eq!(user.full_name, full_name);

        let input = SigninUser::new(email, password);
        assert!(User::verify(&input, &pool).await?.is_some());

        Ok(())
    }
}
