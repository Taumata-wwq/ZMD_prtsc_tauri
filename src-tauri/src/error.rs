use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppError {
    pub message: String,
    pub code: String,
}

impl AppError {
    pub fn new(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: code.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code: {})", self.message, self.code)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::new(e.to_string(), "IO_ERROR")
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::new(e.to_string(), "SQLITE_ERROR")
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::new(e.to_string(), "ANYHOW_ERROR")
    }
}

impl From<image::ImageError> for AppError {
    fn from(e: image::ImageError) -> Self {
        Self::new(e.to_string(), "IMAGE_ERROR")
    }
}

pub type AppResult<T> = Result<T, AppError>;
