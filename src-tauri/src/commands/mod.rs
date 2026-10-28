//! Typed command layer exposed to the frontend.
//!
//! Every command returns `Result<T, AppError>`; `AppError` carries an Arabic
//! message so the UI can show it verbatim.

pub mod backup;
pub mod drafts;
pub mod excel;
pub mod export;
pub mod invoices;
pub mod products;
pub mod settings;
pub mod update;
