use std::mem;

use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash,
};
use chat_core::ChatUser;
use chat_core::User;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::AppError, AppState};

/// create a user with email and password
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreateUser {
    /// Full name of the user
    pub fullname: String,
    /// Email of the user
    pub email: String,
    /// Workspace name - if not exists, create one
    pub workspace: String,
    /// Password of the user
    pub password: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct SigninUser {
    pub email: String,
    pub password: String,
}

#[allow(dead_code)]
impl AppState {
    /// find a user by email
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user =
            sqlx::query_as("select id,ws_id,fullname,email,created_at from users where email = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;
        Ok(user)
    }

    // find a user by id
    pub async fn find_user_by_id(&self, id: i32) -> Result<Option<User>, AppError> {
        let user =
            sqlx::query_as("select id,ws_id,fullname,email,created_at from users where id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(user)
    }

    /// create a new user
    // TODO: use transaction for workspace creation and user creation
    pub async fn create_user(&self, input: &CreateUser) -> Result<User, AppError> {
        // check if email exists
        let user = self.find_user_by_email(&input.email).await?;
        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }

        // check if workspace exists, if not create one
        let ws = match self.find_workspace_by_name(&input.workspace).await? {
            Some(ws) => ws,
            None => self.create_workspace(&input.workspace, 0).await?,
        };

        let password_hash = hash_password(&input.password)?;
        let mut user: User = sqlx::query_as(
            r#"
        INSERT INTO users (ws_id, email, fullname, password_hash)
        VALUES ($1, $2, $3, $4)
        RETURNING id, ws_id, fullname, email, created_at
        "#,
        )
        .bind(ws.id)
        .bind(&input.email)
        .bind(&input.fullname)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        user.ws_name = ws.name.clone();

        if ws.owner_id == 0 {
            self.update_workspace_owner(ws.id as _, user.id as _)
                .await?;
        }

        Ok(user)
    }

    // Verify email and password
    pub async fn verify_user(&self, input: &SigninUser) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, ws_id, fullname, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(&input.email)
        .fetch_optional(&self.pool)
        .await?;

        match user {
            Some(mut user) => {
                let password_hash = mem::take(&mut user.password_hash);
                let is_valid =
                    verify_password(&input.password, &password_hash.unwrap_or_default())?;
                if is_valid {
                    // load ws_name ,ws should exist
                    let ws = self.find_workspace_by_id(user.ws_id as _).await?.unwrap();
                    user.ws_name = ws.name;
                    Ok(Some(user))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    pub async fn fetch_chat_user_by_ids(&self, ids: &[i64]) -> Result<Vec<User>, AppError> {
        let users = sqlx::query_as(
            "select id,ws_id,fullname,email,created_at from users where id = any($1)",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    pub async fn fetch_chat_users(&self, ws_id: u64) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
        SELECT id, fullname, email
        FROM users
        WHERE ws_id = $1
        "#,
        )
        .bind(ws_id as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);

    // Argon2 with default params (Argon2id v19)
    let argon2 = Argon2::default();

    //Hash password to PHC string ($argon2id$v=19...)
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();

    Ok(password_hash)
}
fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let password_hash = PasswordHash::new(password_hash)?;

    // verify password
    let is_valid = argon2
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok();
    Ok(is_valid)
}

#[cfg(test)]
impl CreateUser {
    pub fn new(ws: &str, fullname: &str, email: &str, password: &str) -> Self {
        Self {
            fullname: fullname.to_string(),
            email: email.to_string(),
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
