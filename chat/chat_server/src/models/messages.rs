use std::str::FromStr;

use chat_core::Message;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{error::AppError, AppState};

use super::ChatFile;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, IntoParams, ToSchema, Serialize, Deserialize)]
pub struct ListMessages {
    #[serde(default)]
    pub last_id: Option<u64>,
    #[serde(default)]
    pub limit: u64,
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
        // verify files exist
        for s in &input.files {
            let file = ChatFile::from_str(s)?;
            if !file.path(base_dir).exists() {
                return Err(AppError::CreateMessageError(format!(
                    "File {} doesn't exist",
                    s
                )));
            }
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
        .bind(&input.files)
        .fetch_one(&self.pool)
        .await?;

        Ok(message)
    }
}
