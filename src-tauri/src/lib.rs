use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, State, WindowEvent,
};
use uuid::Uuid;
use watermark_core::{
    batch_embed, batch_extract, export_batch_records, AlgorithmProfile, EmbedOptions,
    EvidenceExportFormat, EvidenceRecord, EvidenceStore, OutputFormat,
};

#[derive(Clone)]
struct AppPaths {
    db_path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmbedRequest {
    input: PathBuf,
    output: PathBuf,
    owner: String,
    key: String,
    profile: UiProfile,
    output_format: UiOutputFormat,
    quality: u8,
    strength: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractRequest {
    input: PathBuf,
    key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportRequest {
    batch_id: Uuid,
    format: UiEvidenceFormat,
    output: PathBuf,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum UiProfile {
    Fidelity,
    Balanced,
    Strong,
}

impl From<UiProfile> for AlgorithmProfile {
    fn from(value: UiProfile) -> Self {
        match value {
            UiProfile::Fidelity => Self::Fidelity,
            UiProfile::Balanced => Self::Balanced,
            UiProfile::Strong => Self::Strong,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum UiOutputFormat {
    Preserve,
    Jpeg,
    Png,
    Webp,
}

impl From<UiOutputFormat> for OutputFormat {
    fn from(value: UiOutputFormat) -> Self {
        match value {
            UiOutputFormat::Preserve => Self::Preserve,
            UiOutputFormat::Jpeg => Self::Jpeg,
            UiOutputFormat::Png => Self::Png,
            UiOutputFormat::Webp => Self::Webp,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum UiEvidenceFormat {
    Json,
    Csv,
    Pdf,
}

impl From<UiEvidenceFormat> for EvidenceExportFormat {
    fn from(value: UiEvidenceFormat) -> Self {
        match value {
            UiEvidenceFormat::Json => Self::Json,
            UiEvidenceFormat::Csv => Self::Csv,
            UiEvidenceFormat::Pdf => Self::Pdf,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    db_path: PathBuf,
}

#[tauri::command]
fn app_info(paths: State<'_, AppPaths>) -> AppInfo {
    AppInfo {
        db_path: paths.db_path.clone(),
    }
}

#[tauri::command]
fn embed_batch(
    paths: State<'_, AppPaths>,
    request: EmbedRequest,
) -> Result<serde_json::Value, String> {
    let store = EvidenceStore::open(&paths.db_path).map_err(display_error)?;
    let key = load_key(&request.key).map_err(display_error)?;
    let options = EmbedOptions {
        profile: request.profile.into(),
        strength: request.strength,
        output_format: request.output_format.into(),
        quality: request.quality,
    };
    let summary = batch_embed(
        request.input,
        request.output,
        &request.owner,
        &key,
        &options,
        &store,
    )
    .map_err(display_error)?;
    serde_json::to_value(summary).map_err(display_error)
}

#[tauri::command]
fn extract_batch(
    paths: State<'_, AppPaths>,
    request: ExtractRequest,
) -> Result<serde_json::Value, String> {
    let store = EvidenceStore::open(&paths.db_path).map_err(display_error)?;
    let key = load_key(&request.key).map_err(display_error)?;
    let summary = batch_extract(request.input, &key, &store).map_err(display_error)?;
    serde_json::to_value(summary).map_err(display_error)
}

#[tauri::command]
fn recent_records(paths: State<'_, AppPaths>, limit: usize) -> Result<Vec<EvidenceRecord>, String> {
    let store = EvidenceStore::open(&paths.db_path).map_err(display_error)?;
    store.recent_records(limit).map_err(display_error)
}

#[tauri::command]
fn export_evidence(paths: State<'_, AppPaths>, request: ExportRequest) -> Result<(), String> {
    let store = EvidenceStore::open(&paths.db_path).map_err(display_error)?;
    export_batch_records(
        &store,
        request.batch_id,
        request.format.into(),
        request.output,
    )
    .map_err(display_error)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::current_dir().expect("current dir"));
            std::fs::create_dir_all(&app_data_dir)?;
            app.manage(AppPaths {
                db_path: app_data_dir.join("watermark-evidence.sqlite"),
            });

            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            embed_batch,
            extract_batch,
            recent_records,
            export_evidence
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn load_key(key_ref: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(key_ref);
    if path.exists() {
        return std::fs::read(path).map_err(Into::into);
    }
    Ok(key_ref.as_bytes().to_vec())
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
