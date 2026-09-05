#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nil_core::{create_note_with_content, load_notes, save_notes, Note};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn handle_cli_args() {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--new-note" => {
                if let Some(content) = args.get(i + 1) {
                    let title = if content.lines().count() > 1 {
                        "Nota"
                    } else {
                        "Nota Rápida"
                    };
                    let _ = create_note_with_content(title, content);
                    i += 1;
                }
            }
            "--attach-image" => {
                if let Some(path) = args.get(i + 1) {
                    let markdown = format!("![Captura](file://{})", path);
                    let _ = create_note_with_content("Captura", &markdown);
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
}

#[tauri::command]
fn list_notes() -> Vec<Note> {
    load_notes()
        .into_iter()
        .filter(|n| n.archived_at.is_none())
        .collect()
}

#[tauri::command]
fn list_archived_notes() -> Vec<Note> {
    load_notes()
        .into_iter()
        .filter(|n| n.archived_at.is_some())
        .collect()
}

#[tauri::command]
fn create_note() -> Result<Note, String> {
    let mut notes = load_notes();
    let note = Note {
        id: Uuid::new_v4().to_string(),
        title: String::new(),
        content: String::new(),
        updated_at: now_secs(),
        archived_at: None,
    };
    notes.push(note.clone());
    save_notes(&notes)?;
    Ok(note)
}

#[tauri::command]
fn update_note(id: String, title: String, content: String) -> Result<(), String> {
    let mut notes = load_notes();
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        note.title = title;
        note.content = content;
        note.updated_at = now_secs();
    }
    save_notes(&notes)
}

#[tauri::command]
fn archive_note(id: String) -> Result<(), String> {
    let mut notes = load_notes();
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        note.archived_at = Some(now_secs());
    }
    save_notes(&notes)
}

#[tauri::command]
fn restore_note(id: String) -> Result<(), String> {
    let mut notes = load_notes();
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        note.archived_at = None;
        note.updated_at = now_secs();
    }
    save_notes(&notes)
}

#[tauri::command]
fn delete_permanently(id: String) -> Result<(), String> {
    let mut notes = load_notes();
    notes.retain(|n| n.id != id);
    save_notes(&notes)
}

#[tauri::command]
fn empty_trash() -> Result<(), String> {
    let mut notes = load_notes();
    notes.retain(|n| n.archived_at.is_none());
    save_notes(&notes)
}

#[tauri::command]
fn close_window(window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.close();
    std::process::exit(0);
}

fn main() {
    handle_cli_args();
    tauri::Builder::default()
        .setup(|app| {
            let _window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::default(),
            )
            .title("nil-notes")
            .inner_size(720.0, 500.0)
            .min_inner_size(480.0, 320.0)
            .center()
            .decorations(false)
            .transparent(false)
            .resizable(true)
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_notes,
            list_archived_notes,
            create_note,
            update_note,
            archive_note,
            restore_note,
            delete_permanently,
            empty_trash,
            close_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
