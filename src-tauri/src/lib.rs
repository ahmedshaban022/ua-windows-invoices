//! Application wiring: state, plugins, windows and the command handler.

pub mod commands;
pub mod db;
pub mod domain;
pub mod export;
pub mod services;
pub mod state;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::commands::export::RENDER_HOST_LABEL;
use crate::db::AppPaths;
use crate::state::AppState;

/// Build and run the desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let identifier = app.config().identifier.clone();
            let paths = AppPaths::resolve(&identifier)?;
            let conn = db::open(&paths.database)?;

            // Make sure the brand folder always holds a usable logo.
            if !paths.logo_file().exists() {
                let _ = std::fs::write(paths.logo_file(), services::brand::DEFAULT_LOGO_SVG);
            }

            let state = AppState::new(paths.clone(), conn);
            app.manage(state);

            // The render host is a chrome-free window that hosts the canonical
            // invoice document for PDF, print and image export.
            WebviewWindowBuilder::new(
                app,
                RENDER_HOST_LABEL,
                WebviewUrl::App("render.html".into()),
            )
            .title("فاتورة - التصدير")
            .inner_size(794.0, 1123.0)
            .position(-2000.0, -2000.0)
            .visible(false)
            .decorations(false)
            .resizable(false)
            .build()?;

            // Optional automatic backup at launch (FR-09).
            run_auto_backup(app.handle());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Products
            commands::products::list_products,
            commands::products::get_product,
            commands::products::create_product,
            commands::products::update_product,
            commands::products::delete_product,
            commands::products::restore_product,
            commands::products::suggest_products,
            commands::products::count_products,
            commands::products::parse_amount,
            // Invoices
            commands::invoices::list_invoices,
            commands::invoices::get_invoice,
            commands::invoices::create_draft,
            commands::invoices::update_draft,
            commands::invoices::issue_invoice,
            commands::invoices::preview_invoice,
            commands::invoices::delete_invoice,
            commands::invoices::restore_invoice,
            commands::invoices::purge_invoice,
            commands::invoices::list_trashed_invoices,
            commands::invoices::duplicate_invoice,
            commands::invoices::validation_error,
            // Drafts (crash recovery)
            commands::drafts::list_drafts,
            commands::drafts::save_draft,
            commands::drafts::delete_draft,
            // Settings and brand
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_brand_logo,
            commands::settings::save_brand_logo,
            commands::settings::import_brand_logo,
            commands::settings::reset_brand_logo,
            commands::settings::apply_brand_color,
            // Excel
            commands::excel::excel_inspect,
            commands::excel::excel_dry_run,
            commands::excel::excel_apply,
            commands::excel::excel_export,
            commands::excel::excel_write_template,
            // Backup
            commands::backup::create_backup,
            commands::backup::validate_backup,
            commands::backup::restore_backup,
            commands::backup::list_backups,
            commands::backup::prune_backups,
            commands::backup::last_backup,
            // Export
            commands::export::invoice_document,
            commands::export::preview_document,
            commands::export::export_invoice_pdf,
            commands::export::print_invoice,
            commands::export::export_invoice_images,
            commands::export::open_containing_folder,
            // Updates
            commands::update::current_version,
            commands::update::check_for_updates,
            commands::update::install_update,
            commands::update::defer_update,
            commands::update::deferred_update,
        ])
        .run(tauri::generate_context!())
        .expect("تعذّر تشغيل التطبيق");
}

/// Create an automatic backup when the feature is enabled in Settings.
fn run_auto_backup(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let settings = match state.settings() {
        Ok(settings) => settings,
        Err(_) => return,
    };
    if !settings.auto_backup_enabled {
        return;
    }

    let folder = if settings.backup_location.trim().is_empty() {
        state.paths.backups.clone()
    } else {
        std::path::PathBuf::from(settings.backup_location.trim())
    };
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let target = folder.join(format!("invoice-autobackup-{stamp}.zip"));

    let result = state
        .with_conn(|conn| services::backup::create_backup(conn, &target, Some(&state.paths.brand)));
    if result.is_ok() {
        let _ = services::backup::prune_backups(
            &folder,
            settings.auto_backup_retention.max(1) as usize,
        );
    }
}
