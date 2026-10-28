//! Settings and brand commands (FR-10).

use tauri::State;

use crate::domain::error::{AppError, AppResult};
use crate::services::brand::{self, BrandLogo};
use crate::services::settings::{self, AppSettings};
use crate::state::AppState;

/// Load the settings block.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.settings()
}

/// Persist the settings block.
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> AppResult<AppSettings> {
    let saved = state.with_conn(|conn| settings::save(conn, &settings))?;
    // Keep the on-disk logo in sync with the stored settings.
    if !saved.logo_svg.trim().is_empty() {
        let _ = brand::save_logo(&state.paths, &saved.logo_svg);
    }
    Ok(saved)
}

/// Read the current logo markup.
#[tauri::command]
pub fn get_brand_logo(state: State<'_, AppState>) -> AppResult<BrandLogo> {
    let settings = state.settings()?;
    Ok(BrandLogo {
        svg: brand::load_logo(&state.paths),
        brand_color: settings.brand_color,
    })
}

/// Replace the logo markup after validating it is safe SVG.
#[tauri::command]
pub fn save_brand_logo(state: State<'_, AppState>, svg: String) -> AppResult<BrandLogo> {
    brand::save_logo(&state.paths, &svg)?;
    let mut settings = state.settings()?;
    settings.logo_svg = svg.clone();
    let saved = state.with_conn(|conn| settings::save(conn, &settings))?;
    Ok(BrandLogo {
        svg,
        brand_color: saved.brand_color,
    })
}

/// Import a logo from a file chosen in the file dialog.
#[tauri::command]
pub fn import_brand_logo(state: State<'_, AppState>, path: String) -> AppResult<BrandLogo> {
    let file = std::path::PathBuf::from(&path);
    if !brand::is_svg(&file) {
        return Err(AppError::validation("الملف المختار ليس صورة SVG"));
    }
    let contents = std::fs::read_to_string(&file)
        .map_err(|e| AppError::io(format!("تعذّر قراءة ملف الشعار: {e}")))?;
    brand::save_logo(&state.paths, &contents)?;
    let mut settings = state.settings()?;
    settings.logo_svg = contents.clone();
    let saved = state.with_conn(|conn| settings::save(conn, &settings))?;
    Ok(BrandLogo {
        svg: contents,
        brand_color: saved.brand_color,
    })
}

/// Restore the logo shipped with the application.
#[tauri::command]
pub fn reset_brand_logo(state: State<'_, AppState>) -> AppResult<BrandLogo> {
    let mut settings = state.settings()?;
    settings.logo_svg = brand::DEFAULT_LOGO_SVG.to_string();
    let saved = state.with_conn(|conn| settings::save(conn, &settings))?;
    brand::save_logo(&state.paths, brand::DEFAULT_LOGO_SVG)?;
    Ok(BrandLogo {
        svg: brand::DEFAULT_LOGO_SVG.to_string(),
        brand_color: saved.brand_color,
    })
}

/// Recolor the current logo to the configured brand color.
#[tauri::command]
pub fn apply_brand_color(state: State<'_, AppState>) -> AppResult<BrandLogo> {
    let settings = state.settings()?;
    let current = brand::load_logo(&state.paths);
    let recolored = brand::recolor_svg(&current, &settings.brand_color);
    brand::save_logo(&state.paths, &recolored)?;
    let mut updated = settings.clone();
    updated.logo_svg = recolored.clone();
    let saved = state.with_conn(|conn| settings::save(conn, &updated))?;
    Ok(BrandLogo {
        svg: recolored,
        brand_color: saved.brand_color,
    })
}
