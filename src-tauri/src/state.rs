//! Shared application state: the database connection and the on-disk paths.
//!
//! The connection lives behind an `Arc<Mutex<..>>` so a restore can swap in a
//! freshly opened database without tearing down the Tauri state object.

use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use rusqlite::Connection;

use crate::db::AppPaths;
use crate::domain::error::AppResult;
use crate::domain::invoice::SellerSnapshot;
use crate::services::brand;
use crate::services::settings::{self, AppSettings};

/// The application's shared state.
pub struct AppState {
    /// The live database handle. Replaced wholesale after a restore.
    pub conn: RwLock<Arc<Mutex<Connection>>>,
    /// Resolved on-disk locations.
    pub paths: AppPaths,
}

impl AppState {
    pub fn new(paths: AppPaths, conn: Connection) -> Self {
        Self {
            conn: RwLock::new(Arc::new(Mutex::new(conn))),
            paths,
        }
    }

    /// Run a closure with the live connection.
    pub fn with_conn<T>(&self, action: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let handle = self.conn.read().clone();
        let guard = handle.lock();
        action(&guard)
    }

    /// Replace the live connection (used after a backup restore).
    pub fn replace_conn(&self, conn: Connection) {
        *self.conn.write() = Arc::new(Mutex::new(conn));
    }

    /// Load the current settings block.
    pub fn settings(&self) -> AppResult<AppSettings> {
        self.with_conn(settings::load)
    }

    /// The currency the invoices are issued in.
    pub fn currency(&self) -> AppResult<crate::domain::money::Currency> {
        Ok(self.settings()?.currency())
    }

    /// The seller identity captured into an invoice when it is issued.
    pub fn seller_snapshot(&self) -> AppResult<SellerSnapshot> {
        let settings = self.settings()?;
        let logo = brand::load_logo(&self.paths);
        Ok(brand::snapshot(&settings, &logo))
    }
}
