#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nil_core::{
    add_clipboard_image, add_clipboard_text, clear_clipboard_history,
    copy_file_to_clipboard, copy_text_bytes_to_clipboard, delete_clipboard_item,
    load_clipboard_history, toggle_pin_clipboard_item, ClipboardItem, ClipboardKind,
};
use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{LogicalSize, Size, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

#[tauri::command]
fn get_clipboard_history() -> Result<Vec<ClipboardItem>, String> {
    Ok(load_clipboard_history())
}

#[tauri::command]
fn select_clipboard_item(window: WebviewWindow, id: String) -> Result<(), String> {
    let items = load_clipboard_history();
    if let Some(item) = items.into_iter().find(|i| i.id == id) {
        if item.kind == ClipboardKind::Image {
            let path = PathBuf::from(&item.content);
            copy_file_to_clipboard(&path)?;
        } else {
            copy_text_bytes_to_clipboard(&item.content)?;
        }
        let _ = window.close();
        std::process::exit(0);
    } else {
        Err("Elemento no encontrado".to_string())
    }
}

#[tauri::command]
fn delete_item(id: String) -> Result<(), String> {
    delete_clipboard_item(&id)
}

#[tauri::command]
fn toggle_pin_item(id: String) -> Result<bool, String> {
    toggle_pin_clipboard_item(&id)
}

#[tauri::command]
fn clear_history() -> Result<(), String> {
    clear_clipboard_history()
}

#[tauri::command]
fn close_window(window: WebviewWindow) -> Result<(), String> {
    let _ = window.close();
    std::process::exit(0);
}

fn handle_stdin_text() {
    let mut buffer = String::new();
    if std::io::stdin().read_to_string(&mut buffer).is_ok() {
        add_clipboard_text(&buffer);
    }
}

fn handle_stdin_image() {
    let mut buffer = Vec::new();
    if std::io::stdin().read_to_end(&mut buffer).is_ok() {
        add_clipboard_image(&buffer);
    }
}

fn run_daemon() {
    let exe = env::current_exe().unwrap_or_else(|_| PathBuf::from("nil-clip"));

    let text_child = Command::new("wl-paste")
        .arg("--watch")
        .arg(&exe)
        .arg("--add-stdin")
        .stdin(Stdio::null())
        .spawn();

    let img_child = Command::new("wl-paste")
        .arg("--type")
        .arg("image/png")
        .arg("--watch")
        .arg(&exe)
        .arg("--add-stdin-img")
        .stdin(Stdio::null())
        .spawn();

    if let (Ok(mut t), Ok(mut i)) = (text_child, img_child) {
        let _ = t.wait();
        let _ = i.wait();
    } else {
        let mut last_text = String::new();
        loop {
            if let Ok(output) = Command::new("wl-paste").arg("-n").output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !text.is_empty() && text != last_text {
                        last_text = text.clone();
                        add_clipboard_text(&text);
                    }
                }
            } else if let Ok(output) = Command::new("xclip").args(["-selection", "clipboard", "-o"]).output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !text.is_empty() && text != last_text {
                        last_text = text.clone();
                        add_clipboard_text(&text);
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}

fn launch_tauri() {
    tauri::Builder::default()
        .setup(|app| {
            let win_w = 620.0;
            let win_h = 380.0;

            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("nil-clip")
                .inner_size(win_w, win_h)
                .min_inner_size(480.0, 300.0)
                .max_inner_size(800.0, 500.0)
                .decorations(false)
                .transparent(false)
                .always_on_top(true)
                .resizable(false)
                .focused(true)
                .center()
                .build()?;

            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::Focused(false) = event {
                    let _ = window_clone.close();
                    std::process::exit(0);
                }
            });

            let _ = window.set_size(Size::Logical(LogicalSize {
                width: win_w,
                height: win_h,
            }));
            let _ = window.center();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            select_clipboard_item,
            delete_item,
            toggle_pin_item,
            clear_history,
            close_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("--ui");

    match mode {
        "--add-stdin" => {
            handle_stdin_text();
        }
        "--add-stdin-img" => {
            handle_stdin_image();
        }
        "--daemon" | "-d" => {
            run_daemon();
        }
        _ => {
            launch_tauri();
        }
    }
}
