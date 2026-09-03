use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::KeyboardEvent;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "callTauri", catch)]
    async fn call_tauri(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardKind {
    Text,
    Image,
    Url,
    Color,
    Code,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ClipboardItem {
    pub id: String,
    pub kind: ClipboardKind,
    pub content: String,
    pub preview: String,
    pub timestamp: u64,
    pub pinned: bool,
    pub char_count: usize,
    pub metadata: Option<String>,
}

#[derive(Serialize)]
struct SelectArgs {
    id: String,
}

#[derive(Serialize)]
struct DeleteArgs {
    id: String,
}

#[derive(Serialize)]
struct PinArgs {
    id: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterKind {
    All,
    Text,
    Image,
    Url,
    Color,
    Code,
    Pinned,
}

impl FilterKind {
    fn label(&self) -> &'static str {
        match self {
            FilterKind::All => "Todos los tipos",
            FilterKind::Text => "Solo texto",
            FilterKind::Image => "Solo imágenes",
            FilterKind::Url => "Solo enlaces",
            FilterKind::Color => "Solo colores",
            FilterKind::Code => "Solo código",
            FilterKind::Pinned => "Solo fijados",
        }
    }
}



fn format_relative_time(timestamp: u64) -> String {
    let now = (js_sys::Date::now() / 1000.0) as u64;
    let diff = if now >= timestamp { now - timestamp } else { 0 };

    if diff < 60 {
        "Reciente".to_string()
    } else if diff < 3600 {
        format!("Hace {} min", diff / 60)
    } else if diff < 86400 {
        format!("Hace {} h", diff / 3600)
    } else {
        format!("Hace {} d", diff / 86400)
    }
}

fn is_today(timestamp: u64) -> bool {
    let now = (js_sys::Date::now() / 1000.0) as u64;
    let diff = if now >= timestamp { now - timestamp } else { 0 };
    diff < 86400
}

fn parse_url_parts(url: &str) -> (String, String) {
    let clean = url.trim();
    let without_proto = if let Some(rest) = clean.strip_prefix("https://") {
        rest
    } else if let Some(rest) = clean.strip_prefix("http://") {
        rest
    } else {
        clean
    };

    if let Some(idx) = without_proto.find('/') {
        let domain = &without_proto[..idx];
        let path = &without_proto[idx..];
        (domain.to_string(), path.to_string())
    } else {
        (without_proto.to_string(), "/".to_string())
    }
}

fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().strip_prefix('#')?;
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
        Some((r, g, b))
    } else if clean.len() == 3 {
        let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

fn detect_code_language(code: &str) -> &'static str {
    let c = code.trim();
    if c.contains("fn ") || c.contains("let mut ") || c.contains("impl ") || c.contains("pub struct") {
        "Rust"
    } else if c.contains("const ") || c.contains("function ") || c.contains("=>") || c.contains("import React") {
        "JavaScript"
    } else if c.contains("def ") || c.contains("import ") || c.contains("print(") {
        "Python"
    } else if c.contains("SELECT ") || c.contains("FROM ") || c.contains("WHERE ") {
        "SQL"
    } else if c.contains("sudo ") || c.contains("curl ") || c.contains("cargo ") || c.contains("git ") {
        "Shell"
    } else if c.contains('{') && c.contains(':') && c.contains(';') {
        "CSS"
    } else {
        "Código"
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (items, set_items) = signal(Vec::<ClipboardItem>::new());
    let (search_query, set_search_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0usize);
    let (active_filter, set_active_filter) = signal(FilterKind::All);
    let (is_filter_menu_open, set_is_filter_menu_open) = signal(false);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let fetch_history = move || {
        leptos::task::spawn_local(async move {
            let args = JsValue::NULL;
            if let Ok(res) = call_tauri("get_clipboard_history", args).await {
                if let Ok(history) = serde_wasm_bindgen::from_value::<Vec<ClipboardItem>>(res) {
                    set_items.set(history);
                }
            }
        });
    };

    Effect::new(move |_| {
        fetch_history();
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    let filtered_items = move || {
        let q = search_query.get().to_lowercase();
        let f = active_filter.get();
        let all = items.get();

        all.into_iter()
            .filter(|item| match f {
                FilterKind::All => true,
                FilterKind::Pinned => item.pinned,
                FilterKind::Text => item.kind == ClipboardKind::Text,
                FilterKind::Image => item.kind == ClipboardKind::Image,
                FilterKind::Url => item.kind == ClipboardKind::Url,
                FilterKind::Color => item.kind == ClipboardKind::Color,
                FilterKind::Code => item.kind == ClipboardKind::Code,
            })
            .filter(|item| {
                if q.is_empty() {
                    return true;
                }
                item.content.to_lowercase().contains(&q)
                    || item.metadata.as_deref().unwrap_or("").to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    };

    let select_item = move |id: String| {
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&SelectArgs { id }).unwrap_or(JsValue::NULL);
            let _ = call_tauri("select_clipboard_item", args).await;
        });
    };

    let delete_item_by_id = move |id: String| {
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&DeleteArgs { id }).unwrap_or(JsValue::NULL);
            let _ = call_tauri("delete_item", args).await;
            fetch_history();
        });
    };

    let toggle_pin_by_id = move |id: String| {
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&PinArgs { id }).unwrap_or(JsValue::NULL);
            let _ = call_tauri("toggle_pin_item", args).await;
            fetch_history();
        });
    };

    let close_app = move || {
        leptos::task::spawn_local(async move {
            let _ = call_tauri("close_window", JsValue::NULL).await;
        });
    };

    let on_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        let list = filtered_items();
        let total = list.len();
        let cur = selected_index.get();

        if is_filter_menu_open.get_untracked() {
            if key == "Escape" || key == "Tab" {
                ev.prevent_default();
                set_is_filter_menu_open.set(false);
            }
            return;
        }

        if key == "Escape" {
            ev.prevent_default();
            close_app();
        } else if key == "ArrowDown" {
            ev.prevent_default();
            if total > 0 {
                set_selected_index.set((cur + 1).min(total - 1));
            }
        } else if key == "ArrowUp" {
            ev.prevent_default();
            if total > 0 && cur > 0 {
                set_selected_index.set(cur - 1);
            }
        } else if key == "Tab" {
            ev.prevent_default();
            set_is_filter_menu_open.set(!is_filter_menu_open.get_untracked());
        } else if key == "Enter" {
            ev.prevent_default();
            if let Some(item) = list.get(cur) {
                select_item(item.id.clone());
            }
        } else if key == "Delete" || (ev.ctrl_key() && key == "d") {
            ev.prevent_default();
            if let Some(item) = list.get(cur) {
                delete_item_by_id(item.id.clone());
            }
        } else if ev.ctrl_key() && (key == "p" || key == "P") {
            ev.prevent_default();
            if let Some(item) = list.get(cur) {
                toggle_pin_by_id(item.id.clone());
            }
        } else if ev.alt_key() {
            if let Ok(digit) = key.parse::<usize>() {
                if digit >= 1 && digit <= 9 && digit <= total {
                    ev.prevent_default();
                    if let Some(item) = list.get(digit - 1) {
                        select_item(item.id.clone());
                    }
                }
            }
        }
    };

    view! {
        <div
            class="h-screen w-screen bg-[#09090b] text-zinc-100 flex flex-col select-none border border-white/10 rounded-xl overflow-hidden shadow-2xl ring-1 ring-white/5"
        >
            <header class="h-10 px-3 flex items-center justify-between border-b border-white/5 bg-zinc-900/40 shrink-0 relative z-30">
                <div class="flex items-center gap-2.5 flex-1 min-w-0 pr-3">
                    <div class="text-zinc-400 flex items-center justify-center w-3.5 h-3.5 shrink-0">
                        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="11" cy="11" r="8" />
                            <path d="m21 21-4.3-4.3" />
                        </svg>
                    </div>

                    <input
                        node_ref=input_ref
                        type="text"
                        placeholder="Buscar en el portapapeles..."
                        class="w-full bg-transparent text-[12.5px] text-zinc-100 placeholder-zinc-500 focus:outline-none tracking-tight font-normal"
                        prop:value=move || search_query.get()
                        on:keydown=on_keydown
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            set_search_query.set(val);
                            set_selected_index.set(0);
                        }
                    />
                </div>

                    <div class="relative">
                        <button
                            class="flex items-center gap-1.5 px-2 py-0.5 rounded bg-white/[0.03] hover:bg-white/[0.06] border border-white/10 text-[11px] text-zinc-300 transition-colors cursor-pointer"
                            on:click=move |_| {
                                set_is_filter_menu_open.set(!is_filter_menu_open.get_untracked());
                            }
                        >
                            <span>{move || active_filter.get().label()}</span>
                            <svg class="w-2.5 h-2.5 text-zinc-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="m6 9 6 6 6-6" />
                            </svg>
                        </button>

                        {move || if is_filter_menu_open.get() {
                            view! {
                                <div class="absolute right-0 top-full mt-1.5 w-40 bg-zinc-900 border border-white/10 rounded-lg shadow-2xl p-1 z-50 flex flex-col gap-0.5 animate-fade-in ring-1 ring-white/5">
                                    {[
                                        FilterKind::All,
                                        FilterKind::Text,
                                        FilterKind::Image,
                                        FilterKind::Url,
                                        FilterKind::Color,
                                        FilterKind::Code,
                                        FilterKind::Pinned,
                                    ].into_iter().map(|f| {
                                        let is_active = active_filter.get() == f;
                                        view! {
                                            <button
                                                class=format!(
                                                    "w-full text-left px-2 py-1 rounded text-[11px] transition-colors flex items-center justify-between cursor-pointer {}",
                                                    if is_active { "bg-white/10 text-white font-medium" } else { "text-zinc-400 hover:text-zinc-200 hover:bg-white/5" }
                                                )
                                                on:click=move |_| {
                                                    set_active_filter.set(f);
                                                    set_is_filter_menu_open.set(false);
                                                    set_selected_index.set(0);
                                                }
                                            >
                                                <span>{f.label()}</span>
                                                {if is_active {
                                                    view! {
                                                        <svg class="w-2.5 h-2.5 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                                                            <path d="M20 6 9 17l-5-5" />
                                                        </svg>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                    </div>
            </header>

            <div class="flex-1 flex min-h-0 overflow-hidden">
                <div class="w-[46%] border-r border-white/5 flex flex-col min-h-0 bg-[#09090b]">
                    <div class="flex-1 overflow-y-auto p-1 space-y-0.5 no-scrollbar">
                        {move || {
                            let list = filtered_items();
                            let cur = selected_index.get();

                            if list.is_empty() {
                                view! {
                                    <div class="h-full flex flex-col items-center justify-center text-zinc-500 py-12 text-center">
                                        <span class="text-xs font-normal">"Sin coincidencias"</span>
                                    </div>
                                }.into_any()
                            } else {
                                let mut last_was_today = true;
                                let mut rendered_items = Vec::new();

                                for (idx, item) in list.into_iter().enumerate() {
                                    let is_item_today = is_today(item.timestamp);
                                    let is_selected = idx == cur;
                                    let id_for_click = item.id.clone();

                                    if idx == 0 {
                                        rendered_items.push(view! {
                                            <div class="px-2 pt-1.5 pb-0.5 text-[9.5px] font-medium tracking-wider uppercase text-zinc-500">
                                                {if is_item_today { "Hoy" } else { "Anteriores" }}
                                            </div>
                                        }.into_any());
                                        last_was_today = is_item_today;
                                    } else if last_was_today && !is_item_today {
                                        rendered_items.push(view! {
                                            <div class="px-2 pt-2.5 pb-0.5 text-[9.5px] font-medium tracking-wider uppercase text-zinc-500">
                                                "Anteriores"
                                            </div>
                                        }.into_any());
                                        last_was_today = false;
                                    }

                                    rendered_items.push(view! {
                                        <div
                                            class=format!(
                                                "group flex items-center gap-2 px-2 py-1.5 rounded-md text-[11.5px] font-medium transition-colors duration-75 cursor-pointer {}",
                                                if is_selected {
                                                    "bg-white/[0.08] text-white"
                                                } else {
                                                    "text-zinc-400 hover:bg-white/[0.03] hover:text-zinc-200"
                                                }
                                            )
                                            on:click=move |_| select_item(id_for_click.clone())
                                            on:mouseenter=move |_| set_selected_index.set(idx)
                                        >
                                            <div class="shrink-0 flex items-center justify-center w-3.5 h-3.5">
                                                {match item.kind {
                                                    ClipboardKind::Color => {
                                                        let hex = item.content.clone();
                                                        view! {
                                                            <span
                                                                class="w-3 h-3 rounded-full border border-white/20 shadow-sm shrink-0"
                                                                style=format!("background-color: {}", hex)
                                                            ></span>
                                                        }.into_any()
                                                    },
                                                    ClipboardKind::Image => {
                                                        view! {
                                                            <svg class="w-3.5 h-3.5 text-zinc-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                                                <rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
                                                                <circle cx="9" cy="9" r="2" />
                                                                <path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" />
                                                            </svg>
                                                        }.into_any()
                                                    },
                                                    ClipboardKind::Url => {
                                                        view! {
                                                            <svg class="w-3.5 h-3.5 text-zinc-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                                                <circle cx="12" cy="12" r="10" />
                                                                <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
                                                                <path d="M2 12h20" />
                                                            </svg>
                                                        }.into_any()
                                                    },
                                                    ClipboardKind::Code => {
                                                        view! {
                                                            <svg class="w-3.5 h-3.5 text-zinc-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                                                <polyline points="16 18 22 12 16 6" />
                                                                <polyline points="8 6 2 12 8 18" />
                                                            </svg>
                                                        }.into_any()
                                                    },
                                                    ClipboardKind::Text => {
                                                        view! {
                                                            <svg class="w-3.5 h-3.5 text-zinc-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                                                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                                                                <path d="M14 2v6h6" />
                                                            </svg>
                                                        }.into_any()
                                                    },
                                                }}
                                            </div>

                                            <span class="flex-1 truncate tracking-tight">
                                                {match item.kind {
                                                    ClipboardKind::Url => {
                                                        let (domain, path) = parse_url_parts(&item.content);
                                                        format!("{}{}", domain, if path == "/" { "".to_string() } else { path })
                                                    },
                                                    ClipboardKind::Image => item.metadata.clone().unwrap_or_else(|| "Captura de pantalla".to_string()),
                                                    _ => item.preview.clone(),
                                                }}
                                            </span>

                                            {if item.pinned {
                                                view! {
                                                    <svg class="w-2.5 h-2.5 text-zinc-300 shrink-0" viewBox="0 0 24 24" fill="currentColor">
                                                        <path d="M12 17v5M5 12V7a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v5l2 2v1H3v-1l2-2z" />
                                                    </svg>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </div>
                                    }.into_any());
                                }

                                rendered_items.into_any()
                            }
                        }}
                    </div>
                </div>

                <div class="w-[54%] flex flex-col min-h-0 bg-zinc-900/10">
                    {move || {
                        let list = filtered_items();
                        let cur = selected_index.get();

                        if let Some(item) = list.get(cur) {
                            let type_str = match item.kind {
                                ClipboardKind::Color => "Color",
                                ClipboardKind::Image => "Imagen",
                                ClipboardKind::Url => "Enlace",
                                ClipboardKind::Code => detect_code_language(&item.content),
                                ClipboardKind::Text => "Texto",
                            };

                            let time_formatted = format_relative_time(item.timestamp);
                            let content_clone = item.content.clone();
                            let preview_clone = item.preview.clone();

                            view! {
                                <div class="flex-1 flex flex-col min-h-0">
                                    <div class="flex-1 flex flex-col items-center justify-center p-3 border-b border-white/5 overflow-hidden">
                                        {match item.kind {
                                            ClipboardKind::Color => {
                                                let hex = content_clone.clone();
                                                let rgb_str = hex_to_rgb(&hex).map(|(r, g, b)| format!("rgb({}, {}, {})", r, g, b)).unwrap_or_default();
                                                view! {
                                                    <div class="flex flex-col items-center justify-center gap-2">
                                                        <span
                                                            class="w-16 h-16 rounded-full border border-white/20 shadow-xl"
                                                            style=format!("background-color: {}", hex)
                                                        ></span>
                                                        <div class="text-center">
                                                            <div class="text-xs font-mono font-medium tracking-wider text-zinc-100 uppercase">
                                                                {hex}
                                                            </div>
                                                            <div class="text-[10px] font-mono text-zinc-500 mt-0.5">
                                                                {rgb_str}
                                                            </div>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            },
                                            ClipboardKind::Url => {
                                                let (domain, path) = parse_url_parts(&content_clone);
                                                view! {
                                                    <div class="w-full flex flex-col items-center text-center p-2">
                                                        <div class="w-8 h-8 rounded-lg bg-white/[0.04] border border-white/10 flex items-center justify-center text-zinc-300 mb-2">
                                                            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                                                <circle cx="12" cy="12" r="10" />
                                                                <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
                                                                <path d="M2 12h20" />
                                                            </svg>
                                                        </div>
                                                        <div class="text-[12.5px] font-medium text-zinc-100 tracking-tight">
                                                            {domain}
                                                        </div>
                                                        <div class="text-[10.5px] text-zinc-500 font-mono mt-1 break-all max-h-16 overflow-y-auto no-scrollbar">
                                                            {path}
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            },
                                            ClipboardKind::Image => {
                                                view! {
                                                    <div class="flex flex-col items-center justify-center max-h-full max-w-full">
                                                        <img
                                                            src=preview_clone
                                                            class="max-h-full max-w-full rounded-md border border-white/10 shadow-sm object-contain bg-black/40"
                                                        />
                                                    </div>
                                                }.into_any()
                                            },
                                            ClipboardKind::Code => {
                                                let line_count = content_clone.lines().count();
                                                view! {
                                                    <div class="w-full h-full flex flex-col bg-zinc-950/60 border border-white/5 rounded-lg overflow-hidden">
                                                        <div class="px-2 py-1 bg-white/[0.02] border-b border-white/5 flex items-center justify-between text-[9.5px] text-zinc-500 font-mono">
                                                            <span>{type_str}</span>
                                                            <span>{format!("{} líneas", line_count)}</span>
                                                        </div>
                                                        <div class="flex-1 p-2 overflow-y-auto text-[11px] font-mono text-zinc-300 leading-relaxed whitespace-pre-wrap no-scrollbar select-text">
                                                            {content_clone}
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            },
                                            ClipboardKind::Text => {
                                                let words = content_clone.split_whitespace().count();
                                                view! {
                                                    <div class="w-full h-full flex flex-col bg-zinc-950/40 border border-white/5 rounded-lg overflow-hidden">
                                                        <div class="px-2 py-1 bg-white/[0.02] border-b border-white/5 flex items-center justify-between text-[9.5px] text-zinc-500">
                                                            <span>"Texto"</span>
                                                            <span>{format!("{} palabras", words)}</span>
                                                        </div>
                                                        <div class="flex-1 p-2 overflow-y-auto text-[11.5px] text-zinc-200 leading-relaxed whitespace-pre-wrap no-scrollbar select-text">
                                                            {content_clone}
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                    </div>

                                    <div class="p-2.5 bg-zinc-950/30 text-[10px] space-y-1 shrink-0">
                                        <div class="flex items-center justify-between text-zinc-500">
                                            <span>"Tipo"</span>
                                            <span class="text-zinc-300">{type_str}</span>
                                        </div>

                                        <div class="flex items-center justify-between text-zinc-500">
                                            <span>"Copiado"</span>
                                            <span class="text-zinc-300">{time_formatted}</span>
                                        </div>

                                        {match item.kind {
                                            ClipboardKind::Image => {
                                                view! {
                                                    <div class="flex items-center justify-between text-zinc-500">
                                                        <span>"Resolución"</span>
                                                        <span class="text-zinc-300 font-mono">{item.metadata.clone().unwrap_or_default()}</span>
                                                    </div>
                                                }.into_any()
                                            },
                                            _ => {
                                                view! {
                                                    <div class="flex items-center justify-between text-zinc-500">
                                                        <span>"Caracteres"</span>
                                                        <span class="text-zinc-300 font-mono">{item.char_count}</span>
                                                    </div>
                                                }.into_any()
                                            }
                                        }}

                                        {if item.pinned {
                                            view! {
                                                <div class="flex items-center justify-between text-zinc-500">
                                                    <span>"Fijado"</span>
                                                    <span class="text-zinc-300 font-medium">"Sí"</span>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex-1 flex items-center justify-center text-zinc-500 text-[11px]">
                                    "Selecciona un elemento"
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            <footer class="h-7 px-3 border-t border-white/5 bg-zinc-900/40 flex items-center justify-between text-[10px] text-zinc-500 shrink-0">
                <div class="flex items-center gap-1">
                    <span class="font-mono text-zinc-400">"nil-clip"</span>
                </div>

                <div class="flex items-center gap-2.5">
                    <span class="flex items-center gap-1 text-zinc-400">
                        <span>"Copiar"</span>
                        <kbd class="px-1 py-0.2 rounded bg-white/[0.06] text-zinc-200 font-mono text-[9px]">"↵"</kbd>
                    </span>

                    <span class="text-zinc-700">"·"</span>

                    <span class="flex items-center gap-1 text-zinc-500">
                        <span>"Fijar"</span>
                        <kbd class="px-1 py-0.2 rounded bg-white/[0.04] text-zinc-400 font-mono text-[9px]">"Ctrl+P"</kbd>
                    </span>

                    <span class="text-zinc-700">"·"</span>

                    <span class="flex items-center gap-1 text-zinc-500">
                        <span>"Borrar"</span>
                        <kbd class="px-1 py-0.2 rounded bg-white/[0.04] text-zinc-400 font-mono text-[9px]">"Del"</kbd>
                    </span>

                    <span class="text-zinc-700">"·"</span>

                    <span class="flex items-center gap-1 text-zinc-500">
                        <span>"Salir"</span>
                        <kbd class="px-1 py-0.2 rounded bg-white/[0.04] text-zinc-400 font-mono text-[9px]">"Esc"</kbd>
                    </span>
                </div>
            </footer>
        </div>
    }
}
