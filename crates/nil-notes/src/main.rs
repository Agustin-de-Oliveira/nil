#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
struct Note {
    id: String,
    title: String,
    content: String,
    updated_at: u64,
    #[serde(default)]
    archived_at: Option<u64>,
}

fn notes_path() -> PathBuf {
    let mut path = dirs_next();
    path.push("nil-notes");
    fs::create_dir_all(&path).ok();
    path.push("notes.json");
    path
}

fn dirs_next() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut h = PathBuf::from(std::env::var("HOME").unwrap_or_default());
            h.push(".local/share");
            h
        })
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_notes() -> Vec<Note> {
    let path = notes_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_notes(notes: &[Note]) -> Result<(), String> {
    let path = notes_path();
    serde_json::to_string(notes)
        .map_err(|e| e.to_string())
        .and_then(|s| fs::write(&path, s).map_err(|e| e.to_string()))
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
