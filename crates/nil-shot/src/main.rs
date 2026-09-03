#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::Engine;
use nil_core::{
    capture_area_image, capture_full_image, copy_image_bytes_to_clipboard,
    copy_text_bytes_to_clipboard, generate_temp_path, get_config_dir, get_hold_lock_path,
    get_png_dimensions, list_gallery_items, load_config, notify_and_maybe_edit,
    run_tesseract, save_config_file, Config, GalleryResponse, OcrBlock,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{LogicalSize, Size, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub struct AppState {
    pub image_path: Mutex<Option<PathBuf>>,
    pub config_path: PathBuf,
    pub is_pinned: Mutex<bool>,
}

fn calculate_window_size(img_w: f64, img_h: f64, mon_w: f64, mon_h: f64) -> (f64, f64) {
    let max_w = (mon_w * 0.88).max(640.0);
    let max_h = (mon_h * 0.88).max(480.0);

    let content_w = img_w + 68.0;
    let content_h = img_h + 40.0;

    let (target_w, target_h) = if content_w > max_w || content_h > max_h {
        let scale_w = (max_w - 68.0) / img_w.max(1.0);
        let scale_h = (max_h - 40.0) / img_h.max(1.0);
        let scale = scale_w.min(scale_h).min(1.0);
        (img_w * scale + 68.0, img_h * scale + 40.0)
    } else {
        (content_w, content_h)
    };

    (target_w.clamp(520.0, max_w), target_h.clamp(460.0, max_h))
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<Config, String> {
    Ok(load_config(&state.config_path))
}

#[tauri::command]
fn set_config(state: State<AppState>, open_editor: Option<bool>) -> Result<(), String> {
    let open = open_editor.unwrap_or(true);
    let cfg = Config { open_editor: open };
    save_config_file(&state.config_path, &cfg)
}

#[tauri::command]
fn get_image_data(state: State<AppState>) -> Result<String, String> {
    let lock = state.image_path.lock().map_err(|e| e.to_string())?;
    let path = lock
        .as_ref()
        .ok_or_else(|| "No image path provided".to_string())?;
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", encoded))
}

#[tauri::command]
fn resize_window(window: WebviewWindow, width: f64, height: f64) -> Result<(), String> {
    let (mon_w, mon_h) = if let Ok(Some(mon)) = window.current_monitor() {
        let size = mon.size();
        let scale = mon.scale_factor();
        (size.width as f64 / scale, size.height as f64 / scale)
    } else {
        (1920.0, 1080.0)
    };

    let (target_w, target_h) = calculate_window_size(width, height, mon_w, mon_h);
    let _ = window.set_size(Size::Logical(LogicalSize {
        width: target_w,
        height: target_h,
    }));
    let _ = window.center();
    Ok(())
}

#[tauri::command]
fn copy_to_clipboard(data_base64: Option<String>) -> Result<(), String> {
    let data = data_base64.ok_or_else(|| "No image data provided".to_string())?;

    let raw = if let Some(idx) = data.find(',') {
        &data[idx + 1..]
    } else {
        &data
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    copy_image_bytes_to_clipboard(&bytes)
}

#[tauri::command]
fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    copy_text_bytes_to_clipboard(&text)
}

#[tauri::command]
fn save_image(data_base64: Option<String>) -> Result<String, String> {
    let data = data_base64.ok_or_else(|| "No image data provided".to_string())?;

    let raw = if let Some(idx) = data.find(',') {
        &data[idx + 1..]
    } else {
        &data
    };

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let imagenes = PathBuf::from(&home).join("Im\u{00e1}genes");
    let dir = if imagenes.exists() {
        imagenes.join("Capturas de pantalla")
    } else {
        PathBuf::from(&home).join("Pictures").join("Screenshots")
    };
    fs::create_dir_all(&dir).map_err(|e| format!("Create dir error: {}", e))?;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let filename = format!("screenshot_{}.png", ts);
    let target = dir.join(&filename);
    fs::write(&target, &bytes).map_err(|e| format!("File write error: {}", e))?;

    let target_str = target.to_string_lossy().to_string();
    let target_path_for_thread = target.clone();
    let dir_for_thread = dir.clone();
    let filename_for_notify = filename.clone();

    std::thread::spawn(move || {
        let output = Command::new("notify-send")
            .arg("-a")
            .arg("nil-shot")
            .arg("-i")
            .arg(&target_path_for_thread)
            .arg("-A")
            .arg("open=Abrir")
            .arg("Captura guardada")
            .arg(&filename_for_notify)
            .output();

        if let Ok(out) = output {
            let choice = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if choice == "open" || choice == "0" || choice == "default" {
                let _ = Command::new("xdg-open").arg(&dir_for_thread).spawn();
            }
        }
    });

    Ok(target_str)
}

#[tauri::command]
async fn get_gallery_items(offset: usize, limit: usize) -> Result<GalleryResponse, String> {
    tauri::async_runtime::spawn_blocking(move || list_gallery_items(offset, limit))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
async fn load_gallery_image(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err("El archivo no existe".to_string());
    }
    let p_clone = p.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || fs::read(&p_clone))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let mut lock = state.image_path.lock().map_err(|e| e.to_string())?;
    *lock = Some(p);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", encoded))
}

#[tauri::command]
fn toggle_pin_window(window: WebviewWindow, state: State<AppState>) -> Result<bool, String> {
    let mut lock = state.is_pinned.lock().map_err(|e| e.to_string())?;
    let next = !*lock;
    *lock = next;
    let _ = window.set_always_on_top(next);
    Ok(next)
}

#[tauri::command]
fn is_tiling_desktop() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();
    std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
        || desktop.contains("hyprland")
        || desktop.contains("sway")
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    let clean = url.trim();
    let target = if clean.starts_with("http://") || clean.starts_with("https://") {
        clean.to_string()
    } else {
        format!("https://{}", clean)
    };
    let _ = Command::new("xdg-open").arg(&target).spawn();
    Ok(())
}

#[tauri::command]
async fn extract_text(
    state: State<'_, AppState>,
    data_base64: Option<String>,
) -> Result<Vec<OcrBlock>, String> {
    let maybe_data = data_base64;

    let (path_to_process, is_custom_temp) = if let Some(data) = maybe_data {
        if data.is_empty() {
            let lock = state.image_path.lock().map_err(|e| e.to_string())?;
            let path = lock
                .as_ref()
                .cloned()
                .ok_or_else(|| "No image path provided".to_string())?;
            (path, false)
        } else {
            let raw = if let Some(idx) = data.find(',') {
                &data[idx + 1..]
            } else {
                &data
            };
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(raw.trim())
                .map_err(|e| format!("Base64 decode error: {}", e))?;
            let temp_path = generate_temp_path();
            fs::write(&temp_path, &bytes)
                .map_err(|e| format!("Temp write error: {}", e))?;
            (temp_path, true)
        }
    } else {
        let lock = state.image_path.lock().map_err(|e| e.to_string())?;
        let path = lock
            .as_ref()
            .cloned()
            .ok_or_else(|| "No image path provided".to_string())?;
        (path, false)
    };

    let path_clone = path_to_process.clone();
    let result = tauri::async_runtime::spawn_blocking(move || run_tesseract(&path_clone))
        .await
        .map_err(|e| format!("Task spawn error: {}", e))??;

    if is_custom_temp {
        let _ = fs::remove_file(&path_to_process);
    }

    Ok(result)
}

#[tauri::command]
fn close_window(window: WebviewWindow) -> Result<(), String> {
    let _ = window.close();
    std::process::exit(0);
}

fn launch_tauri(image_path: Option<PathBuf>, config_path: PathBuf) {
    let initial_dims = image_path
        .as_ref()
        .map(get_png_dimensions)
        .unwrap_or((960, 600));

    let state = AppState {
        image_path: Mutex::new(image_path),
        config_path,
        is_pinned: Mutex::new(false),
    };

    tauri::Builder::default()
        .manage(state)
        .setup(move |app| {
            let (w, h) = initial_dims;
            let (mon_w, mon_h) = if let Some(mon) = app.primary_monitor().ok().flatten() {
                let size = mon.size();
                let scale = mon.scale_factor();
                (size.width as f64 / scale, size.height as f64 / scale)
            } else {
                (1920.0, 1080.0)
            };

            let (win_w, win_h) = calculate_window_size(w as f64, h as f64, mon_w, mon_h);

            let _window =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                    .title("nil-shot")
                    .inner_size(win_w, win_h)
                    .center()
                    .decorations(false)
                    .transparent(false)
                    .resizable(true)
                    .build()?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            get_image_data,
            resize_window,
            copy_to_clipboard,
            copy_text_to_clipboard,
            save_image,
            toggle_pin_window,
            is_tiling_desktop,
            get_gallery_items,
            load_gallery_image,
            extract_text,
            open_external_url,
            close_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn run_full_capture(config_path: &PathBuf) {
    let path = generate_temp_path();
    if capture_full_image(&path) {
        if notify_and_maybe_edit(&path, "Captura copiada", "Pantalla completa en el portapapeles") {
            launch_tauri(Some(path), config_path.clone());
        }
    }
}

fn run_area_capture(config_path: &PathBuf) {
    let path = generate_temp_path();
    if capture_area_image(&path) {
        if notify_and_maybe_edit(&path, "Captura copiada", "\u{00c1}rea guardada en el portapapeles") {
            launch_tauri(Some(path), config_path.clone());
        }
    }
}

fn handle_key_press(config_path: PathBuf) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let lock_path = get_hold_lock_path();
    let token = format!("{}:{}", std::process::id(), now);
    let _ = fs::write(&lock_path, &token);

    std::thread::sleep(std::time::Duration::from_millis(400));

    if let Ok(current_token) = fs::read_to_string(&lock_path) {
        if current_token == token {
            let _ = fs::remove_file(&lock_path);
            run_area_capture(&config_path);
        }
    }
}

fn handle_key_release(config_path: &PathBuf) {
    let lock_path = get_hold_lock_path();
    if let Ok(token) = fs::read_to_string(&lock_path) {
        let parts: Vec<&str> = token.split(':').collect();
        if parts.len() == 2 {
            if let Ok(press_time) = parts[1].parse::<u128>() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                if now >= press_time && (now - press_time) < 600 {
                    let _ = fs::remove_file(&lock_path);
                    run_full_capture(config_path);
                }
            }
        }
    }
}

fn main() {
    let config_dir = get_config_dir();
    let config_path = config_dir.join("config.json");

    let args: Vec<String> = env::args().collect();
    if let Some(arg) = args.get(1) {
        let path = PathBuf::from(arg);
        if path.exists() && path.is_file() {
            launch_tauri(Some(path), config_path);
            return;
        }
    }

    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("--area");

    match mode {
        "--press" => {
            handle_key_press(config_path);
        }
        "--release" => {
            handle_key_release(&config_path);
        }
        "--full" | "-f" => {
            run_full_capture(&config_path);
        }
        "--area" | "-a" => {
            run_area_capture(&config_path);
        }
        "--edit" | "-e" => {
            let img_path = args.get(2).map(PathBuf::from);
            launch_tauri(img_path, config_path);
        }
        _ => {
            run_area_capture(&config_path);
        }
    }
}
