use wasm_bindgen::prelude::*;
use leptos::*;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = window, js_name = callTauri)]
    async fn call_tauri_inner(cmd: &str, args: JsValue) -> JsValue;
}

async fn call_tauri(cmd: &str, args: JsValue) -> Result<JsValue, JsValue> {
    Ok(call_tauri_inner(cmd, args).await)
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
struct Note {
    id: String,
    title: String,
    content: String,
    updated_at: u64,
    #[serde(default)]
    archived_at: Option<u64>,
}

fn format_time(ts: u64) -> String {
    let now = (js_sys::Date::now() / 1000.0) as u64;
    let diff = now.saturating_sub(ts);
    if diff < 60 { return "Ahora".into(); }
    if diff < 3600 { return format!("Hace {} min", diff / 60); }
    if diff < 86400 { return format!("Hace {} h", diff / 3600); }
    if diff < 86400 * 7 { return format!("Hace {} d", diff / 86400); }
    let days = diff / 86400;
    format!("Hace {} sem", days / 7)
}

fn strip_html_tags(s: &str) -> String {
    let s = s
        .replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n")
        .replace("</div>", "\n").replace("</p>", "\n")
        .replace("</h1>", "\n").replace("</h2>", "\n").replace("</h3>", "\n")
        .replace("</li>", "\n");
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' { in_tag = true; }
        else if c == '>' { in_tag = false; }
        else if !in_tag { out.push(c); }
    }
    out.replace("&nbsp;", " ")
       .replace("&amp;", "&")
       .replace("&lt;", "<")
       .replace("&gt;", ">")
       .replace("&quot;", "\"")
}

fn note_display_title(note: &Note) -> String {
    let plain = strip_html_tags(&note.content);
    let first = plain.lines().map(|l| l.trim()).find(|l| !l.is_empty()).unwrap_or("");
    if first.is_empty() {
        if note.title.is_empty() {
            "Nueva nota".into()
        } else {
            note.title.clone()
        }
    } else {
        first.to_string()
    }
}

fn word_count(s: &str) -> usize {
    let plain = strip_html_tags(s);
    plain.split_whitespace().count()
}

fn load_stored_string(key: &str, default: &str) -> String {
    if let Some(window) = web_sys::window() {
        if let Ok(storage_val) = js_sys::Reflect::get(&window, &JsValue::from_str("localStorage")) {
            if !storage_val.is_undefined() && !storage_val.is_null() {
                if let Ok(item_val) = js_sys::Reflect::get(&storage_val, &JsValue::from_str(key)) {
                    if let Some(s) = item_val.as_string() {
                        return s;
                    }
                }
            }
        }
    }
    default.to_string()
}

fn load_stored_bool(key: &str, default: bool) -> bool {
    load_stored_string(key, &default.to_string())
        .parse()
        .unwrap_or(default)
}

fn load_stored_usize(key: &str, default: usize) -> usize {
    load_stored_string(key, &default.to_string())
        .parse()
        .unwrap_or(default)
}

fn save_stored(key: &str, val: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(storage_val) = js_sys::Reflect::get(&window, &JsValue::from_str("localStorage")) {
            if !storage_val.is_undefined() && !storage_val.is_null() {
                let _ = js_sys::Reflect::set(&storage_val, &JsValue::from_str(key), &JsValue::from_str(val));
            }
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (notes, set_notes) = signal(Vec::<Note>::new());
    let (active_id, set_active_id) = signal(Option::<String>::None);
    let (theme, set_theme) = signal(load_stored_string("nil_notes_theme", "dark"));
    let (focus_mode, set_focus_mode) = signal(load_stored_bool("nil_notes_focus_mode", false));
    let (hide_tabs, set_hide_tabs) = signal(load_stored_bool("nil_notes_hide_tabs", false));
    let (settings_open, set_settings_open) = signal(false);
    let (trash_open, set_trash_open) = signal(false);
    let (right_panel_hover, set_right_panel_hover) = signal(false);
    let (font_size, set_font_size) = signal(load_stored_usize("nil_notes_font_size", 14));
    let (layout_width, set_layout_width) = signal(load_stored_string("nil_notes_layout_width", "full"));
    let (draft_content, set_draft_content) = signal(String::new());
    let (title_collapsed, set_title_collapsed) = signal(false);
    let (app_ready, set_app_ready) = signal(false);
    let (closing_note_ids, set_closing_note_ids) = signal(std::collections::HashSet::<String>::new());
    let (new_note_id, set_new_note_id) = signal(Option::<String>::None);
    let (archived_notes, set_archived_notes) = signal(Vec::<Note>::new());
    let (removing_archived_ids, set_removing_archived_ids) = signal(std::collections::HashSet::<String>::new());
    let (panel_view, set_panel_view) = signal("notes");
    let (save_status, set_save_status) = signal("saved");
    let (save_version, set_save_version) = signal(0u64);

    Effect::new(move |_| {
        save_stored("nil_notes_theme", &theme.get());
    });
    Effect::new(move |_| {
        save_stored("nil_notes_focus_mode", &focus_mode.get().to_string());
    });
    Effect::new(move |_| {
        save_stored("nil_notes_hide_tabs", &hide_tabs.get().to_string());
    });
    Effect::new(move |_| {
        save_stored("nil_notes_font_size", &font_size.get().to_string());
    });
    Effect::new(move |_| {
        save_stored("nil_notes_layout_width", &layout_width.get());
    });

    let fetch_archived_notes = move || {
        leptos::task::spawn_local(async move {
            if let Ok(res) = call_tauri("list_archived_notes", JsValue::NULL).await {
                if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<Note>>(res) {
                    set_archived_notes.set(list);
                }
            }
        });
    };

    let fetch_notes = move || {
        leptos::task::spawn_local(async move {
            if let Ok(res) = call_tauri("list_notes", JsValue::NULL).await {
                if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<Note>>(res) {
                    let first_id = list.first().map(|n| n.id.clone());
                    set_notes.set(list.clone());
                    if active_id.get_untracked().is_none() {
                        if let Some(id) = first_id {
                            if let Some(note) = list.iter().find(|n| n.id == id) {
                                set_draft_content.set(note.content.clone());
                            }
                            set_active_id.set(Some(id));
                        }
                    }
                }
            }
            if !app_ready.get_untracked() {
                set_app_ready.set(true);
            }
        });
    };

    Effect::new(move |_| { fetch_notes(); });

    Effect::new(move |_| {
        if app_ready.get() {
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if let Some(body) = doc.body() {
                        let _ = body.class_list().add_1("app-ready");
                    }
                }
            }
        }
    });

    let active_note = move || {
        let id = active_id.get()?;
        notes.get().into_iter().find(|n| n.id == id)
    };

    let save_current = move || {
        let id = match active_id.get_untracked() { Some(i) => i, None => return };
        let content = draft_content.get_untracked();
        let plain = strip_html_tags(&content);
        let first = plain.lines().map(|l| l.trim()).find(|l| !l.is_empty()).unwrap_or("");
        let title_save = if first.is_empty() { "Nueva nota".to_string() } else { first.to_string() };
        let id_save = id.clone();
        let content_save = content.clone();
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({
                "id": id_save, "title": title_save, "content": content_save
            })).unwrap_or(JsValue::NULL);
            let _ = call_tauri("update_note", args).await;
            set_save_status.set("saved");
            set_notes.update(|list| {
                if let Some(n) = list.iter_mut().find(|n| n.id == id_save) {
                    n.title = title_save;
                    n.content = content_save;
                }
            });
        });
    };

    let trigger_content_change = move |new_html: String| {
        set_draft_content.set(new_html);
        let v = save_version.get_untracked() + 1;
        set_save_version.set(v);
        leptos::task::spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(w) = web_sys::window() {
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 2000);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            if save_version.get_untracked() == v {
                set_save_status.set("saving");
                let spin_promise = js_sys::Promise::new(&mut |resolve, _| {
                    if let Some(w) = web_sys::window() {
                        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1000);
                    }
                });
                let _ = wasm_bindgen_futures::JsFuture::from(spin_promise).await;
                if save_version.get_untracked() == v {
                    save_current();
                    set_save_status.set("saved");
                }
            }
        });
    };

    let select_note = move |note: Note| {
        save_current();
        set_title_collapsed.set(false);
        set_draft_content.set(note.content.clone());
        set_active_id.set(Some(note.id));
    };

    let new_note = move || {
        save_current();
        set_title_collapsed.set(false);
        leptos::task::spawn_local(async move {
            if let Ok(res) = call_tauri("create_note", JsValue::NULL).await {
                if let Ok(note) = serde_wasm_bindgen::from_value::<Note>(res) {
                    set_draft_content.set("<h1><br></h1>".to_string());
                    let new_id = note.id.clone();
                    set_new_note_id.set(Some(new_id.clone()));
                    set_notes.update(|list| list.push(note));
                    set_active_id.set(Some(new_id.clone()));
                    let note_id_clear = new_id.clone();
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        if let Some(w) = web_sys::window() {
                            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 260);
                        }
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    if new_note_id.get_untracked().as_deref() == Some(&note_id_clear) {
                        set_new_note_id.set(None);
                    }
                }
            }
        });
    };

    let delete_note_by_id = move |id: String| {
        set_closing_note_ids.update(|set| { set.insert(id.clone()); });
        let id_del = id.clone();
        leptos::task::spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(w) = web_sys::window() {
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 190);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "id": id_del }))
                .unwrap_or(JsValue::NULL);
            let _ = call_tauri("archive_note", args).await;
            if active_id.get_untracked().as_deref() == Some(&id_del) {
                set_active_id.set(None);
            }
            set_closing_note_ids.update(|set| { set.remove(&id_del); });
            fetch_notes();
            fetch_archived_notes();
        });
    };

    let restore_note_by_id = move |id: String| {
        set_removing_archived_ids.update(|set| { set.insert(id.clone()); });
        let id_restore = id.clone();
        leptos::task::spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(w) = web_sys::window() {
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 180);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "id": id_restore }))
                .unwrap_or(JsValue::NULL);
            let _ = call_tauri("restore_note", args).await;
            set_removing_archived_ids.update(|set| { set.remove(&id_restore); });
            fetch_notes();
            fetch_archived_notes();
        });
    };

    let delete_permanent_by_id = move |id: String| {
        set_removing_archived_ids.update(|set| { set.insert(id.clone()); });
        let id_del = id.clone();
        leptos::task::spawn_local(async move {
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                if let Some(w) = web_sys::window() {
                    let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 180);
                }
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            let args = serde_wasm_bindgen::to_value(&serde_json::json!({ "id": id_del }))
                .unwrap_or(JsValue::NULL);
            let _ = call_tauri("delete_permanently", args).await;
            set_removing_archived_ids.update(|set| { set.remove(&id_del); });
            fetch_archived_notes();
        });
    };

    let empty_trash_cmd = move || {
        leptos::task::spawn_local(async move {
            let _ = call_tauri("empty_trash", JsValue::NULL).await;
            fetch_archived_notes();
        });
    };

    let close_app = move || {
        leptos::task::spawn_local(async move {
            let _ = call_tauri("close_window", JsValue::NULL).await;
        });
    };

    let editor_ref = NodeRef::<html::Div>::new();
    let (toolbar_visible, set_toolbar_visible) = signal(false);
    let (toolbar_x, set_toolbar_x) = signal(0i32);
    let (toolbar_y, set_toolbar_y) = signal(0i32);
    let (active_formats, set_active_formats) = signal(std::collections::HashSet::<String>::new());

    let check_selection = move || {
        if let Some(window) = web_sys::window() {
            if let Ok(fn_val) = js_sys::Reflect::get(&window, &JsValue::from_str("queryActiveFormats")) {
                if !fn_val.is_undefined() && !fn_val.is_null() {
                    let f: &js_sys::Function = fn_val.unchecked_ref();
                    if let Ok(val) = f.call0(&window) {
                        if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<String>>(val) {
                            set_active_formats.set(list.into_iter().collect());
                        }
                    }
                }
            }
            if let Ok(sel_val) = js_sys::Reflect::get(&window, &JsValue::from_str("getSelection")) {
                let get_sel_fn: &js_sys::Function = sel_val.unchecked_ref();
                if let Ok(sel) = get_sel_fn.call0(&window) {
                    if let Ok(text_val) = js_sys::Reflect::get(&sel, &JsValue::from_str("toString")) {
                        let to_string_fn: &js_sys::Function = text_val.unchecked_ref();
                        if let Ok(text) = to_string_fn.call0(&sel) {
                            let text_str = text.as_string().unwrap_or_default();
                            if text_str.trim().is_empty() {
                                set_toolbar_visible.set(false);
                                return;
                            }
                            let range_count = js_sys::Reflect::get(&sel, &JsValue::from_str("rangeCount"))
                                .ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                            if range_count > 0.0 {
                                if let Ok(get_range_val) = js_sys::Reflect::get(&sel, &JsValue::from_str("getRangeAt")) {
                                    let get_range_fn: &js_sys::Function = get_range_val.unchecked_ref();
                                    if let Ok(range) = get_range_fn.call1(&sel, &JsValue::from_f64(0.0)) {
                                        if let Ok(get_rect_val) = js_sys::Reflect::get(&range, &JsValue::from_str("getBoundingClientRect")) {
                                            let get_rect_fn: &js_sys::Function = get_rect_val.unchecked_ref();
                                            if let Ok(rect) = get_rect_fn.call0(&range) {
                                                let left = js_sys::Reflect::get(&rect, &JsValue::from_str("left")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                                                let top = js_sys::Reflect::get(&rect, &JsValue::from_str("top")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                                                let bottom = js_sys::Reflect::get(&rect, &JsValue::from_str("bottom")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                                                let width = js_sys::Reflect::get(&rect, &JsValue::from_str("width")).ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                                                let toolbar_h = 36i32;
                                                let y = if top - toolbar_h - 8 < 44 { bottom + 8 } else { top - toolbar_h - 8 };
                                                let win_w = web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(800.0) as i32;
                                                let toolbar_w = 280i32;
                                                let half_w = toolbar_w / 2;
                                                let padding = 12i32;
                                                let x = (left + width / 2).clamp(half_w + padding, win_w - half_w - padding);
                                                set_toolbar_x.set(x);
                                                set_toolbar_y.set(y);
                                                set_toolbar_visible.set(true);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let exec = move |cmd: &str, val: &str| {
        if let Some(window) = web_sys::window() {
            if let Ok(fn_val) = js_sys::Reflect::get(&window, &JsValue::from_str("formatText")) {
                if !fn_val.is_undefined() && !fn_val.is_null() {
                    let f: &js_sys::Function = fn_val.unchecked_ref();
                    let _ = f.call2(&window, &JsValue::from_str(cmd), &JsValue::from_str(val));
                }
            }
            if let Some(el) = editor_ref.get() {
                let _ = el.focus();
                trigger_content_change(el.inner_html());
            }
            check_selection();
        }
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if key == "Enter" && !ev.shift_key() {
            if let Some(window) = web_sys::window() {
                if let Ok(fn_val) = js_sys::Reflect::get(&window, &JsValue::from_str("handleHeadingEnter")) {
                    if !fn_val.is_undefined() && !fn_val.is_null() {
                        let f: &js_sys::Function = fn_val.unchecked_ref();
                        if let Some(el) = editor_ref.get() {
                            if let Ok(res) = f.call1(&window, &el) {
                                if let Some(kind) = res.as_string() {
                                    ev.prevent_default();
                                    if kind == "title" {
                                        set_title_collapsed.set(true);
                                    }
                                    trigger_content_change(el.inner_html());
                                    check_selection();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        } else if key == "ArrowUp" {
            if title_collapsed.get_untracked() {
                if let Some(window) = web_sys::window() {
                    if let Ok(fn_val) = js_sys::Reflect::get(&window, &JsValue::from_str("isAtTopOfBody")) {
                        if !fn_val.is_undefined() && !fn_val.is_null() {
                            let f: &js_sys::Function = fn_val.unchecked_ref();
                            if let Some(el) = editor_ref.get() {
                                if let Ok(res) = f.call1(&window, &el) {
                                    if res.as_bool().unwrap_or(false) {
                                        ev.prevent_default();
                                        set_title_collapsed.set(false);
                                        if let Ok(focus_val) = js_sys::Reflect::get(&window, &JsValue::from_str("focusTitleEnd")) {
                                            let focus_f: &js_sys::Function = focus_val.unchecked_ref();
                                            let _ = focus_f.call1(&window, &el);
                                        }
                                        check_selection();
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if key == "Backspace" {
            if title_collapsed.get_untracked() {
                if let Some(window) = web_sys::window() {
                    if let Ok(fn_val) = js_sys::Reflect::get(&window, &JsValue::from_str("isAtStartOfFirstBody")) {
                        if !fn_val.is_undefined() && !fn_val.is_null() {
                            let f: &js_sys::Function = fn_val.unchecked_ref();
                            if let Some(el) = editor_ref.get() {
                                if let Ok(res) = f.call1(&window, &el) {
                                    if res.as_bool().unwrap_or(false) {
                                        ev.prevent_default();
                                        set_title_collapsed.set(false);
                                        if let Ok(focus_val) = js_sys::Reflect::get(&window, &JsValue::from_str("focusTitleEnd")) {
                                            let focus_f: &js_sys::Function = focus_val.unchecked_ref();
                                            let _ = focus_f.call1(&window, &el);
                                        }
                                        check_selection();
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if key == "Escape" {
            ev.prevent_default();
            if settings_open.get_untracked() {
                set_settings_open.set(false);
            } else if trash_open.get_untracked() {
                set_trash_open.set(false);
            } else if right_panel_hover.get_untracked() {
                set_right_panel_hover.set(false);
            } else if focus_mode.get_untracked() {
                set_focus_mode.set(false);
            } else {
                save_current();
                close_app();
            }
        } else if ev.ctrl_key() && key == "n" {
            ev.prevent_default();
            new_note();
        } else if ev.ctrl_key() && key == "w" {
            ev.prevent_default();
            save_current();
            if let Some(cur) = active_id.get_untracked() {
                delete_note_by_id(cur);
            }
        } else if ev.ctrl_key() && ev.shift_key() && (key == "c" || key == "C") {
            ev.prevent_default();
            exec("checkbox", "");
        } else if key == "F11" || (ev.ctrl_key() && (key == "f" || key == "F" || key == "d" || key == "D")) {
            ev.prevent_default();
            set_focus_mode.update(|f| *f = !*f);
        }
    };

    let char_count = move || strip_html_tags(&draft_content.get()).chars().count();
    let is_dark = move || theme.get() == "dark";

    Effect::new(move |_| {
        let _id = active_id.get();
        let mut content = draft_content.get_untracked();
        if content.trim().is_empty() {
            content = "<h1><br></h1>".to_string();
        }
        if let Some(el) = editor_ref.get() {
            el.set_inner_html(&content);
        }
    });

    view! {
        <div class=move || format!(
            "h-screen w-screen flex flex-row select-none border rounded-xl overflow-hidden shadow-2xl relative {}",
            if is_dark() {
                "bg-[#101014] text-zinc-100 border-white/10 ring-1 ring-white/5"
            } else {
                "bg-[#faf8f5] text-[#1a1918] border-black/10 ring-1 ring-black/5"
            }
        )>
            <div
                class=move || format!(
                    "floating-toolbar fixed z-[200] -translate-x-1/2 flex items-center gap-0.5 px-1.5 py-1 rounded-lg border shadow-xl {}",
                    if toolbar_visible.get() {
                        "opacity-100 translate-y-0 scale-100 pointer-events-auto"
                    } else {
                        "opacity-0 translate-y-1.5 scale-95 pointer-events-none"
                    },
                )
                style=move || format!("left: {}px; top: {}px; background: {}; border-color: {};",
                    toolbar_x.get(), toolbar_y.get(),
                    if is_dark() { "#1c1c20" } else { "#ffffff" },
                    if is_dark() { "rgba(255,255,255,0.12)" } else { "rgba(0,0,0,0.10)" },
                )
                on:mousedown=move |ev| {
                    ev.prevent_default();
                }
            >
                {
                    let exec2 = exec;
                    let btns: Vec<(&'static str, &'static str, &'static str)> = vec![
                        ("B", "bold", ""),
                        ("I", "italic", ""),
                        ("U", "underline", ""),
                        ("S", "strikeThrough", ""),
                        ("H1", "formatBlock", "<h1>"),
                        ("H2", "formatBlock", "<h2>"),
                        ("H3", "formatBlock", "<h3>"),
                        ("•", "insertUnorderedList", ""),
                        ("1.", "insertOrderedList", ""),
                        ("☑", "checkbox", ""),
                    ];
                    btns.into_iter().map(|(label, cmd, val)| {
                        let cmd_str = cmd.to_string();
                        let val_str = val.to_string();
                        let key = if cmd == "formatBlock" { label.to_string() } else { cmd.to_string() };
                        let is_active = move || active_formats.get().contains(&key);
                        view! {
                            <button
                                class=move || format!(
                                    "flex items-center justify-center w-6 h-6 rounded text-[11px] font-semibold transition-colors cursor-pointer {}",
                                    if is_active() {
                                        if is_dark() {
                                            "bg-white/20 text-white shadow-sm ring-1 ring-white/10"
                                        } else {
                                            "bg-black/15 text-zinc-900 shadow-sm ring-1 ring-black/10"
                                        }
                                    } else {
                                        if is_dark() {
                                            "text-zinc-300 hover:bg-white/10 hover:text-white"
                                        } else {
                                            "text-zinc-600 hover:bg-black/8 hover:text-zinc-900"
                                        }
                                    }
                                )
                                on:mousedown=move |ev| {
                                    ev.prevent_default();
                                }
                                on:click=move |ev| {
                                    ev.prevent_default();
                                    exec2(cmd_str.as_str(), val_str.as_str());
                                }
                            >
                                {label}
                            </button>
                        }
                    }).collect::<Vec<_>>()
                }
            </div>
            <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden relative">
                <header
                    class=move || format!(
                        "px-4 flex items-center justify-between shrink-0 z-20 gap-2 overflow-hidden transition-all duration-300 ease-in-out {} {}",
                        if is_dark() { "bg-[#101014]" } else { "bg-[#faf8f5]" },
                        if focus_mode.get() {
                            "h-0 opacity-0 -translate-y-full pointer-events-none py-0"
                        } else {
                            "h-9 opacity-100 translate-y-0"
                        }
                    )
                >
                    <div class="flex-1 relative flex items-center h-full min-w-0 overflow-hidden">
                        <div class=move || format!(
                            "absolute inset-0 flex items-center justify-center min-w-0 px-2 transition-all duration-300 ease-out {}",
                            if hide_tabs.get() { "opacity-100 translate-y-0 pointer-events-auto" } else { "opacity-0 -translate-y-2 pointer-events-none" }
                        )>
                            <span class=move || format!(
                                "text-[12px] font-medium tracking-tight truncate max-w-[300px] {}",
                                if is_dark() { "text-zinc-300" } else { "text-zinc-700" }
                            )>
                                {active_note().map(|n| note_display_title(&n)).unwrap_or_else(|| "nil-notes".into())}
                            </span>
                        </div>

                        <div class=move || format!(
                            "flex items-center gap-1 overflow-x-auto no-scrollbar flex-1 min-w-0 py-0.5 transition-all duration-300 ease-out {}",
                            if !hide_tabs.get() { "opacity-100 translate-y-0 pointer-events-auto" } else { "opacity-0 translate-y-2 pointer-events-none" }
                        )>
                                <For
                                    each=move || notes.get()
                                    key=|note| note.id.clone()
                                    let:note
                                >
                                    {
                                        let note_id = note.id.clone();
                                        let note_for_click = note.clone();
                                        let note_id_close = note.id.clone();
                                        let note_id_close_click = note.id.clone();
                                        let note_id_active = note.id.clone();
                                        let note_id_closing = note.id.clone();
                                        let note_id_new = note.id.clone();
                                        let note_for_title = note.clone();

                                        view! {
                                            <div
                                                class=move || format!(
                                                    "{} relative flex items-center gap-1 pl-2 pr-1 py-0.5 rounded-md text-[11.5px] font-medium transition-colors duration-100 cursor-pointer shrink-0 border {}",
                                                    if closing_note_ids.get().contains(&note_id_closing) { "tab-exit" } else if new_note_id.get().as_deref() == Some(&note_id_new) { "tab-enter" } else { "" },
                                                    if active_id.get().as_deref() == Some(&note_id_active) {
                                                        if is_dark() {
                                                            "bg-white/[0.04] text-zinc-200 border-white/5"
                                                        } else {
                                                            "bg-black/[0.03] text-zinc-800 border-black/5"
                                                        }
                                                    } else {
                                                        if is_dark() {
                                                            "text-zinc-500 hover:text-zinc-300 hover:bg-white/[0.02] border-transparent"
                                                        } else {
                                                            "text-zinc-400 hover:text-zinc-700 hover:bg-black/[0.02] border-transparent"
                                                        }
                                                    }
                                                )
                                                on:click=move |_| select_note(note_for_click.clone())
                                            >
                                                <span class="truncate max-w-[120px]">
                                                    {move || {
                                                        let is_act = active_id.get().as_deref() == Some(&note_id);
                                                        if is_act {
                                                            let plain = strip_html_tags(&draft_content.get());
                                                            let first = plain.lines()
                                                                .map(|l| l.trim())
                                                                .find(|l| !l.is_empty())
                                                                .unwrap_or("");
                                                            if first.is_empty() { "Nueva nota".to_string() } else { first.to_string() }
                                                        } else {
                                                            note_display_title(&note_for_title)
                                                        }
                                                    }}
                                                </span>
                                                <button
                                                    class=move || format!(
                                                        "w-3.5 h-3.5 rounded flex items-center justify-center transition-colors duration-100 cursor-pointer {}",
                                                        if active_id.get().as_deref() == Some(&note_id_close) {
                                                            if is_dark() {
                                                                "text-zinc-400 hover:text-zinc-100 hover:bg-white/10"
                                                            } else {
                                                                "text-zinc-500 hover:text-zinc-900 hover:bg-black/10"
                                                            }
                                                        } else {
                                                            if is_dark() {
                                                                "text-zinc-500 hover:text-zinc-200 hover:bg-white/10"
                                                            } else {
                                                                "text-zinc-400 hover:text-zinc-800 hover:bg-black/10"
                                                            }
                                                        }
                                                    )
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        delete_note_by_id(note_id_close_click.clone());
                                                    }
                                                >
                                                    <svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                                        <path d="M18 6 6 18M6 6l12 12" />
                                                    </svg>
                                                </button>
                                            </div>
                                        }
                                    }
                                </For>

                                <button
                                    class=move || format!(
                                        "flex items-center justify-center w-5 h-5 rounded-md transition-all duration-150 cursor-pointer shrink-0 {}",
                                        if is_dark() {
                                            "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06] active:scale-95"
                                        } else {
                                            "text-zinc-500 hover:text-zinc-900 hover:bg-black/[0.05] active:scale-95"
                                        }
                                    )
                                    on:click=move |_| new_note()
                                >
                                    <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M12.001 5.00003V19.002"></path>
                                        <path d="M19.002 12.002L4.99998 12.002"></path>
                                    </svg>
                                </button>
                            </div>
                        </div>

                    <div class="flex items-center gap-1 shrink-0">
                        <button
                            class=move || format!(
                                "flex items-center justify-center w-6 h-6 rounded-md transition-all duration-150 cursor-pointer {}",
                                if trash_open.get() {
                                    if is_dark() { "text-white bg-white/[0.08]" } else { "text-zinc-900 bg-black/[0.06]" }
                                } else {
                                    if is_dark() {
                                        "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
                                    } else {
                                        "text-zinc-500 hover:text-zinc-900 hover:bg-black/[0.04]"
                                    }
                                }
                            )
                            title="Papelera"
                            on:click=move |_| {
                                set_trash_open.update(|s| {
                                    *s = !*s;
                                    if *s {
                                        set_settings_open.set(false);
                                        fetch_archived_notes();
                                    }
                                });
                            }
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M20 8V15.0093C20 17.8415 20 19.2576 19.1213 20.1375C18.48 20.7797 17.5534 20.9531 16 21M4 8V15.0093C4 17.8415 4 19.2576 4.87868 20.1375C5.51998 20.7797 6.44655 20.9531 7.99999 21"></path>
                                <path d="M19.5 3H4.5C3.56538 3 3.09808 3 2.75 3.20096C2.52197 3.33261 2.33261 3.52197 2.20096 3.75C2 4.09808 2 4.56538 2 5.5C2 6.43462 2 6.90192 2.20096 7.25C2.33261 7.47803 2.52197 7.66739 2.75 7.79904C3.09808 8 3.56538 8 4.5 8H19.5C20.4346 8 20.9019 8 21.25 7.79904C21.478 7.66739 21.6674 7.47803 21.799 7.25C22 6.90192 22 6.43462 22 5.5C22 4.56538 22 4.09808 21.799 3.75C21.6674 3.52197 21.478 3.33261 21.25 3.20096C20.9019 3 20.4346 3 19.5 3Z"></path>
                            </svg>
                        </button>

                        <button
                            class=move || format!(
                                "flex items-center justify-center w-6 h-6 rounded-md transition-all duration-150 cursor-pointer {}",
                                if settings_open.get() {
                                    if is_dark() { "text-white bg-white/[0.08]" } else { "text-zinc-900 bg-black/[0.06]" }
                                } else {
                                    if is_dark() {
                                        "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
                                    } else {
                                        "text-zinc-500 hover:text-zinc-900 hover:bg-black/[0.04]"
                                    }
                                }
                            )
                            title="Ajustes"
                            on:click=move |_| {
                                set_settings_open.update(|s| {
                                    *s = !*s;
                                    if *s {
                                        set_trash_open.set(false);
                                    }
                                });
                            }
                        >
                            <svg class="w-4 h-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M21.3175 7.14139L20.8239 6.28479C20.4506 5.63696 20.264 5.31305 19.9464 5.18388C19.6288 5.05472 19.2696 5.15664 18.5513 5.36048L17.3311 5.70418C16.8725 5.80994 16.3913 5.74994 15.9726 5.53479L15.6357 5.34042C15.2766 5.11043 15.0004 4.77133 14.8475 4.37274L14.5136 3.37536C14.294 2.71534 14.1842 2.38533 13.9228 2.19657C13.6615 2.00781 13.3143 2.00781 12.6199 2.00781H11.5051C10.8108 2.00781 10.4636 2.00781 10.2022 2.19657C9.94085 2.38533 9.83106 2.71534 9.61149 3.37536L9.27753 4.37274C9.12465 4.77133 8.84845 5.11043 8.48937 5.34042L8.15249 5.53479C7.73374 5.74994 7.25259 5.80994 6.79398 5.70418L5.57375 5.36048C4.85541 5.15664 4.49625 5.05472 4.17867 5.18388C3.86109 5.31305 3.67445 5.63696 3.30115 6.28479L2.80757 7.14139C2.45766 7.74864 2.2827 8.05227 2.31666 8.37549C2.35061 8.69871 2.58483 8.95918 3.05326 9.48012L4.0843 10.6328C4.3363 10.9518 4.51521 11.5078 4.51521 12.0077C4.51521 12.5078 4.33636 13.0636 4.08433 13.3827L3.05326 14.5354C2.58483 15.0564 2.35062 15.3168 2.31666 15.6401C2.2827 15.9633 2.45766 16.2669 2.80757 16.8741L3.30114 17.7307C3.67443 18.3785 3.86109 18.7025 4.17867 18.8316C4.49625 18.9608 4.85542 18.8589 5.57377 18.655L6.79394 18.3113C7.25263 18.2055 7.73387 18.2656 8.15267 18.4808L8.4895 18.6752C8.84851 18.9052 9.12464 19.2442 9.2775 19.6428L9.61149 20.6403C9.83106 21.3003 9.94085 21.6303 10.2022 21.8191C10.4636 22.0078 10.8108 22.0078 11.5051 22.0078H12.6199C13.3143 22.0078 13.6615 22.0078 13.9228 21.8191C14.1842 21.6303 14.294 21.3003 14.5136 20.6403L14.8476 19.6428C15.0004 19.2442 15.2765 18.9052 15.6356 18.6752L15.9724 18.4808C16.3912 18.2656 16.8724 18.2055 17.3311 18.3113L18.5513 18.655C19.2696 18.8589 19.6288 18.9608 19.9464 18.8316C20.264 18.7025 20.4506 18.3785 20.8239 17.7307L21.3175 16.8741C21.6674 16.2669 21.8423 15.9633 21.8084 15.6401C21.7744 15.3168 21.5402 15.0564 21.0718 14.5354L20.0407 13.3827C19.7887 13.0636 19.6098 12.5078 19.6098 12.0077C19.6098 11.5078 19.7888 10.9518 20.0407 10.6328L21.0718 9.48012C21.5402 8.95918 21.7744 8.69871 21.8084 8.37549C21.8423 8.05227 21.6674 7.74864 21.3175 7.14139Z" stroke-linecap="round"></path>
                                <path d="M15.5195 12C15.5195 13.933 13.9525 15.5 12.0195 15.5C10.0865 15.5 8.51953 13.933 8.51953 12C8.51953 10.067 10.0865 8.5 12.0195 8.5C13.9525 8.5 15.5195 10.067 15.5195 12Z"></path>
                            </svg>
                        </button>
                    </div>
                </header>

                <main class=move || format!("flex-1 flex flex-col min-h-0 transition-colors duration-200 {}", if is_dark() { "bg-[#101014]" } else { "bg-[#faf8f5]" })>
                    {move || match active_id.get() {
                        None => view! {
                            <div class=move || format!(
                                "flex-1 flex flex-col items-center justify-center gap-3 {}",
                                if is_dark() { "text-zinc-600" } else { "text-zinc-400" }
                            )>
                                <svg class="w-8 h-8 opacity-30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                                    <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" />
                                </svg>
                                <span class="text-[12px]">"Presioná + o Ctrl+N para una nueva nota"</span>
                            </div>
                        }.into_any(),
                        Some(_) => {
                            let size_class = match font_size.get() {
                                12 => "text-[12px]",
                                14 => "text-[14px]",
                                16 => "text-[16px]",
                                18 => "text-[18px]",
                                _ => "text-[14px]",
                            };
                            view! {
                                <div class=move || format!(
                                    "flex-1 flex flex-col min-h-0 py-5 overflow-y-auto no-scrollbar w-full transition-all duration-300 ease-in-out {}",
                                    if layout_width.get() == "center" { "px-20 md:px-32" } else { "px-4" }
                                )>
                                    <div
                                        node_ref=editor_ref
                                        contenteditable="true"
                                        data-placeholder="Escribí algo..."
                                        class=move || format!(
                                            "nil-editor flex-1 w-full bg-transparent focus:outline-none leading-relaxed select-text font-normal min-h-full {} {} {}",
                                            size_class,
                                            if is_dark() { "text-zinc-200" } else { "text-[#1a1918]" },
                                            if title_collapsed.get() { "title-collapsed" } else { "" }
                                        )
                                        on:input=move |_| {
                                            if let Some(el) = editor_ref.get() {
                                                trigger_content_change(el.inner_html());
                                                if title_collapsed.get_untracked() {
                                                    if let Some(h1) = el.query_selector("h1:first-child").ok().flatten() {
                                                        if h1.next_element_sibling().is_none() {
                                                            set_title_collapsed.set(false);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        on:mouseup=move |_| check_selection()
                                        on:keyup=move |_| check_selection()
                                        on:blur=move |_| set_toolbar_visible.set(false)
                                        on:keydown=on_keydown
                                    />
                                </div>
                            }.into_any()
                        },
                    }}
                </main>

                <footer
                    class=move || format!(
                        "px-4 flex items-center justify-between shrink-0 text-[11px] font-normal overflow-hidden transition-all duration-300 ease-in-out {} {}",
                        if is_dark() { "bg-[#101014] text-zinc-500" } else { "bg-[#faf8f5] text-zinc-500" },
                        if focus_mode.get() {
                            "h-0 opacity-0 translate-y-full pointer-events-none py-0"
                        } else {
                            "h-7 opacity-100 translate-y-0"
                        }
                    )
                >
                    <div class="flex items-center gap-2">
                        <div class="w-[88px] flex items-center shrink-0">
                            {move || match save_status.get() {
                                "saving" => view! {
                                    <div class="flex items-center gap-1.5 text-zinc-400">
                                        <svg class="animate-spin w-3 h-3" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3"></circle>
                                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v4a4 4 0 00-4 4H4z"></path>
                                        </svg>
                                        <span class="text-[11px]">"guardando..."</span>
                                    </div>
                                }.into_any(),
                                _ => view! {
                                    <div class="flex items-center gap-1 text-zinc-400">
                                        <svg class="w-3.5 h-3.5 text-zinc-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M20 6 9 17l-5-5" />
                                        </svg>
                                        <span class="text-[11px]">"guardado"</span>
                                    </div>
                                }.into_any(),
                            }}
                        </div>

                        <span class="opacity-20">"|"</span>

                        <button
                            class=move || format!(
                                "flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px] transition-colors cursor-pointer {}",
                                if is_dark() { "text-zinc-400 hover:text-zinc-200 hover:bg-white/5" } else { "text-zinc-600 hover:text-zinc-900 hover:bg-black/5" }
                            )
                            title="Insertar casilla (Ctrl+Shift+C)"
                            on:mousedown=move |ev| ev.prevent_default()
                            on:click=move |ev| {
                                ev.prevent_default();
                                exec("checkbox", "");
                            }
                        >
                            <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <rect x="3" y="3" width="18" height="18" rx="3" />
                                <path d="m9 12 2 2 4-4" />
                            </svg>
                            <span>"Casilla"</span>
                        </button>
                    </div>

                    {move || active_id.get().map(|_| view! {
                        <span class="text-zinc-400">
                            {move || format!("{} palabras · {} caracteres", word_count(&draft_content.get()), char_count())}
                        </span>
                    })}
                </footer>

                {move || if hide_tabs.get() || focus_mode.get() {
                    let is_open = right_panel_hover.get();
                    view! {
                        <div
                            class="absolute right-0 top-0 bottom-0 w-8 z-40 flex items-center justify-end pr-1 cursor-pointer group"
                            on:click=move |_| set_right_panel_hover.update(|o| *o = !*o)
                        >
                            <div class=move || format!(
                                "p-1 rounded-md transition-all duration-200 {}",
                                if is_open {
                                    "opacity-0 scale-90"
                                } else {
                                    if is_dark() {
                                        "opacity-30 group-hover:opacity-80 text-zinc-400 group-hover:text-zinc-200 group-hover:bg-white/[0.05]"
                                    } else {
                                        "opacity-40 group-hover:opacity-90 text-zinc-500 group-hover:text-zinc-800 group-hover:bg-black/[0.04]"
                                    }
                                }
                            )>
                                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M8 5L20 5"></path>
                                    <path d="M4 12L20 12"></path>
                                    <path d="M12 19L20 19"></path>
                                </svg>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}

                {move || if (hide_tabs.get() || focus_mode.get()) && right_panel_hover.get() {
                    view! {
                        <div
                            class="fixed inset-0 z-40 bg-transparent"
                            on:click=move |_| set_right_panel_hover.set(false)
                        />
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}

                {move || if hide_tabs.get() || focus_mode.get() {
                    let is_open = right_panel_hover.get();
                    view! {
                        <div
                            class=move || format!(
                                "absolute right-3 top-1/2 -translate-y-1/2 w-52 h-fit max-h-[calc(100%-60px)] rounded-xl p-2 flex flex-col gap-1.5 z-50 shadow-lg backdrop-blur-xl transition-all duration-200 ease-out border {} {}",
                                if is_dark() {
                                    "bg-[#141418] border-white/10"
                                } else {
                                    "bg-[#f2ede4] border-black/10"
                                },
                                if is_open { "translate-x-0 opacity-100 scale-100" } else { "translate-x-3 opacity-0 scale-95 pointer-events-none" }
                            )
                        >
                            <div class=move || format!(
                                "flex items-center justify-between px-2 py-1 text-[11px] font-medium border-b shrink-0 {}",
                                if is_dark() { "text-zinc-400 border-white/5" } else { "text-zinc-600 border-black/5" }
                            )>
                                <span class="tracking-tight">"Notas"</span>
                                <button
                                    class=move || format!(
                                        "w-5 h-5 rounded-md transition-all flex items-center justify-center cursor-pointer active:scale-95 {}",
                                        if is_dark() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.08]" } else { "text-zinc-600 hover:text-zinc-900 hover:bg-black/[0.06]" }
                                    )
                                    on:click=move |_| {
                                        new_note();
                                        set_right_panel_hover.set(false);
                                    }
                                >
                                    <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M12.001 5.00003V19.002"></path>
                                        <path d="M19.002 12.002L4.99998 12.002"></path>
                                    </svg>
                                </button>
                            </div>
                            <div class="overflow-y-auto no-scrollbar space-y-0.5 min-h-0">
                                {move || {
                                    let list = notes.get();
                                    let cur = active_id.get();
                                    list.into_iter().map(|note| {
                                        let is_active = cur.as_deref() == Some(&note.id);
                                        let note_for_click = note.clone();
                                        let note_id_close = note.id.clone();
                                        view! {
                                            <div
                                                class=move || format!(
                                                    "group flex items-center justify-between px-2.5 py-1.5 rounded-lg text-[11.5px] cursor-pointer border {}",
                                                    if is_active {
                                                        if is_dark() { "bg-white/[0.08] text-white font-medium border-white/5" } else { "bg-black/[0.06] text-zinc-900 font-medium border-black/5" }
                                                    } else {
                                                        if is_dark() { "text-zinc-400 hover:bg-white/[0.03] hover:text-zinc-200 border-transparent font-normal" } else { "text-zinc-600 hover:bg-black/[0.025] hover:text-zinc-900 border-transparent font-normal" }
                                                    }
                                                )
                                                on:click=move |_| {
                                                    select_note(note_for_click.clone());
                                                    set_right_panel_hover.set(false);
                                                }
                                            >
                                                <span class="truncate flex-1 min-w-0 pr-1">
                                                    {note_display_title(&note)}
                                                </span>
                                                <button
                                                    class=move || format!(
                                                        "w-3.5 h-3.5 rounded flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer shrink-0 {}",
                                                        if is_dark() { "text-zinc-500 hover:text-zinc-200 hover:bg-white/10" } else { "text-zinc-400 hover:text-zinc-700 hover:bg-black/10" }
                                                    )
                                                    on:click=move |ev| {
                                                        ev.stop_propagation();
                                                        delete_note_by_id(note_id_close.clone());
                                                    }
                                                >
                                                    <svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                                                        <path d="M18 6 6 18M6 6l12 12" />
                                                    </svg>
                                                </button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()
                                }}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>

            <aside
                class=move || format!(
                    "border-l shrink-0 h-full flex flex-col overflow-hidden transition-all duration-300 ease-in-out select-none {} {}",
                    if is_dark() { "border-white/5 bg-[#0c0c0e]" } else { "border-black/5 bg-[#ece7df]" },
                    if settings_open.get() { "w-72 opacity-100" } else { "w-0 opacity-0 pointer-events-none border-l-0" }
                )
            >
                <div class=move || format!(
                    "px-3.5 h-10 flex items-center justify-between border-b shrink-0 {}",
                    if is_dark() { "border-white/5" } else { "border-black/5" }
                )>
                    <span class=move || format!(
                        "text-[12px] font-medium {}",
                        if is_dark() { "text-zinc-200" } else { "text-zinc-800" }
                    )>
                        "Ajustes"
                    </span>
                    <button
                        class=move || format!(
                            "w-6 h-6 rounded-md flex items-center justify-center cursor-pointer transition-colors {}",
                            if is_dark() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.08]" } else { "text-zinc-500 hover:text-zinc-900 hover:bg-black/[0.06]" }
                        )
                        on:click=move |_| set_settings_open.set(false)
                    >
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M18 6 6 18M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                <div class="p-3.5 space-y-4 flex-1 overflow-y-auto no-scrollbar text-[12px]">
                    <div class="space-y-1.5">
                        <span class=move || format!("text-[11px] font-medium block {}", if is_dark() { "text-zinc-400" } else { "text-zinc-600" })>
                            "Tema"
                        </span>
                        <div class="grid grid-cols-2 gap-1.5">
                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if is_dark() {
                                        "bg-white/10 border-white/10 text-white"
                                    } else {
                                        "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]"
                                    }
                                )
                                on:click=move |_| set_theme.set("dark".to_string())
                            >
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M21.5 14.0784C20.3003 14.7189 18.9301 15.0821 17.4751 15.0821C12.7491 15.0821 8.91792 11.2509 8.91792 6.52485C8.91792 5.06986 9.28105 3.69968 9.92163 2.5C5.66765 3.49698 2.5 7.31513 2.5 11.8731C2.5 17.1899 6.8101 21.5 12.1269 21.5C16.6849 21.5 20.503 18.3324 21.5 14.0784Z"></path>
                                </svg>
                                <span class="text-[11px] font-medium">"Oscuro"</span>
                            </button>

                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if !is_dark() {
                                        "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold"
                                    } else {
                                        "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]"
                                    }
                                )
                                on:click=move |_| set_theme.set("light".to_string())
                            >
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <circle cx="12" cy="12" r="4" />
                                    <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" />
                                </svg>
                                <span class="text-[11px] font-medium">"Claro"</span>
                            </button>
                        </div>
                    </div>

                    <div class="space-y-1.5">
                        <span class=move || format!("text-[11px] font-medium block {}", if is_dark() { "text-zinc-400" } else { "text-zinc-600" })>
                            "Pestañas"
                        </span>
                        <div class="grid grid-cols-2 gap-1.5">
                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if !hide_tabs.get() {
                                        if is_dark() { "bg-white/10 border-white/10 text-white" } else { "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold" }
                                    } else {
                                        if is_dark() { "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]" } else { "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]" }
                                    }
                                )
                                on:click=move |_| set_hide_tabs.set(false)
                            >
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M13.5 12L16 12"></path>
                                    <path d="M8 12L10.5 12"></path>
                                    <path d="M2.5 12H5"></path>
                                </svg>
                                <span class="text-[11px] font-medium">"Visibles"</span>
                            </button>

                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if hide_tabs.get() {
                                        if is_dark() { "bg-white/10 border-white/10 text-white" } else { "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold" }
                                    } else {
                                        if is_dark() { "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]" } else { "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]" }
                                    }
                                )
                                on:click=move |_| set_hide_tabs.set(true)
                            >
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M2.5 12L21.5002 12"></path>
                                </svg>
                                <span class="text-[11px] font-medium">"Ocultas"</span>
                            </button>
                        </div>
                    </div>

                    <div class="space-y-1.5">
                        <span class=move || format!("text-[11px] font-medium block {}", if is_dark() { "text-zinc-400" } else { "text-zinc-600" })>
                            "Modo foco"
                        </span>
                        <div class="grid grid-cols-2 gap-1.5">
                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if !focus_mode.get() {
                                        if is_dark() { "bg-white/10 border-white/10 text-white" } else { "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold" }
                                    } else {
                                        if is_dark() { "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]" } else { "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]" }
                                    }
                                )
                                on:click=move |_| set_focus_mode.set(false)
                            >
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M3 12C3 7.75736 3 5.63604 4.31802 4.31802C5.63604 3 7.75736 3 12 3C16.2426 3 18.364 3 19.682 4.31802C21 5.63604 21 7.75736 21 12C21 16.2426 21 18.364 19.682 19.682C18.364 21 16.2426 21 12 21C7.75736 21 5.63604 21 4.31802 19.682C3 18.364 3 16.2426 3 12Z"></path>
                                    <path d="M3 9H21"></path>
                                </svg>
                                <span class="text-[11px] font-medium">"Desactivado"</span>
                            </button>

                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if focus_mode.get() {
                                        if is_dark() { "bg-white/10 border-white/10 text-white" } else { "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold" }
                                    } else {
                                        if is_dark() { "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]" } else { "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]" }
                                    }
                                )
                                on:click=move |_| set_focus_mode.set(true)
                            >
                                <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75">
                                    <path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" />
                                </svg>
                                <span class="text-[11px] font-medium">"Activado"</span>
                            </button>
                        </div>
                    </div>

                    <div class=move || format!(
                        "space-y-1.5 pt-1 border-t {}",
                        if is_dark() { "border-white/5" } else { "border-black/5" }
                    )>
                        <span class=move || format!("text-[11px] font-medium block {}", if is_dark() { "text-zinc-400" } else { "text-zinc-600" })>
                            "Márgenes de Texto"
                        </span>
                        <div class="grid grid-cols-2 gap-1.5">
                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if layout_width.get() != "center" {
                                        if is_dark() { "bg-white/10 border-white/10 text-white" } else { "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold" }
                                    } else {
                                        if is_dark() { "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]" } else { "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]" }
                                    }
                                )
                                on:click=move |_| set_layout_width.set("full".to_string())
                            >
                                <svg class="w-4 h-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M15 4.5H20"></path>
                                    <path d="M15 9.5H18"></path>
                                    <path d="M15 14.5H20"></path>
                                    <path d="M15 19.5H18"></path>
                                    <path d="M11 3V21"></path>
                                    <path d="M7.5 8.5L5.95782 9.74227C4.65261 10.7937 4 11.3193 4 12C4 12.6807 4.65261 13.2063 5.95782 14.2577L7.5 15.5"></path>
                                </svg>
                                <span class="text-[11px] font-medium">"Completo"</span>
                            </button>

                            <button
                                class=move || format!(
                                    "p-2 rounded-lg border flex items-center justify-center gap-2 cursor-pointer transition-colors {}",
                                    if layout_width.get() == "center" {
                                        if is_dark() { "bg-white/10 border-white/10 text-white" } else { "bg-black/[0.08] border-black/10 text-zinc-900 font-semibold" }
                                    } else {
                                        if is_dark() { "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]" } else { "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]" }
                                    }
                                )
                                on:click=move |_| set_layout_width.set("center".to_string())
                            >
                                <svg class="w-4 h-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M15 4.5H20"></path>
                                    <path d="M15 9.5H18"></path>
                                    <path d="M15 14.5H20"></path>
                                    <path d="M15 19.5H18"></path>
                                    <path d="M11 3V21"></path>
                                    <path d="M4 8.5L5.54218 9.74227C6.84739 10.7937 7.5 11.3193 7.5 12C7.5 12.6807 6.84739 13.2063 5.54218 14.2577L4 15.5"></path>
                                </svg>
                                <span class="text-[11px] font-medium">"Centrado"</span>
                            </button>
                        </div>
                    </div>



                    <div class=move || format!(
                        "space-y-1.5 pt-1 border-t {}",
                        if is_dark() { "border-white/5" } else { "border-black/5" }
                    )>
                        <div class="flex items-center justify-between">
                            <span class=move || format!("text-[11px] font-medium {}", if is_dark() { "text-zinc-400" } else { "text-zinc-600" })>
                                "Tamaño de Fuente"
                            </span>
                            <span class=move || format!("text-[11px] font-mono {}", if is_dark() { "text-zinc-500" } else { "text-zinc-400" })>
                                {move || format!("{} pt", font_size.get())}
                            </span>
                        </div>

                        <div class="grid grid-cols-4 gap-1.5 pt-0.5">
                            {[
                                (12usize, "text-[11px]"),
                                (14, "text-[13px]"),
                                (16, "text-[15px]"),
                                (18, "text-[17px]"),
                            ].into_iter().map(|(size, preview_class)| {
                                view! {
                                    <button
                                        class=move || format!(
                                            "py-2 px-1 rounded-lg border flex flex-col items-center justify-center cursor-pointer transition-all {}",
                                            if font_size.get() == size {
                                                if is_dark() {
                                                    "bg-white/10 border-white/20 text-white"
                                                } else {
                                                    "bg-black/[0.08] border-black/20 text-zinc-900 font-semibold"
                                                }
                                            } else {
                                                if is_dark() {
                                                    "bg-transparent border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.03]"
                                                } else {
                                                    "bg-transparent border-transparent text-zinc-500 hover:text-zinc-800 hover:bg-black/[0.03]"
                                                }
                                            }
                                        )
                                        on:click=move |_| set_font_size.set(size)
                                    >
                                        <span class=format!("font-medium leading-none {}", preview_class)>"Aa"</span>
                                    </button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>
            </aside>

            <aside
                class=move || format!(
                    "border-l shrink-0 h-full flex flex-col overflow-hidden transition-all duration-300 ease-in-out select-none {} {}",
                    if is_dark() { "border-white/5 bg-[#0c0c0e]" } else { "border-black/5 bg-[#ece7df]" },
                    if trash_open.get() { "w-72 opacity-100" } else { "w-0 opacity-0 pointer-events-none border-l-0" }
                )
            >
                <div class=move || format!(
                    "px-3.5 h-10 flex items-center justify-between border-b shrink-0 {}",
                    if is_dark() { "border-white/5" } else { "border-black/5" }
                )>
                    <div class="flex items-center gap-2">
                        <span class=move || format!(
                            "text-[12px] font-medium {}",
                            if is_dark() { "text-zinc-200" } else { "text-zinc-800" }
                        )>
                            "Papelera"
                        </span>
                        {move || {
                            let count = archived_notes.get().len();
                            if count > 0 {
                                view! {
                                    <span class=move || format!(
                                        "text-[10px] px-1.5 py-0.5 rounded-full {}",
                                        if is_dark() { "bg-white/10 text-zinc-400" } else { "bg-black/10 text-zinc-600" }
                                    )>
                                        {count}
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        }}
                    </div>
                    <div class="flex items-center gap-1">
                        {move || {
                            let count = archived_notes.get().len();
                            if count > 0 {
                                view! {
                                    <button
                                        class="text-[10px] text-red-500 hover:text-red-400 cursor-pointer px-1 py-0.5"
                                        on:click=move |_| empty_trash_cmd()
                                    >
                                        "Vaciar"
                                    </button>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        }}
                        <button
                            class=move || format!(
                                "w-6 h-6 rounded-md flex items-center justify-center cursor-pointer transition-colors {}",
                                if is_dark() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.08]" } else { "text-zinc-500 hover:text-zinc-900 hover:bg-black/[0.06]" }
                            )
                            on:click=move |_| set_trash_open.set(false)
                        >
                            <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M18 6 6 18M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div class="p-3.5 flex-1 overflow-y-auto no-scrollbar text-[12px]">
                    {move || {
                        let list = archived_notes.get();
                        if list.is_empty() {
                            view! {
                                <div class=move || format!(
                                    "py-8 text-center text-[11px] {}",
                                    if is_dark() { "text-zinc-600" } else { "text-zinc-400" }
                                )>
                                    "La papelera está vacía"
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-1.5">
                                    {list.into_iter().map(|note| {
                                        let id_restore = note.id.clone();
                                        let id_del = note.id.clone();
                                        let id_check = note.id.clone();
                                        view! {
                                            <div class=move || format!(
                                                "{} flex items-center justify-between px-2.5 py-2 rounded-lg text-[11.5px] border transition-colors {}",
                                                if removing_archived_ids.get().contains(&id_check) { "trash-item-exit" } else { "" },
                                                if is_dark() { "bg-white/[0.03] border-white/5 text-zinc-300" } else { "bg-black/[0.03] border-black/5 text-zinc-700" }
                                            )>
                                                <span class="truncate flex-1 min-w-0 pr-2">
                                                    {note_display_title(&note)}
                                                </span>
                                                <div class="flex items-center gap-1 shrink-0">
                                                    <button
                                                        class=move || format!(
                                                            "p-1 rounded cursor-pointer transition-colors {}",
                                                            if is_dark() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/10" } else { "text-zinc-600 hover:text-zinc-900 hover:bg-black/10" }
                                                        )
                                                        title="Restaurar nota"
                                                        on:click={
                                                            let id = id_restore.clone();
                                                            move |_| restore_note_by_id(id.clone())
                                                        }
                                                    >
                                                        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                            <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                                                            <path d="M3 3v5h5" />
                                                        </svg>
                                                    </button>
                                                    <button
                                                        class="p-1 rounded cursor-pointer text-red-500/70 hover:text-red-400 hover:bg-red-500/10 transition-colors"
                                                        title="Eliminar definitivamente"
                                                        on:click={
                                                            let id = id_del.clone();
                                                            move |_| delete_permanent_by_id(id.clone())
                                                        }
                                                    >
                                                        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                            <path d="M18 6 6 18M6 6l12 12" />
                                                        </svg>
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </aside>
        </div>
    }
}
