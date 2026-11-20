use serde::ser::{Serialize, Serializer};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Msg(String),
    #[error("PASSWORD_REQUIRED: This archive is encrypted. Enter the password used when it was created.")]
    PasswordRequired,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Clipboard error: {0}")]
    Clipboard(String),
}

impl AppError {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Msg(message.into())
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
