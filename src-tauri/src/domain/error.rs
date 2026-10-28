//! Domain-level error type. Every error that can reach the UI carries an
//! Arabic message, because all user-facing feedback must be Arabic (PRD §5).

use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppError {
    /// A business rule was violated (e.g. quantity must be > 0).
    #[error("{0}")]
    Validation(String),

    /// The requested record does not exist.
    #[error("{0}")]
    NotFound(String),

    /// The operation conflicts with current state (e.g. invoice already issued).
    #[error("{0}")]
    Conflict(String),

    /// Storage / database failure.
    #[error("حدث خطأ أثناء الحفظ: {0}")]
    Storage(String),

    /// Import / export failure.
    #[error("حدث خطأ أثناء الاستيراد أو التصدير: {0}")]
    Io(String),

    /// Print / PDF / image export failure.
    #[error("تعذّر إنشاء الملف: {0}")]
    Export(String),

    /// Update subsystem failure.
    #[error("حدث خطأ أثناء التحديث: {0}")]
    Update(String),

    /// Anything unexpected.
    #[error("حدث خطأ غير متوقع: {0}")]
    Internal(String),
}

impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }
    pub fn io(msg: impl Into<String>) -> Self {
        Self::Io(msg.into())
    }
    pub fn export(msg: impl Into<String>) -> Self {
        Self::Export(msg.into())
    }
    pub fn update(msg: impl Into<String>) -> Self {
        Self::Update(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl From<rust_xlsxwriter::XlsxError> for AppError {
    fn from(e: rust_xlsxwriter::XlsxError) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<calamine::Error> for AppError {
    fn from(e: calamine::Error) -> Self {
        Self::Io(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
