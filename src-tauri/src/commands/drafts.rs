//! Draft autosave and crash recovery (FR-08).
//!
//! The editor autosaves its in-progress invoice into the local database with a
//! debounce, so an abnormal termination never loses the work. Drafts never
//! receive a permanent invoice number: they are purely a working copy that the
//! user can restore on the next launch.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::domain::error::AppResult;
use crate::state::AppState;

/// A stored, restorable draft.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredDraft {
    /// The key the editor uses for this draft (e.g. "new" or an invoice id).
    pub key: String,
    /// The editor payload, stored verbatim as JSON.
    pub data: serde_json::Value,
    /// When the draft was last written.
    pub updated_at: String,
}

/// The single row that holds the draft collection.
const DRAFT_ROW_ID: i64 = 1;

/// List every stored draft, newest first.
#[tauri::command]
pub fn list_drafts(app: AppHandle) -> AppResult<Vec<StoredDraft>> {
    let state = app.state::<AppState>();
    state.with_conn(|conn| {
        let raw: Option<String> = conn
            .query_row(
                "SELECT payload_json FROM drafts WHERE id = ?1",
                [DRAFT_ROW_ID],
                |row| row.get(0),
            )
            .ok();
        let mut drafts: Vec<StoredDraft> = match raw {
            Some(text) => serde_json::from_str(&text).unwrap_or_default(),
            None => Vec::new(),
        };
        drafts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(drafts)
    })
}

/// Store (or replace) a draft.
#[tauri::command]
pub fn save_draft(app: AppHandle, key: String, data: serde_json::Value) -> AppResult<StoredDraft> {
    let state = app.state::<AppState>();
    state.with_conn(|conn| {
        let raw: Option<String> = conn
            .query_row(
                "SELECT payload_json FROM drafts WHERE id = ?1",
                [DRAFT_ROW_ID],
                |row| row.get(0),
            )
            .ok();
        let mut drafts: Vec<StoredDraft> = match raw {
            Some(text) => serde_json::from_str(&text).unwrap_or_default(),
            None => Vec::new(),
        };

        let draft = StoredDraft {
            key: key.clone(),
            data,
            updated_at: crate::db::now(),
        };
        drafts.retain(|existing| existing.key != key);
        drafts.push(draft.clone());

        let payload = serde_json::to_string(&drafts)?;
        conn.execute(
            "INSERT INTO drafts(id, payload_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET payload_json = excluded.payload_json,
                 updated_at = excluded.updated_at",
            rusqlite::params![DRAFT_ROW_ID, payload, crate::db::now()],
        )?;
        Ok(draft)
    })
}

/// Remove a stored draft (called once the invoice is saved or discarded).
#[tauri::command]
pub fn delete_draft(app: AppHandle, key: String) -> AppResult<()> {
    let state = app.state::<AppState>();
    state.with_conn(|conn| {
        let raw: Option<String> = conn
            .query_row(
                "SELECT payload_json FROM drafts WHERE id = ?1",
                [DRAFT_ROW_ID],
                |row| row.get(0),
            )
            .ok();
        let mut drafts: Vec<StoredDraft> = match raw {
            Some(text) => serde_json::from_str(&text).unwrap_or_default(),
            None => Vec::new(),
        };
        drafts.retain(|existing| existing.key != key);
        let payload = serde_json::to_string(&drafts)?;
        conn.execute(
            "INSERT INTO drafts(id, payload_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET payload_json = excluded.payload_json,
                 updated_at = excluded.updated_at",
            rusqlite::params![DRAFT_ROW_ID, payload, crate::db::now()],
        )?;
        Ok(())
    })
}
