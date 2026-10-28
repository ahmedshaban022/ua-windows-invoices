//! In-app auto-update from GitHub Releases (FR-11).
//!
//! The updater never installs silently: the user is shown the version, the
//! release notes and a progress bar, and the update is only installed after an
//! explicit click. Signature verification is performed by the Tauri updater
//! plugin itself — an invalid signature aborts the download.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::domain::error::{AppError, AppResult};
use crate::state::AppState;

/// The outcome of an update check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    /// The version currently installed.
    pub current_version: String,
    /// True when a newer version is available.
    pub update_available: bool,
    /// The version available for download, when there is one.
    pub latest_version: Option<String>,
    /// The release notes of the available version.
    pub notes: Option<String>,
    /// The release date of the available version.
    pub published_at: Option<String>,
    /// An Arabic message describing the outcome.
    pub message: String,
}

/// The progress of a running download.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes, when the server reports a length.
    pub total: Option<u64>,
}

/// The current application version.
#[tauri::command]
pub fn current_version(app: AppHandle) -> AppResult<String> {
    Ok(app.package_info().version.to_string())
}

/// Check GitHub Releases for a newer version.
///
/// Never fails the application when offline: the error is returned as an
/// Arabic message and the app keeps working.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> AppResult<UpdateStatus> {
    let current = app.package_info().version.to_string();

    let updater = app
        .updater()
        .map_err(|e| AppError::update(format!("تعذّر الاتصال بمصدر التحديثات: {e}")))?;

    let update = updater
        .check()
        .await
        .map_err(|e| AppError::update(format!("تعذّر التحقق من وجود تحديث جديد: {e}")))?;

    match update {
        None => Ok(UpdateStatus {
            current_version: current,
            update_available: false,
            latest_version: None,
            notes: None,
            published_at: None,
            message: "أنت تستخدم أحدث إصدار".to_string(),
        }),
        Some(update) => Ok(UpdateStatus {
            current_version: current,
            update_available: true,
            latest_version: Some(update.version.clone()),
            notes: update.body.clone(),
            published_at: update.date.map(|date| date.to_string()),
            message: format!("يتوفر الإصدار {}", update.version),
        }),
    }
}

/// Download and install the pending update.
///
/// The download runs in the background and reports progress through the
/// `update-progress` event; the user is never interrupted mid-download.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    let updater = app
        .updater()
        .map_err(|e| AppError::update(format!("تعذّر الاتصال بمصدر التحديثات: {e}")))?;

    let update = updater
        .check()
        .await
        .map_err(|e| AppError::update(format!("تعذّر التحقق من وجود تحديث جديد: {e}")))?
        .ok_or_else(|| AppError::update("لا يوجد تحديث متاح للتثبيت"))?;

    let app_handle = app.clone();
    let progress_app = app.clone();

    update
        .download_and_install(
            move |chunk, content_length| {
                let _ = progress_app.emit(
                    "update-progress",
                    UpdateProgress {
                        downloaded: chunk as u64,
                        total: content_length,
                    },
                );
            },
            move || {
                let _ = app_handle.emit("update-ready", ());
            },
        )
        .await
        .map_err(|e| AppError::update(format!("تعذّر تثبيت التحديث: {e}")))?;

    Ok(())
}

/// Remember that the user asked not to be reminded about an update.
#[tauri::command]
pub fn defer_update(app: AppHandle, version: String) -> AppResult<()> {
    let state = app.state::<AppState>();
    state.with_conn(|conn| {
        crate::services::settings::set_value(
            conn,
            "update_deferred_version",
            &serde_json::Value::String(version),
        )
    })
}

/// The version the user asked to be reminded about later, if any.
#[tauri::command]
pub fn deferred_update(app: AppHandle) -> AppResult<Option<String>> {
    let state = app.state::<AppState>();
    state.with_conn(|conn| {
        let value = crate::services::settings::get_value(conn, "update_deferred_version")?;
        Ok(value.and_then(|value| value.as_str().map(|text| text.to_string())))
    })
}
