use leptos::ev;
use leptos::html::Canvas;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement, KeyboardEvent, MouseEvent, WheelEvent};

const COLOR_LIST: &[(&str, &str)] = &[
    ("#e06c75", "#e06c75"),
    ("#e59866", "#e59866"),
    ("#e5c07b", "#e5c07b"),
    ("#98c379", "#98c379"),
    ("#56b6c2", "#56b6c2"),
    ("#61afef", "#61afef"),
    ("#c678dd", "#c678dd"),
    ("#e4e4e7", "#e4e4e7"),
    ("#27272a", "#27272a"),
];

const WIDTHS: &[(f64, &str)] = &[
    (2.5, "h-[2px]"),
    (5.0, "h-[4px]"),
    (10.0, "h-[7px]"),
    (18.0, "h-[12px]"),
];

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = callTauri, catch)]
    async fn call_tauri(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResizeArgs {
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CopyArgs {
    #[serde(rename = "dataBase64")]
    pub data_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CopyTextArgs {
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SaveArgs {
    #[serde(rename = "dataBase64")]
    pub data_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OcrArgs {
    #[serde(rename = "dataBase64")]
    pub data_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OpenUrlArgs {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OcrBlock {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Pen,
    Highlighter,
    Arrow,
    Rectangle,
    Circle,
    Blur,
    Picker,
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CropHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

#[derive(Clone, Debug)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Debug)]
pub enum DrawingItem {
    Freehand {
        points: Vec<Point>,
        color: String,
        width: f64,
        is_highlighter: bool,
    },
    Arrow {
        start: Point,
        end: Point,
        color: String,
        width: f64,
    },
    Rectangle {
        start: Point,
        end: Point,
        color: String,
        width: f64,
    },
    Circle {
        start: Point,
        end: Point,
        color: String,
        width: f64,
    },
    Blur {
        start: Point,
        end: Point,
    },
}

#[derive(Clone, Debug)]
pub enum HistoryAction {
    Add(DrawingItem),
    Clear(Vec<DrawingItem>),
    Crop {
        prev_image_data: String,
        prev_items: Vec<DrawingItem>,
        prev_width: u32,
        prev_height: u32,
        new_image_data: String,
        new_items: Vec<DrawingItem>,
        new_width: u32,
        new_height: u32,
    },
}

fn calculate_aspect_ratio_str(w: f64, h: f64) -> String {
    let w_u = w.round() as u32;
    let h_u = h.round() as u32;
    if w_u == 0 || h_u == 0 {
        return String::new();
    }
    let ratio = w / h;
    let known: &[((u32, u32), f64)] = &[
        ((1, 1), 1.0),
        ((16, 9), 16.0 / 9.0),
        ((16, 10), 16.0 / 10.0),
        ((4, 3), 4.0 / 3.0),
        ((3, 2), 3.0 / 2.0),
        ((21, 9), 21.0 / 9.0),
        ((9, 16), 9.0 / 16.0),
        ((3, 4), 3.0 / 4.0),
        ((2, 3), 2.0 / 3.0),
    ];
    for &((rw, rh), val) in known {
        if (ratio - val).abs() < 0.035 {
            return format!("{} × {} · {}:{}", w_u, h_u, rw, rh);
        }
    }
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }
    let g = gcd(w_u, h_u);
    let rw = w_u / g;
    let rh = h_u / g;
    if rw < 50 && rh < 50 {
        format!("{} × {} · {}:{}", w_u, h_u, rw, rh)
    } else {
        format!("{} × {} · {:.2}:1", w_u, h_u, ratio)
    }
}

async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    let _ = JsFuture::from(promise).await;
}

fn detect_url(text: &str) -> Option<String> {
    let t = text.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        Some(t.to_string())
    } else if t.starts_with("www.") {
        Some(format!("https://{}", t))
    } else if (t.starts_with("github.com") || t.starts_with("gitlab.com") || t.starts_with("twitter.com") || t.starts_with("x.com")) && !t.contains(' ') {
        Some(format!("https://{}", t))
    } else {
        None
    }
}

fn detect_color(text: &str) -> Option<String> {
    let t = text.trim();
    if t.starts_with('#') {
        let hex = &t[1..];
        if (hex.len() == 3 || hex.len() == 4 || hex.len() == 6 || hex.len() == 8)
            && hex.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Some(t.to_string());
        }
    } else if (t.starts_with("rgb(") || t.starts_with("rgba(") || t.starts_with("hsl(") || t.starts_with("hsla(")) && t.ends_with(')') {
        return Some(t.to_string());
    }
    None
}

#[component]
pub fn App() -> impl IntoView {
    let (active_tool, set_active_tool) = signal(Tool::Pen);
    let (color, set_color) = signal("#e06c75".to_string());
    let (stroke_width, set_stroke_width) = signal(4.0f64);
    let (status_text, set_status_text) = signal(String::new());
    let (show_about, set_show_about) = signal(false);
    let (is_about_closing, set_is_about_closing) = signal(false);
    let (dimension_text, set_dimension_text) = signal(String::new());
    let (is_cropping_active, set_is_cropping_active) = signal(false);
    let (picker_preview, set_picker_preview) = signal(None::<(f64, f64, String)>);

    let (is_pinned, set_is_pinned) = signal(false);
    let (is_tiling, set_is_tiling) = signal(false);
    let (is_ocr_active, set_is_ocr_active) = signal(false);
    let (ocr_blocks, set_ocr_blocks) = signal(Vec::<OcrBlock>::new());
    let (selected_ocr_indices, set_selected_ocr_indices) = signal(BTreeSet::<usize>::new());
    let (is_ocr_loading, set_is_ocr_loading) = signal(false);

    let (zoom_level, set_zoom_level) = signal(1.0f64);
    let (pan_offset, set_pan_offset) = signal((0.0f64, 0.0f64));

    let (can_undo, set_can_undo) = signal(false);
    let (can_redo, set_can_redo) = signal(false);
    let (can_clear, set_can_clear) = signal(false);
    let (is_panning_ui, set_is_panning_ui) = signal(false);
    let (toast_gen, set_toast_gen) = signal(0u32);
    let (copy_success, set_copy_success) = signal(false);
    let (copy_success_gen, set_copy_success_gen) = signal(0u32);
    let (save_success, set_save_success) = signal(false);
    let (save_success_gen, set_save_success_gen) = signal(0u32);
    let (image_dimensions, set_image_dimensions) = signal((1.0f64, 1.0f64));

    let canvas_ref = NodeRef::<Canvas>::new();
    let loupe_canvas_ref = NodeRef::<Canvas>::new();
    let preview_scroll_ref = NodeRef::<leptos::html::Div>::new();
    let base_image = Rc::new(RefCell::new(None::<HtmlImageElement>));
    let items = Rc::new(RefCell::new(Vec::<DrawingItem>::new()));
    let history = Rc::new(RefCell::new(Vec::<HistoryAction>::new()));
    let future = Rc::new(RefCell::new(Vec::<HistoryAction>::new()));
    let is_drawing = Rc::new(RefCell::new(false));
    let is_panning = Rc::new(RefCell::new(false));
    let pan_start_mouse = Rc::new(RefCell::new((0.0f64, 0.0f64)));
    let pan_initial = Rc::new(RefCell::new((0.0f64, 0.0f64)));
    let start_point = Rc::new(RefCell::new(None::<Point>));
    let current_points = Rc::new(RefCell::new(Vec::<Point>::new()));

    let crop_handle_state = Rc::new(RefCell::new(None::<CropHandle>));
    let crop_start_mouse = Rc::new(RefCell::new(Point { x: 0.0, y: 0.0 }));
    let current_crop_rect = Rc::new(RefCell::new(Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }));

    let notify = move |msg: String| {
        let gen = toast_gen.get_untracked().wrapping_add(1);
        set_toast_gen.set(gen);
        set_status_text.set(msg);
        leptos::task::spawn_local(async move {
            sleep_ms(2800).await;
            if toast_gen.get_untracked() == gen {
                set_status_text.set(String::new());
            }
        });
    };

    let close_about = move || {
        if show_about.get_untracked() && !is_about_closing.get_untracked() {
            set_is_about_closing.set(true);
            leptos::task::spawn_local(async move {
                sleep_ms(160).await;
                set_show_about.set(false);
                set_is_about_closing.set(false);
            });
        }
    };

    let draw_arrow_head = |ctx: &CanvasRenderingContext2d, from: &Point, to: &Point, width: f64| {
        let head_len = (width * 3.5).max(12.0);
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let angle = dy.atan2(dx);

        ctx.begin_path();
        ctx.move_to(to.x, to.y);
        ctx.line_to(
            to.x - head_len * (angle - std::f64::consts::PI / 6.0).cos(),
            to.y - head_len * (angle - std::f64::consts::PI / 6.0).sin(),
        );
        ctx.line_to(
            to.x - head_len * (angle + std::f64::consts::PI / 6.0).cos(),
            to.y - head_len * (angle + std::f64::consts::PI / 6.0).sin(),
        );
        ctx.close_path();
        ctx.fill();
    };

    let apply_pixelate = |ctx: &CanvasRenderingContext2d, img: &HtmlImageElement, x: f64, y: f64, w: f64, h: f64| {
        if w <= 2.0 || h <= 2.0 {
            return;
        }
        let block_size = 12.0;
        let small_w = (w / block_size).max(1.0).round();
        let small_h = (h / block_size).max(1.0).round();

        let doc = match web_sys::window().and_then(|win| win.document()) {
            Some(d) => d,
            None => return,
        };
        let off_canvas: HtmlCanvasElement = match doc.create_element("canvas").ok().and_then(|el| el.dyn_into().ok()) {
            Some(c) => c,
            None => return,
        };
        off_canvas.set_width(small_w as u32);
        off_canvas.set_height(small_h as u32);

        let off_ctx = match off_canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok()) {
            Some(c) => c,
            None => return,
        };

        off_ctx.set_image_smoothing_enabled(true);
        let _ = off_ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            img,
            x, y, w, h,
            0.0, 0.0, small_w, small_h,
        );

        ctx.save();
        ctx.set_image_smoothing_enabled(false);
        let _ = ctx.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &off_canvas,
            0.0, 0.0, small_w, small_h,
            x, y, w, h,
        );
        ctx.restore();
    };

    let redraw_canvas = {
        let canvas_ref = canvas_ref.clone();
        let base_image = base_image.clone();
        let items = items.clone();
        Rc::new(move || {
            let canvas = match canvas_ref.get() {
                Some(c) => c,
                None => return,
            };
            let ctx = match canvas
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
            {
                Some(ctx) => ctx,
                None => return,
            };

            let width = canvas.width() as f64;
            let height = canvas.height() as f64;
            ctx.clear_rect(0.0, 0.0, width, height);

            if let Some(ref img) = *base_image.borrow() {
                let _ = ctx.draw_image_with_html_image_element(img, 0.0, 0.0);
            }

            let item_list = items.borrow().clone();
            for item in item_list.iter() {
                match item {
                    DrawingItem::Blur { start, end } => {
                        let x = start.x.min(end.x);
                        let y = start.y.min(end.y);
                        let w = (start.x - end.x).abs();
                        let h = (start.y - end.y).abs();
                        if w > 1.0 && h > 1.0 {
                            if let Some(ref img) = *base_image.borrow() {
                                apply_pixelate(&ctx, img, x, y, w, h);
                            }
                        }
                    }
                    DrawingItem::Freehand {
                        points,
                        color,
                        width,
                        is_highlighter,
                    } => {
                        if points.is_empty() {
                            continue;
                        }
                        ctx.save();
                        if *is_highlighter {
                            ctx.set_global_alpha(0.35);
                        }
                        ctx.set_stroke_style_str(color);
                        ctx.set_fill_style_str(color);
                        ctx.set_line_width(*width);
                        ctx.set_line_cap("round");
                        ctx.set_line_join("round");

                        if points.len() == 1 {
                            ctx.begin_path();
                            let _ = ctx.arc(points[0].x, points[0].y, *width / 2.0, 0.0, std::f64::consts::PI * 2.0);
                            ctx.fill();
                        } else {
                            ctx.begin_path();
                            ctx.move_to(points[0].x, points[0].y);
                            for pt in points.iter().skip(1) {
                                ctx.line_to(pt.x, pt.y);
                            }
                            ctx.stroke();
                        }
                        ctx.restore();
                    }
                    DrawingItem::Arrow {
                        start,
                        end,
                        color,
                        width,
                    } => {
                        ctx.save();
                        ctx.set_stroke_style_str(color);
                        ctx.set_fill_style_str(color);
                        ctx.set_line_width(*width);
                        ctx.set_line_cap("round");
                        ctx.begin_path();
                        ctx.move_to(start.x, start.y);
                        ctx.line_to(end.x, end.y);
                        ctx.stroke();
                        draw_arrow_head(&ctx, start, end, *width);
                        ctx.restore();
                    }
                    DrawingItem::Rectangle {
                        start,
                        end,
                        color,
                        width,
                    } => {
                        let x = start.x.min(end.x);
                        let y = start.y.min(end.y);
                        let w = (start.x - end.x).abs();
                        let h = (start.y - end.y).abs();
                        ctx.save();
                        ctx.set_stroke_style_str(color);
                        ctx.set_line_width(*width);
                        ctx.set_line_join("round");
                        ctx.stroke_rect(x, y, w, h);
                        ctx.restore();
                    }
                    DrawingItem::Circle {
                        start,
                        end,
                        color,
                        width,
                    } => {
                        let cx = (start.x + end.x) / 2.0;
                        let cy = (start.y + end.y) / 2.0;
                        let rx = ((start.x - end.x).abs() / 2.0).max(0.1);
                        let ry = ((start.y - end.y).abs() / 2.0).max(0.1);
                        ctx.save();
                        ctx.set_stroke_style_str(color);
                        ctx.set_line_width(*width);
                        ctx.begin_path();
                        let _ = ctx.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::TAU);
                        ctx.stroke();
                        ctx.restore();
                    }
                }
            }
        })
    };

    let switch_image = {
        let canvas_ref = canvas_ref.clone();
        let base_image = base_image.clone();
        let redraw_canvas = redraw_canvas.clone();
        Rc::new(move |img_src: String, w: u32, h: u32| {
            let img = HtmlImageElement::new().unwrap();
            let img_clone = img.clone();
            let base_image_clone = base_image.clone();
            let redraw = redraw_canvas.clone();
            let canvas_ref_clone = canvas_ref.clone();

            let onload = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                if let Some(canvas) = canvas_ref_clone.get() {
                    canvas.set_width(w);
                    canvas.set_height(h);
                }
                set_image_dimensions.set((w as f64, h as f64));
                *base_image_clone.borrow_mut() = Some(img_clone.clone());
                set_dimension_text.set(calculate_aspect_ratio_str(w as f64, h as f64));
                redraw();

                leptos::task::spawn_local(async move {
                    let args = serde_wasm_bindgen::to_value(&ResizeArgs {
                        width: w as f64,
                        height: h as f64,
                    })
                    .unwrap_or(JsValue::NULL);
                    let _ = call_tauri("resize_window", args).await;
                });
            }));

            img.set_onload(Some(onload.as_ref().unchecked_ref()));
            onload.forget();
            img.set_src(&img_src);
        })
    };

    Effect::new({
        let switch_image = switch_image.clone();
        move |_| {
            leptos::task::spawn_local({
                let switch_img = switch_image.clone();
                async move {
                    if let Ok(val) = call_tauri("get_image_data", JsValue::NULL).await {
                        if let Some(img_data_url) = val.as_string() {
                            let img = HtmlImageElement::new().unwrap();
                            let img_clone = img.clone();
                            let switch_img_clone = switch_img.clone();

                            let onload = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                                let w = img_clone.natural_width();
                                let h = img_clone.natural_height();
                                switch_img_clone(img_clone.src(), w, h);
                            }));

                            img.set_onload(Some(onload.as_ref().unchecked_ref()));
                            onload.forget();
                            img.set_src(&img_data_url);
                        }
                    }
                }
            });
        }
    });

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(val) = call_tauri("is_tiling_desktop", JsValue::NULL).await {
                if let Some(b) = val.as_bool() {
                    set_is_tiling.set(b);
                }
            }
        });
    });

    let toggle_pin = move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(val) = call_tauri("toggle_pin_window", JsValue::NULL).await {
                if let Some(pinned) = val.as_bool() {
                    set_is_pinned.set(pinned);
                    notify(if pinned { "Fijado siempre visible".to_string() } else { "Desfijado".to_string() });
                }
            }
        });
    };

    let copy_text_direct = move |text_str: String| {
        let t_clone = text_str.clone();
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&CopyTextArgs {
                text: t_clone,
            })
            .unwrap_or(JsValue::NULL);
            let _ = call_tauri("copy_text_to_clipboard", args).await;
            notify("Texto copiado al portapapeles".to_string());
        });
    };

    let open_url = move |url: String| {
        leptos::task::spawn_local(async move {
            let args = serde_wasm_bindgen::to_value(&OpenUrlArgs { url: url.clone() }).unwrap_or(JsValue::NULL);
            let _ = call_tauri("open_external_url", args).await;
            notify(format!("Abriendo: {}", url));
        });
    };

    let sort_ocr_blocks = |blocks: &[OcrBlock]| -> Vec<OcrBlock> {
        let mut sorted = blocks.to_vec();
        sorted.sort_by(|a, b| {
            let y_diff = a.y - b.y;
            if y_diff.abs() < 14.0 {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
        sorted
    };

    let copy_all_ocr = {
        let ocr_blocks = ocr_blocks.clone();
        let copy_text_direct = copy_text_direct.clone();
        Arc::new(move || {
            let blocks = ocr_blocks.get_untracked();
            if blocks.is_empty() {
                return;
            }
            let sorted = sort_ocr_blocks(&blocks);
            let full_text = sorted
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            copy_text_direct(full_text);
        })
    };

    let copy_selected_ocr = {
        let ocr_blocks = ocr_blocks.clone();
        let selected_ocr_indices = selected_ocr_indices.clone();
        let copy_text_direct = copy_text_direct.clone();
        let set_selected_ocr_indices = set_selected_ocr_indices.clone();
        Arc::new(move || {
            let sel = selected_ocr_indices.get_untracked();
            let blocks = ocr_blocks.get_untracked();
            let selected_blocks: Vec<OcrBlock> = blocks
                .into_iter()
                .enumerate()
                .filter(|(idx, _)| sel.contains(idx))
                .map(|(_, b)| b)
                .collect();
            if !selected_blocks.is_empty() {
                let sorted = sort_ocr_blocks(&selected_blocks);
                let full_text = sorted
                    .iter()
                    .map(|b| b.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                copy_text_direct(full_text);
                set_selected_ocr_indices.set(BTreeSet::new());
            }
        })
    };

    Effect::new({
        let selected_ocr_indices = selected_ocr_indices.clone();
        let preview_scroll_ref = preview_scroll_ref.clone();
        move |_| {
            let _ = selected_ocr_indices.get();
            let _ = ocr_blocks.get();
            if let Some(el) = preview_scroll_ref.get() {
                el.set_scroll_top(el.scroll_height());
            }
        }
    });

    let trigger_ocr_action = {
        let canvas_ref = canvas_ref.clone();
        let history = history.clone();
        let set_selected_ocr_indices = set_selected_ocr_indices.clone();
        Rc::new(move || {
            if is_ocr_active.get_untracked() {
                set_is_ocr_active.set(false);
                set_ocr_blocks.set(Vec::new());
                set_selected_ocr_indices.set(BTreeSet::new());
                return;
            }

            set_is_ocr_loading.set(true);
            set_status_text.set("Escaneando texto (OCR)...".to_string());
            set_selected_ocr_indices.set(BTreeSet::new());

            let has_drawings = !history.borrow().is_empty();
            let canvas_ref = canvas_ref.clone();

            leptos::task::spawn_local(async move {
                let args = if has_drawings {
                    sleep_ms(16).await;
                    let data_url = canvas_ref
                        .get()
                        .and_then(|c| c.to_data_url().ok())
                        .unwrap_or_default();
                    serde_wasm_bindgen::to_value(&OcrArgs {
                        data_base64: data_url,
                    })
                    .unwrap_or(JsValue::NULL)
                } else {
                    JsValue::NULL
                };

                let res = call_tauri("extract_text", args).await;

                match res {
                    Ok(val) => {
                        if let Ok(blocks) = serde_wasm_bindgen::from_value::<Vec<OcrBlock>>(val) {
                            let count = blocks.len();
                            set_ocr_blocks.set(blocks);
                            set_is_ocr_active.set(true);
                            set_selected_ocr_indices.set(BTreeSet::new());
                            notify(format!("{} textos detectados", count));
                        }
                    }
                    Err(e) => {
                        let msg = e.as_string().unwrap_or_else(|| "Error al procesar OCR".to_string());
                        notify(msg);
                    }
                }
                set_is_ocr_loading.set(false);
            });
        })
    };

    let trigger_ocr = {
        let action = trigger_ocr_action.clone();
        move |_| {
            action();
        }
    };

    let on_undo_action = {
        let items = items.clone();
        let history = history.clone();
        let future = future.clone();
        let redraw = redraw_canvas.clone();
        let switch_img = switch_image.clone();
        Rc::new(move || {
            let mut hist = history.borrow_mut();
            if let Some(action) = hist.pop() {
                match action {
                    HistoryAction::Add(item) => {
                        items.borrow_mut().pop();
                        future.borrow_mut().push(HistoryAction::Add(item));
                        redraw();
                    }
                    HistoryAction::Clear(cleared_items) => {
                        *items.borrow_mut() = cleared_items.clone();
                        future.borrow_mut().push(HistoryAction::Clear(cleared_items));
                        redraw();
                    }
                    HistoryAction::Crop {
                        prev_image_data,
                        prev_items,
                        prev_width,
                        prev_height,
                        new_image_data,
                        new_items,
                        new_width,
                        new_height,
                    } => {
                        *items.borrow_mut() = prev_items.clone();
                        switch_img(prev_image_data.clone(), prev_width, prev_height);
                        future.borrow_mut().push(HistoryAction::Crop {
                            prev_image_data,
                            prev_items,
                            prev_width,
                            prev_height,
                            new_image_data,
                            new_items,
                            new_width,
                            new_height,
                        });
                    }
                }
                set_can_undo.set(!hist.is_empty());
                drop(hist);
                set_can_redo.set(!future.borrow().is_empty());
                set_can_clear.set(!items.borrow().is_empty());
            }
        })
    };

    let on_undo = {
        let undo = on_undo_action.clone();
        move |_| {
            undo();
        }
    };

    let on_redo_action = {
        let items = items.clone();
        let history = history.clone();
        let future = future.clone();
        let redraw = redraw_canvas.clone();
        let switch_img = switch_image.clone();
        Rc::new(move || {
            let mut fut = future.borrow_mut();
            if let Some(action) = fut.pop() {
                match action {
                    HistoryAction::Add(item) => {
                        items.borrow_mut().push(item.clone());
                        history.borrow_mut().push(HistoryAction::Add(item));
                        redraw();
                    }
                    HistoryAction::Clear(cleared_items) => {
                        items.borrow_mut().clear();
                        history.borrow_mut().push(HistoryAction::Clear(cleared_items));
                        redraw();
                    }
                    HistoryAction::Crop {
                        prev_image_data,
                        prev_items,
                        prev_width,
                        prev_height,
                        new_image_data,
                        new_items,
                        new_width,
                        new_height,
                    } => {
                        *items.borrow_mut() = new_items.clone();
                        switch_img(new_image_data.clone(), new_width, new_height);
                        history.borrow_mut().push(HistoryAction::Crop {
                            prev_image_data,
                            prev_items,
                            prev_width,
                            prev_height,
                            new_image_data,
                            new_items,
                            new_width,
                            new_height,
                        });
                    }
                }
                set_can_redo.set(!fut.is_empty());
                drop(fut);
                set_can_undo.set(!history.borrow().is_empty());
                set_can_clear.set(!items.borrow().is_empty());
            }
        })
    };

    let on_redo = {
        let redo = on_redo_action.clone();
        move |_| {
            redo();
        }
    };

    let on_clear_action = {
        let items = items.clone();
        let history = history.clone();
        let future = future.clone();
        let redraw = redraw_canvas.clone();
        Rc::new(move || {
            let current = items.borrow().clone();
            if !current.is_empty() {
                items.borrow_mut().clear();
                history.borrow_mut().push(HistoryAction::Clear(current));
                future.borrow_mut().clear();
                redraw();
                set_can_clear.set(false);
                set_can_undo.set(true);
                set_can_redo.set(false);
            }
        })
    };

    let on_clear = {
        let clear = on_clear_action.clone();
        move |_| {
            clear();
        }
    };

    let on_copy_action = {
        let canvas_ref = canvas_ref.clone();
        Rc::new(move || {
            if let Some(canvas) = canvas_ref.get() {
                let data_url = canvas.to_data_url().unwrap_or_default();
                set_status_text.set("Copiando...".to_string());
                leptos::task::spawn_local(async move {
                    let args = serde_wasm_bindgen::to_value(&CopyArgs {
                        data_base64: data_url,
                    })
                    .unwrap_or(JsValue::NULL);
                    match call_tauri("copy_to_clipboard", args).await {
                        Ok(_) => {
                            let gen = copy_success_gen.get_untracked().wrapping_add(1);
                            set_copy_success_gen.set(gen);
                            set_copy_success.set(true);
                            leptos::task::spawn_local(async move {
                                sleep_ms(1800).await;
                                if copy_success_gen.get_untracked() == gen {
                                    set_copy_success.set(false);
                                }
                            });
                            notify("Copiado al portapapeles".to_string());
                        }
                        Err(e) => {
                            let msg = e.as_string().unwrap_or_else(|| "Error al copiar".to_string());
                            notify(msg);
                        }
                    }
                });
            }
        })
    };

    let on_copy = {
        let copy = on_copy_action.clone();
        move |_| {
            copy();
        }
    };

    let on_save_action = {
        let canvas_ref = canvas_ref.clone();
        Rc::new(move || {
            if let Some(canvas) = canvas_ref.get() {
                let data_url = canvas.to_data_url().unwrap_or_default();
                set_status_text.set("Guardando...".to_string());
                leptos::task::spawn_local(async move {
                    let args = serde_wasm_bindgen::to_value(&SaveArgs {
                        data_base64: data_url,
                    })
                    .unwrap_or(JsValue::NULL);
                    match call_tauri("save_image", args).await {
                        Ok(res) => {
                            let gen = save_success_gen.get_untracked().wrapping_add(1);
                            set_save_success_gen.set(gen);
                            set_save_success.set(true);
                            leptos::task::spawn_local(async move {
                                sleep_ms(1800).await;
                                if save_success_gen.get_untracked() == gen {
                                    set_save_success.set(false);
                                }
                            });
                            if let Some(path) = res.as_string() {
                                let label = path.split('/').last().unwrap_or(&path).to_string();
                                notify(format!("Guardado: {}", label));
                            } else {
                                notify("Guardado con éxito".to_string());
                            }
                        }
                        Err(e) => {
                            let msg = e.as_string().unwrap_or_else(|| "Error al guardar".to_string());
                            notify(msg);
                        }
                    }
                });
            }
        })
    };

    let on_save = {
        let save = on_save_action.clone();
        move |_| {
            save();
        }
    };

    let on_close_action = Rc::new(move || {
        leptos::task::spawn_local(async move {
            let _ = call_tauri("close_window", JsValue::NULL).await;
        });
    });

    let on_close = {
        let close = on_close_action.clone();
        move |_| {
            close();
        }
    };

    let _ = window_event_listener(ev::contextmenu, |e: MouseEvent| {
        e.prevent_default();
    });

    let _ = window_event_listener(ev::keydown, {
        let undo = on_undo_action.clone();
        let redo = on_redo_action.clone();
        let copy = on_copy_action.clone();
        let save = on_save_action.clone();
        let close = on_close_action.clone();
        let trigger_ocr = trigger_ocr_action.clone();
        let copy_all_ocr = copy_all_ocr.clone();
        let copy_selected_ocr = copy_selected_ocr.clone();
        let set_selected_ocr_indices = set_selected_ocr_indices.clone();
        move |e: KeyboardEvent| {
            let key = e.key();
            let ctrl = e.ctrl_key() || e.meta_key();
            let shift = e.shift_key();
            let alt = e.alt_key();

            if ctrl && (key == "a" || key == "A") && is_ocr_active.get_untracked() {
                e.prevent_default();
                let len = ocr_blocks.get_untracked().len();
                set_selected_ocr_indices.set((0..len).collect());
            } else if key == "Enter" && is_ocr_active.get_untracked() {
                e.prevent_default();
                if !selected_ocr_indices.get_untracked().is_empty() {
                    copy_selected_ocr();
                } else {
                    copy_all_ocr();
                }
            } else if ctrl && (key == "z" || key == "Z") && !shift {
                e.prevent_default();
                undo();
            } else if (ctrl && (key == "y" || key == "Y")) || (ctrl && shift && (key == "z" || key == "Z")) {
                e.prevent_default();
                redo();
            } else if ctrl && (key == "c" || key == "C") {
                e.prevent_default();
                copy();
            } else if ctrl && (key == "s" || key == "S") {
                e.prevent_default();
                save();
            } else if ctrl && (key == "0") {
                e.prevent_default();
                set_zoom_level.set(1.0);
                set_pan_offset.set((0.0, 0.0));
            } else if key == "Escape" {
                e.prevent_default();
                if show_about.get_untracked() {
                    close_about();
                } else if !selected_ocr_indices.get_untracked().is_empty() {
                    set_selected_ocr_indices.set(BTreeSet::new());
                } else if is_ocr_active.get_untracked() {
                    set_is_ocr_active.set(false);
                    set_ocr_blocks.set(Vec::new());
                    set_selected_ocr_indices.set(BTreeSet::new());
                } else if zoom_level.get_untracked() > 1.05 {
                    set_zoom_level.set(1.0);
                    set_pan_offset.set((0.0, 0.0));
                } else {
                    close();
                }
            } else if !ctrl && !alt {
                match key.as_str() {
                    "p" | "P" => {
                        set_picker_preview.set(None);
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Pen);
                    }
                    "h" | "H" => {
                        set_picker_preview.set(None);
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Highlighter);
                    }
                    "a" | "A" => {
                        set_picker_preview.set(None);
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Arrow);
                    }
                    "r" | "R" => {
                        set_picker_preview.set(None);
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Rectangle);
                    }
                    "c" | "C" => {
                        set_picker_preview.set(None);
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Circle);
                    }
                    "b" | "B" => {
                        set_picker_preview.set(None);
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Blur);
                    }
                    "i" | "I" => {
                        set_is_ocr_active.set(false);
                        set_active_tool.set(Tool::Picker);
                    }
                    "o" | "O" => {
                        trigger_ocr();
                    }
                    "[" => {
                        let cur = stroke_width.get_untracked();
                        if let Some(pos) = WIDTHS.iter().position(|&(w, _)| (w - cur).abs() < 0.1) {
                            if pos > 0 {
                                let new_w = WIDTHS[pos - 1].0;
                                set_stroke_width.set(new_w);
                                notify(format!("Grosor: {}px", new_w));
                            }
                        }
                    }
                    "]" => {
                        let cur = stroke_width.get_untracked();
                        if let Some(pos) = WIDTHS.iter().position(|&(w, _)| (w - cur).abs() < 0.1) {
                            if pos + 1 < WIDTHS.len() {
                                let new_w = WIDTHS[pos + 1].0;
                                set_stroke_width.set(new_w);
                                notify(format!("Grosor: {}px", new_w));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    let on_wheel = {
        let canvas_ref = canvas_ref.clone();
        move |ev: WheelEvent| {
            ev.prevent_default();
            let current_zoom = zoom_level.get_untracked();
            let delta = ev.delta_y();
            let factor = if delta < 0.0 { 1.25 } else { 0.8 };
            let next_zoom = (current_zoom * factor).clamp(1.0, 24.0);

            if (next_zoom - 1.0).abs() < 0.01 {
                set_zoom_level.set(1.0);
                set_pan_offset.set((0.0, 0.0));
            } else {
                let (mut px, mut py) = pan_offset.get_untracked();
                let cx = ev.client_x() as f64;
                let cy = ev.client_y() as f64;

                if (current_zoom - 1.0).abs() < 0.01 {
                    if let Some(canvas) = canvas_ref.get() {
                        let r = canvas.get_bounding_client_rect();
                        px = r.left();
                        py = r.top();
                    }
                }

                let new_px = cx - (cx - px) * (next_zoom / current_zoom);
                let new_py = cy - (cy - py) * (next_zoom / current_zoom);

                set_zoom_level.set(next_zoom);
                set_pan_offset.set((new_px, new_py));
            }
        }
    };

    let on_mouse_down = {
        let is_drawing = is_drawing.clone();
        let is_panning = is_panning.clone();
        let pan_start_mouse = pan_start_mouse.clone();
        let pan_initial = pan_initial.clone();
        let start_point = start_point.clone();
        let current_points = current_points.clone();
        let canvas_ref = canvas_ref.clone();
        let copy_txt = copy_text_direct.clone();
        move |ev: MouseEvent| {
            if ev.button() == 1 {
                *is_panning.borrow_mut() = true;
                set_is_panning_ui.set(true);
                *pan_start_mouse.borrow_mut() = (ev.client_x() as f64, ev.client_y() as f64);
                *pan_initial.borrow_mut() = pan_offset.get_untracked();
                return;
            }

            if ev.button() != 0 {
                return;
            }

            let canvas = match canvas_ref.get() {
                Some(c) => c,
                None => return,
            };
            let rect = canvas.get_bounding_client_rect();
            let scale_x = canvas.width() as f64 / rect.width();
            let scale_y = canvas.height() as f64 / rect.height();
            let x = (ev.client_x() as f64 - rect.left()) * scale_x;
            let y = (ev.client_y() as f64 - rect.top()) * scale_y;

            let tool = active_tool.get_untracked();
            if tool == Tool::Picker {
                if let Some(ctx) = canvas
                    .get_context("2d")
                    .ok()
                    .flatten()
                    .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                {
                    if let Ok(img_data) = ctx.get_image_data(x, y, 1.0, 1.0) {
                        let d = img_data.data();
                        let picked_hex = format!("#{:02x}{:02x}{:02x}", d[0], d[1], d[2]);
                        set_color.set(picked_hex.clone());
                        set_picker_preview.set(None);
                        copy_txt(picked_hex.clone());
                        set_active_tool.set(Tool::Pen);
                    }
                }
                return;
            }

            *is_drawing.borrow_mut() = true;
            *start_point.borrow_mut() = Some(Point { x, y });
            current_points.borrow_mut().clear();
            current_points.borrow_mut().push(Point { x, y });
        }
    };

    let on_mouse_move = {
        let is_drawing = is_drawing.clone();
        let is_panning = is_panning.clone();
        let pan_start_mouse = pan_start_mouse.clone();
        let pan_initial = pan_initial.clone();
        let start_point = start_point.clone();
        let current_points = current_points.clone();
        let canvas_ref = canvas_ref.clone();
        let loupe_canvas_ref = loupe_canvas_ref.clone();
        let redraw = redraw_canvas.clone();
        let base_image = base_image.clone();
        move |ev: MouseEvent| {
            if *is_panning.borrow() {
                let (sx, sy) = *pan_start_mouse.borrow();
                let (ix, iy) = *pan_initial.borrow();
                let dx = ev.client_x() as f64 - sx;
                let dy = ev.client_y() as f64 - sy;
                set_pan_offset.set((ix + dx, iy + dy));
                return;
            }

            let canvas = match canvas_ref.get() {
                Some(c) => c,
                None => return,
            };
            let rect = canvas.get_bounding_client_rect();
            let scale_x = canvas.width() as f64 / rect.width();
            let scale_y = canvas.height() as f64 / rect.height();
            let x = (ev.client_x() as f64 - rect.left()) * scale_x;
            let y = (ev.client_y() as f64 - rect.top()) * scale_y;

            let tool = active_tool.get_untracked();

            if tool == Tool::Picker {
                if let Some(ctx) = canvas
                    .get_context("2d")
                    .ok()
                    .flatten()
                    .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                {
                    if let Ok(img_data) = ctx.get_image_data(x, y, 1.0, 1.0) {
                        let d = img_data.data();
                        let picked_hex = format!("#{:02x}{:02x}{:02x}", d[0], d[1], d[2]);
                        set_picker_preview.set(Some((ev.client_x() as f64, ev.client_y() as f64, picked_hex)));

                        if let Some(l_canvas) = loupe_canvas_ref.get() {
                            if let Some(l_ctx) = l_canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok()) {
                                let patch_size = 9.0;
                                let half = patch_size / 2.0;
                                let src_x = (x - half).max(0.0);
                                let src_y = (y - half).max(0.0);

                                l_ctx.set_image_smoothing_enabled(false);
                                let _ = l_ctx.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                    &canvas,
                                    src_x, src_y, patch_size, patch_size,
                                    0.0, 0.0, 80.0, 80.0,
                                );

                                l_ctx.save();
                                l_ctx.set_stroke_style_str("rgba(255, 255, 255, 0.2)");
                                l_ctx.set_line_width(0.5);
                                let cell = 80.0 / patch_size;
                                for i in 1..9 {
                                    let pos = i as f64 * cell;
                                    l_ctx.begin_path();
                                    l_ctx.move_to(pos, 0.0);
                                    l_ctx.line_to(pos, 80.0);
                                    l_ctx.stroke();
                                    l_ctx.begin_path();
                                    l_ctx.move_to(0.0, pos);
                                    l_ctx.line_to(80.0, pos);
                                    l_ctx.stroke();
                                }

                                let center_start = 4.0 * cell;
                                l_ctx.set_stroke_style_str("rgba(255, 255, 255, 0.95)");
                                l_ctx.set_line_width(1.5);
                                l_ctx.stroke_rect(center_start, center_start, cell, cell);
                                l_ctx.restore();
                            }
                        }
                    }
                }
                return;
            }

            if !*is_drawing.borrow() {
                return;
            }

            let width = stroke_width.get_untracked();
            let clr = color.get_untracked();

            match tool {
                Tool::Pen | Tool::Highlighter => {
                    let prev_pt = current_points.borrow().last().cloned();
                    current_points.borrow_mut().push(Point { x, y });
                    if let Some(prev) = prev_pt {
                        if let Some(ctx) = canvas
                            .get_context("2d")
                            .ok()
                            .flatten()
                            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                        {
                            ctx.save();
                            if tool == Tool::Highlighter {
                                ctx.set_global_alpha(0.35);
                            }
                            ctx.begin_path();
                            ctx.set_stroke_style_str(&clr);
                            ctx.set_line_width(width);
                            ctx.set_line_cap("round");
                            ctx.set_line_join("round");
                            ctx.move_to(prev.x, prev.y);
                            ctx.line_to(x, y);
                            ctx.stroke();
                            ctx.restore();
                        }
                    }
                }
                Tool::Blur => {
                    current_points.borrow_mut().clear();
                    current_points.borrow_mut().push(Point { x, y });
                    redraw();
                    if let Some(ref start) = *start_point.borrow() {
                        if let Some(ctx) = canvas
                            .get_context("2d")
                            .ok()
                            .flatten()
                            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                        {
                            let bx = start.x.min(x);
                            let by = start.y.min(y);
                            let bw = (start.x - x).abs();
                            let bh = (start.y - y).abs();
                            if bw > 1.0 && bh > 1.0 {
                                if let Some(ref img) = *base_image.borrow() {
                                    apply_pixelate(&ctx, img, bx, by, bw, bh);
                                }
                                ctx.save();
                                ctx.set_stroke_style_str("rgba(255, 255, 255, 0.4)");
                                ctx.set_line_width(1.0);
                                ctx.stroke_rect(bx, by, bw, bh);
                                ctx.restore();
                            }
                        }
                    }
                }
                Tool::Arrow | Tool::Rectangle | Tool::Circle => {
                    current_points.borrow_mut().clear();
                    current_points.borrow_mut().push(Point { x, y });
                    redraw();
                    if let Some(ref start) = *start_point.borrow() {
                        if let Some(ctx) = canvas
                            .get_context("2d")
                            .ok()
                            .flatten()
                            .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                        {
                            ctx.save();
                            ctx.set_stroke_style_str(&clr);
                            ctx.set_fill_style_str(&clr);
                            ctx.set_line_width(width);
                            ctx.set_line_cap("round");
                            ctx.set_line_join("round");

                            if tool == Tool::Arrow {
                                ctx.begin_path();
                                ctx.move_to(start.x, start.y);
                                ctx.line_to(x, y);
                                ctx.stroke();
                                draw_arrow_head(&ctx, start, &Point { x, y }, width);
                            } else if tool == Tool::Rectangle {
                                let rx = start.x.min(x);
                                let ry = start.y.min(y);
                                let rw = (start.x - x).abs();
                                let rh = (start.y - y).abs();
                                ctx.stroke_rect(rx, ry, rw, rh);
                            } else {
                                let cx = (start.x + x) / 2.0;
                                let cy = (start.y + y) / 2.0;
                                let rx = ((start.x - x).abs() / 2.0).max(0.1);
                                let ry = ((start.y - y).abs() / 2.0).max(0.1);
                                ctx.begin_path();
                                let _ = ctx.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::TAU);
                                ctx.stroke();
                            }
                            ctx.restore();
                        }
                    }
                }
                Tool::Picker => {}
            }
        }
    };

    let on_mouse_up_action = {
        let is_drawing = is_drawing.clone();
        let is_panning = is_panning.clone();
        let start_point = start_point.clone();
        let current_points = current_points.clone();
        let items = items.clone();
        let history = history.clone();
        let future = future.clone();
        let redraw = redraw_canvas.clone();
        Rc::new(move || {
            *is_panning.borrow_mut() = false;
            set_is_panning_ui.set(false);

            if !*is_drawing.borrow() {
                return;
            }
            *is_drawing.borrow_mut() = false;
            let tool = active_tool.get_untracked();
            let width = stroke_width.get_untracked();
            let clr = color.get_untracked();

            let new_item = match tool {
                Tool::Pen | Tool::Highlighter => {
                    let pts = current_points.borrow().clone();
                    if !pts.is_empty() {
                        Some(DrawingItem::Freehand {
                            points: pts,
                            color: clr,
                            width,
                            is_highlighter: tool == Tool::Highlighter,
                        })
                    } else {
                        None
                    }
                }
                Tool::Blur => {
                    if let (Some(start), Some(end)) = (start_point.borrow().clone(), current_points.borrow().last().cloned()) {
                        if (start.x - end.x).abs() > 2.0 || (start.y - end.y).abs() > 2.0 {
                            Some(DrawingItem::Blur { start, end })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Tool::Arrow => {
                    if let (Some(start), Some(end)) = (start_point.borrow().clone(), current_points.borrow().last().cloned()) {
                        if (start.x - end.x).hypot(start.y - end.y) > 2.0 {
                            Some(DrawingItem::Arrow {
                                start,
                                end,
                                color: clr,
                                width,
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Tool::Rectangle => {
                    if let (Some(start), Some(end)) = (start_point.borrow().clone(), current_points.borrow().last().cloned()) {
                        if (start.x - end.x).abs() > 2.0 || (start.y - end.y).abs() > 2.0 {
                            Some(DrawingItem::Rectangle {
                                start,
                                end,
                                color: clr,
                                width,
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Tool::Circle => {
                    if let (Some(start), Some(end)) = (start_point.borrow().clone(), current_points.borrow().last().cloned()) {
                        if (start.x - end.x).abs() > 2.0 || (start.y - end.y).abs() > 2.0 {
                            Some(DrawingItem::Circle {
                                start,
                                end,
                                color: clr,
                                width,
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Tool::Picker => None,
            };

            if let Some(item) = new_item {
                items.borrow_mut().push(item.clone());
                history.borrow_mut().push(HistoryAction::Add(item));
                future.borrow_mut().clear();
                redraw();
                set_can_undo.set(true);
                set_can_redo.set(false);
                set_can_clear.set(true);
            }
        })
    };

    let on_mouse_up = {
        let action = on_mouse_up_action.clone();
        move |_ev: MouseEvent| {
            action();
        }
    };

    let on_mouse_leave = {
        let action = on_mouse_up_action.clone();
        move |_ev: MouseEvent| {
            set_picker_preview.set(None);
            action();
        }
    };

    let start_crop_drag = {
        let crop_handle_state = crop_handle_state.clone();
        let crop_start_mouse = crop_start_mouse.clone();
        let current_crop_rect = current_crop_rect.clone();
        let canvas_ref = canvas_ref.clone();
        move |handle: CropHandle, ev: MouseEvent| {
            ev.stop_propagation();
            ev.prevent_default();
            if let Some(canvas) = canvas_ref.get() {
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;
                *crop_handle_state.borrow_mut() = Some(handle);
                *crop_start_mouse.borrow_mut() = Point {
                    x: ev.client_x() as f64,
                    y: ev.client_y() as f64,
                };
                *current_crop_rect.borrow_mut() = Rect { x: 0.0, y: 0.0, w, h };
                set_is_cropping_active.set(true);
            }
        }
    };

    let _ = window_event_listener(ev::mousemove, {
        let crop_handle_state = crop_handle_state.clone();
        let crop_start_mouse = crop_start_mouse.clone();
        let current_crop_rect = current_crop_rect.clone();
        let canvas_ref = canvas_ref.clone();
        let redraw = redraw_canvas.clone();
        move |ev: MouseEvent| {
            let handle_opt = *crop_handle_state.borrow();
            if let Some(handle) = handle_opt {
                if let Some(canvas) = canvas_ref.get() {
                    let total_w = canvas.width() as f64;
                    let total_h = canvas.height() as f64;
                    let rect = canvas.get_bounding_client_rect();
                    let scale_x = total_w / rect.width();
                    let scale_y = total_h / rect.height();

                    let dx = (ev.client_x() as f64 - crop_start_mouse.borrow().x) * scale_x;
                    let dy = (ev.client_y() as f64 - crop_start_mouse.borrow().y) * scale_y;

                    let mut min_x = 0.0f64;
                    let mut min_y = 0.0f64;
                    let mut max_x = total_w;
                    let mut max_y = total_h;

                    match handle {
                        CropHandle::TopLeft => {
                            min_x = dx.max(0.0).min(total_w - 20.0);
                            min_y = dy.max(0.0).min(total_h - 20.0);
                        }
                        CropHandle::Top => {
                            min_y = dy.max(0.0).min(total_h - 20.0);
                        }
                        CropHandle::TopRight => {
                            max_x = (total_w + dx).max(20.0).min(total_w);
                            min_y = dy.max(0.0).min(total_h - 20.0);
                        }
                        CropHandle::Right => {
                            max_x = (total_w + dx).max(20.0).min(total_w);
                        }
                        CropHandle::BottomRight => {
                            max_x = (total_w + dx).max(20.0).min(total_w);
                            max_y = (total_h + dy).max(20.0).min(total_h);
                        }
                        CropHandle::Bottom => {
                            max_y = (total_h + dy).max(20.0).min(total_h);
                        }
                        CropHandle::BottomLeft => {
                            min_x = dx.max(0.0).min(total_w - 20.0);
                            max_y = (total_h + dy).max(20.0).min(total_h);
                        }
                        CropHandle::Left => {
                            min_x = dx.max(0.0).min(total_w - 20.0);
                        }
                    }

                    let crop_w = (max_x - min_x).max(20.0);
                    let crop_h = (max_y - min_y).max(20.0);
                    *current_crop_rect.borrow_mut() = Rect { x: min_x, y: min_y, w: crop_w, h: crop_h };

                    set_dimension_text.set(calculate_aspect_ratio_str(crop_w, crop_h));

                    redraw();

                    if let Some(ctx) = canvas
                        .get_context("2d")
                        .ok()
                        .flatten()
                        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
                    {
                        ctx.save();
                        ctx.set_fill_style_str("rgba(0, 0, 0, 0.65)");
                        ctx.fill_rect(0.0, 0.0, total_w, min_y);
                        ctx.fill_rect(0.0, min_y + crop_h, total_w, total_h - (min_y + crop_h));
                        ctx.fill_rect(0.0, min_y, min_x, crop_h);
                        ctx.fill_rect(min_x + crop_w, min_y, total_w - (min_x + crop_w), crop_h);

                        ctx.set_stroke_style_str("rgba(255, 255, 255, 0.9)");
                        ctx.set_line_width(1.5);
                        ctx.stroke_rect(min_x, min_y, crop_w, crop_h);
                        ctx.restore();
                    }
                }
            }
        }
    });

    let _ = window_event_listener(ev::mouseup, {
        let crop_handle_state = crop_handle_state.clone();
        let current_crop_rect = current_crop_rect.clone();
        let canvas_ref = canvas_ref.clone();
        let base_image = base_image.clone();
        let items = items.clone();
        let history = history.clone();
        let future = future.clone();
        let switch_img = switch_image.clone();
        let redraw = redraw_canvas.clone();
        move |_| {
            if crop_handle_state.borrow().is_some() {
                *crop_handle_state.borrow_mut() = None;
                set_is_cropping_active.set(false);

                let rect = current_crop_rect.borrow().clone();
                if let Some(canvas) = canvas_ref.get() {
                    let total_w = canvas.width() as f64;
                    let total_h = canvas.height() as f64;

                    if rect.w > 10.0 && rect.h > 10.0 && (rect.w < total_w - 2.0 || rect.h < total_h - 2.0 || rect.x > 2.0 || rect.y > 2.0) {
                        let doc = match web_sys::window().and_then(|win| win.document()) {
                            Some(d) => d,
                            None => return,
                        };
                        let off_canvas: HtmlCanvasElement = match doc.create_element("canvas").ok().and_then(|el| el.dyn_into().ok()) {
                            Some(c) => c,
                            None => return,
                        };
                        off_canvas.set_width(rect.w as u32);
                        off_canvas.set_height(rect.h as u32);

                        let off_ctx = match off_canvas.get_context("2d").ok().flatten().and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok()) {
                            Some(c) => c,
                            None => return,
                        };

                        let _ = off_ctx.draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &canvas,
                            rect.x, rect.y, rect.w, rect.h,
                            0.0, 0.0, rect.w, rect.h,
                        );

                        let prev_img_data = base_image.borrow().as_ref().map(|i| i.src()).unwrap_or_default();
                        let prev_items_list = items.borrow().clone();
                        let prev_w = canvas.width();
                        let prev_h = canvas.height();

                        let new_img_data = off_canvas.to_data_url().unwrap_or_default();
                        let new_w = rect.w as u32;
                        let new_h = rect.h as u32;

                        items.borrow_mut().clear();

                        history.borrow_mut().push(HistoryAction::Crop {
                            prev_image_data: prev_img_data,
                            prev_items: prev_items_list,
                            prev_width: prev_w,
                            prev_height: prev_h,
                            new_image_data: new_img_data.clone(),
                            new_items: Vec::new(),
                            new_width: new_w,
                            new_height: new_h,
                        });
                        future.borrow_mut().clear();

                        switch_img(new_img_data, new_w, new_h);
                        notify(format!("Recortado a {}x{}", new_w, new_h));
                        set_can_undo.set(true);
                        set_can_redo.set(false);
                        set_can_clear.set(false);
                    } else {
                        redraw();
                    }
                }
            }
        }
    });

    let reset_zoom = move |_| {
        set_zoom_level.set(1.0);
        set_pan_offset.set((0.0, 0.0));
    };

    view! {
        <div
            class="flex h-screen w-screen bg-zinc-950 text-zinc-100 select-none overflow-hidden font-sans relative"
            on:contextmenu=move |e| e.prevent_default()
        >
            <div class="absolute top-2.5 right-2.5 flex items-center gap-1.5 z-40">
                {move || {
                    if !is_tiling.get() {
                        view! {
                            <button
                                class=move || format!(
                                    "w-8 h-8 rounded-lg bg-zinc-900 border border-white/10 transition-all duration-150 flex items-center justify-center cursor-pointer active:scale-95 shadow-xl {}",
                                    if is_pinned.get() { "text-zinc-100 bg-white/15 border-white/25" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                )
                                aria-label=move || if is_pinned.get() { "Desfijar ventana" } else { "Fijar ventana siempre visible" }
                                on:click=toggle_pin
                            >
                                {move || if is_pinned.get() {
                                    view! {
                                        <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M7.5 8C6.95863 8.1281 6.49932 8.14239 5.99268 8.45891C5.07234 9.03388 4.85108 9.71674 5.08821 10.7612C5.94028 14.5139 9.48599 18.0596 13.2388 18.9117C14.2834 19.1489 14.9661 18.928 15.5416 18.0077C15.8411 17.5288 15.8716 17.0081 16 16.5" />
                                            <path d="M12 7.79915C12.1776 7.77794 12.3182 7.74034 12.4295 7.68235C13.3997 7.17686 13.9291 5.53361 14.4498 4.60009C14.9311 3.73715 15.1718 3.30567 15.7379 3.10227C16.3041 2.89888 16.6448 3.02205 17.3262 3.26839C18.9197 3.8445 20.1555 5.08032 20.7316 6.6738C20.978 7.35521 21.1009 7.69591 20.8977 8.26204C20.6943 8.82817 20.2628 9.06884 19.3999 9.55018C18.4608 10.074 16.7954 10.6108 16.2905 11.5898C16.2345 11.6983 16.1978 11.8327 16.1769 12" />
                                            <path d="M3 21L8 16" />
                                            <path d="M3 3L21 21" />
                                        </svg>
                                    }.into_any()
                                } else {
                                    view! {
                                        <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M3 21L8 16" />
                                            <path d="M13.2585 18.8714C9.51516 18.0215 5.97844 14.4848 5.12853 10.7415C4.99399 10.1489 4.92672 9.85266 5.12161 9.37197C5.3165 8.89129 5.55457 8.74255 6.03071 8.44509C7.10705 7.77265 8.27254 7.55888 9.48209 7.66586C11.1793 7.81598 12.0279 7.89104 12.4512 7.67048C12.8746 7.44991 13.1622 6.93417 13.7376 5.90269L14.4664 4.59604C14.9465 3.73528 15.1866 3.3049 15.7513 3.10202C16.316 2.89913 16.6558 3.02199 17.3355 3.26771C18.9249 3.84236 20.1576 5.07505 20.7323 6.66449C20.978 7.34417 21.1009 7.68401 20.898 8.2487C20.6951 8.8134 20.2647 9.05346 19.4039 9.53358L18.0672 10.2792C17.0376 10.8534 16.5229 11.1406 16.3024 11.568C16.0819 11.9955 16.162 12.8256 16.3221 14.4859C16.4399 15.7068 16.2369 16.88 15.5555 17.9697C15.2577 18.4458 15.1088 18.6839 14.6283 18.8786C14.1477 19.0733 13.8513 19.006 13.2585 18.8714Z" />
                                        </svg>
                                    }.into_any()
                                }}
                            </button>
                        }.into_any()
                    } else {
                        view! { <span class="hidden" /> }.into_any()
                    }
                }}

                <button
                    class="w-8 h-8 rounded-lg bg-zinc-900 border border-white/10 text-zinc-400 hover:text-red-400 hover:bg-white/5 transition-all duration-150 flex items-center justify-center cursor-pointer active:scale-95 shadow-xl"
                    aria-label="Cerrar"
                    on:click=on_close
                >
                    <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M18 6L12 12M12 12L6 18M12 12L18 18M12 12L6 6" />
                    </svg>
                </button>
            </div>

            <aside
                class="absolute left-2.5 top-2.5 bottom-2.5 w-13 bg-zinc-900 border border-white/10 rounded-lg flex flex-col items-center py-2.5 justify-between z-40 shadow-2xl overflow-y-auto no-scrollbar"
                on:click=move |e| e.stop_propagation()
            >
                <div class="flex flex-col items-center gap-1.5 w-full">
                    {move || {
                        if !is_tiling.get() {
                            view! {
                                <div
                                    class="w-8.5 h-8.5 rounded-lg text-zinc-400 hover:text-zinc-200 flex items-center justify-center cursor-grab active:cursor-grabbing hover:bg-white/5 transition-all duration-150"
                                    data-tauri-drag-region
                                >
                                    <svg class="w-[18px] h-[18px] pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M6 5C6.55228 5 7 5.44772 7 6C7 6.55228 6.55228 7 6 7C5.44772 7 5 6.55228 5 6C5 5.44772 5 6 5Z" />
                                        <path d="M6 11C6.55228 11 7 11.4477 7 12C7 12.5523 6.55228 13 6 13C5.44772 13 5 12.5523 5 12C5 11.4477 5 11 6 11Z" />
                                        <path d="M6 17C6.55228 17 7 17.4477 7 18C7 18.5523 6.55228 19 6 19C5.44772 19 5 18.5523 5 18C5 17.4477 5 17 6 17Z" />
                                        <path d="M18 5C18.5523 5 19 5.44772 19 6C19 6.55228 18.5523 7 18 7C17.4477 7 17 6.55228 17 6C17 5.44772 17.4477 5 18 5Z" />
                                        <path d="M18 11C18.5523 11 19 11.4477 19 12C19 12.5523 18.5523 13 18 13C17.4477 13 17 12.5523 17 12C17 11.4477 17.4477 11 18 11Z" />
                                        <path d="M12 11C12.5523 11 13 11.4477 13 12C13 12.5523 12.5523 13 12 13C11.4477 13 11 12.5523 11 12C11 11.4477 11.4477 11 12 11Z" />
                                        <path d="M12 5C12.5523 5 13 5.44772 13 6C13 6.55228 12.5523 7 12 7C11.4477 7 11 6.55228 11 6C11 5.44772 11.4477 5 12 5Z" />
                                        <path d="M18 17C18.5523 17 19 17.4477 19 18C19 18.5523 18.5523 19 18 19C17.4477 19 17 18.5523 17 18C17 17.4477 17.4477 17 18 17Z" />
                                        <path d="M12 17C12.5523 17 13 17.4477 13 18C13 18.5523 12.5523 19 12 19C11.4477 19 11 18.5523 11 18C11 17.4477 11.4477 17 12 17Z" />
                                    </svg>
                                </div>
                                <div class="w-6 h-px bg-white/10 my-0.5"></div>
                            }.into_any()
                        } else {
                            view! { <span class="hidden" /> }.into_any()
                        }
                    }}

                    <div class="accordion-group">
                        <div class="relative tooltip-trigger w-full flex items-center justify-center">
                            <button
                                class=move || {
                                    let t = active_tool.get();
                                    let is_act = !is_ocr_active.get() && t == Tool::Pen;
                                    format!(
                                        "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                        if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                    )
                                }
                                aria-label="Lápiz (P)"
                                on:click=move |_| {
                                    set_picker_preview.set(None);
                                    set_is_ocr_active.set(false);
                                    set_active_tool.set(Tool::Pen);
                                }
                            >
                                <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M3.49977 18.9853V20.5H5.01449C6.24074 20.5 6.85387 20.5 7.40518 20.2716C7.9565 20.0433 8.39004 19.6097 9.25713 18.7426L19.1211 8.87868C20.0037 7.99612 20.4449 7.55483 20.4937 7.01325C20.5018 6.92372 20.5018 6.83364 20.4937 6.74411C20.4449 6.20253 20.0037 5.76124 19.1211 4.87868C18.2385 3.99612 17.7972 3.55483 17.2557 3.50605C17.1661 3.49798 17.0761 3.49798 16.9865 3.50605C16.4449 3.55483 16.0037 3.99612 15.1211 4.87868L5.25713 14.7426C4.39004 15.6097 3.9565 16.0433 3.72813 16.5946C3.49977 17.1459 3.49977 17.759 3.49977 18.9853Z" />
                                    <path d="M13.5 6.5L17.5 10.5" />
                                </svg>
                            </button>
                            <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                                "Lápiz · P"
                            </div>
                        </div>

                        <div class="accordion-drawer">
                            <div class="accordion-drawer-inner">
                                <div class="relative tooltip-trigger w-full flex items-center justify-center">
                                    <button
                                        class=move || {
                                            let t = active_tool.get();
                                            let is_act = !is_ocr_active.get() && t == Tool::Highlighter;
                                            format!(
                                                "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                                if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                            )
                                        }
                                        aria-label="Resaltar (H)"
                                        on:click=move |_| {
                                            set_picker_preview.set(None);
                                            set_is_ocr_active.set(false);
                                            set_active_tool.set(Tool::Highlighter);
                                        }
                                    >
                                        <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M6.6777 16.2071L8.79289 18.3223M6.6777 16.2071L2.5 20.5H6.5L8.79289 18.3223M6.6777 16.2071C6.28717 15.8166 6.29534 15.1872 6.63537 14.752C7.42742 13.7383 7.71531 12.8216 7.79924 12.1382C7.89158 11.3863 8.07366 10.5734 8.60933 10.0377L9.50122 9.14828M8.79289 18.3223C9.18342 18.7128 9.81278 18.7047 10.248 18.3646C11.2617 17.5726 12.1784 17.2847 12.8618 17.2008C13.6137 17.1084 14.4266 16.9263 14.9623 16.3907L15.8517 15.4988M15.8517 15.4988L9.50122 9.14828M15.8517 15.4988C16.2422 15.8893 16.8754 15.8893 17.2659 15.4988L21.5 11.2647M9.50122 9.14828C9.1107 8.75776 9.1107 8.12459 9.50122 7.73407L13.7353 3.5" />
                                        </svg>
                                    </button>
                                    <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                                        "Resaltar · H"
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="accordion-group">
                        <div class="relative tooltip-trigger w-full flex items-center justify-center">
                            <button
                                class=move || {
                                    let t = active_tool.get();
                                    let is_act = !is_ocr_active.get() && t == Tool::Rectangle;
                                    format!(
                                        "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                        if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                    )
                                }
                                aria-label="Rectángulo (R)"
                                on:click=move |_| {
                                    set_picker_preview.set(None);
                                    set_is_ocr_active.set(false);
                                    set_active_tool.set(Tool::Rectangle);
                                }
                            >
                                <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124Z" />
                                </svg>
                            </button>
                            <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                                "Rectángulo · R"
                            </div>
                        </div>

                        <div class="accordion-drawer">
                            <div class="accordion-drawer-inner">
                                <div class="relative tooltip-trigger w-full flex items-center justify-center">
                                    <button
                                        class=move || {
                                            let t = active_tool.get();
                                            let is_act = !is_ocr_active.get() && t == Tool::Arrow;
                                            format!(
                                                "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                                if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                            )
                                        }
                                        aria-label="Flecha (A)"
                                        on:click=move |_| {
                                            set_picker_preview.set(None);
                                            set_is_ocr_active.set(false);
                                            set_active_tool.set(Tool::Arrow);
                                        }
                                    >
                                        <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M14 12L4 12" />
                                            <path d="M18.5859 13.6026L17.6194 14.3639C16.0536 15.5974 15.2707 16.2141 14.6354 15.9328C14 15.6515 14 14.6881 14 12.7613L14 11.2387C14 9.31191 14 8.34853 14.6354 8.06721C15.2707 7.7859 16.0536 8.40264 17.6194 9.63612L18.5858 10.3974C19.5286 11.1401 20 11.5115 20 12C20 12.4885 19.5286 12.8599 18.5859 13.6026Z" />
                                        </svg>
                                    </button>
                                    <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                                        "Flecha · A"
                                    </div>
                                </div>

                                <div class="relative tooltip-trigger w-full flex items-center justify-center">
                                    <button
                                        class=move || {
                                            let t = active_tool.get();
                                            let is_act = !is_ocr_active.get() && t == Tool::Circle;
                                            format!(
                                                "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                                if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                            )
                                        }
                                        aria-label="Círculo (C)"
                                        on:click=move |_| {
                                            set_picker_preview.set(None);
                                            set_is_ocr_active.set(false);
                                            set_active_tool.set(Tool::Circle);
                                        }
                                    >
                                        <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round">
                                            <circle cx="12" cy="12" r="10" />
                                        </svg>
                                    </button>
                                    <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                                        "Círculo · C"
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>

                    <div class="w-6 h-px bg-white/10 my-0.5"></div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || {
                                let t = active_tool.get();
                                let is_act = !is_ocr_active.get() && t == Tool::Blur;
                                format!(
                                    "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                    if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                )
                            }
                            aria-label="Censurar (B)"
                            on:click=move |_| {
                                set_picker_preview.set(None);
                                set_is_ocr_active.set(false);
                                set_active_tool.set(Tool::Blur);
                            }
                        >
                            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                                <path d="M6.43385 6.51953C4.22009 7.89049 2.93281 9.86457 2.31858 11.0339C2.10621 11.4382 2.00003 11.6403 2 12.0082C1.99997 12.3761 2.10584 12.5777 2.3176 12.981C3.32862 14.9066 6.16702 19.0195 11.9669 19.0195C14.2454 19.0195 16.0669 18.3848 17.5 17.4972" stroke-linejoin="round" />
                                <path d="M9.87868 9.87868C9.33579 10.4216 9 11.1716 9 12C9 13.6569 10.3431 15 12 15C12.8284 15 13.5784 14.6642 14.1213 14.1213" />
                                <path d="M2 2L22 22" stroke-linejoin="round" />
                                <path d="M10 5.14847C10.5934 5.05255 11.224 5 11.8936 5C17.7747 5 20.6528 9.05385 21.6779 10.9517C21.8927 11.3492 22 11.548 22 11.9106C22 12.2733 21.8921 12.4727 21.6765 12.8717C21.3678 13.4428 20.8916 14.2085 20.2167 15" stroke-linejoin="round" />
                            </svg>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Censurar · B"
                        </div>
                    </div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || {
                                let t = active_tool.get();
                                let is_act = !is_ocr_active.get() && t == Tool::Picker;
                                format!(
                                    "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 {}",
                                    if is_act { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                )
                            }
                            aria-label="Gotero (I)"
                            on:click=move |_| {
                                set_is_ocr_active.set(false);
                                set_active_tool.set(Tool::Picker);
                            }
                        >
                            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M13.435 7L7.15915 13.2759M7.15915 13.2759L4.82728 15.6077C3.92569 16.5093 3.47489 16.9601 3.23745 17.5334C3 18.1066 3 18.7441 3 20.0192V21H3.98082C5.25586 21 5.89338 21 6.46663 20.7626C7.03988 20.5251 7.49068 20.0743 8.39227 19.1727L14.2891 13.2759M7.15915 13.2759H14.2891M14.2891 13.2759L17 10.565" />
                                <path d="M19.2087 8.38869L20.82 10M19.2087 8.38869L20.0705 7.52682C20.363 7.23431 20.5093 7.08805 20.611 6.94529C21.1297 6.21676 21.1297 5.23953 20.611 4.511C20.5093 4.36824 20.363 4.22198 20.0705 3.92947C19.778 3.63697 19.6318 3.4907 19.489 3.38905C18.7605 2.87032 17.7832 2.87032 17.0547 3.38905C16.912 3.4907 16.7657 3.63695 16.4732 3.92947L15.6113 4.79133M19.2087 8.38869L15.6113 4.79133M14 3.18002L15.6113 4.79133" />
                            </svg>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Gotero · I"
                        </div>
                    </div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || {
                                format!(
                                    "w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 relative {}",
                                    if is_ocr_active.get() { "bg-white/15 text-zinc-100" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                                )
                            }
                            aria-label="Texto OCR (O)"
                            on:click=trigger_ocr
                        >
                            <div class="relative flex items-center justify-center">
                                <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M11 20C7.25027 20 5.3754 20 4.06107 19.0451C3.6366 18.7367 3.26331 18.3634 2.95491 17.9389C2 16.6246 2 14.7497 2 11C2 7.25027 2 5.3754 2.95491 4.06107C3.26331 3.6366 3.26331 3.26331 4.06107 2.95491C5.3754 2 7.25027 2 11 2H11.5C14.7734 2 16.4101 2 17.6125 2.7368C18.2853 3.14908 18.8509 3.71473 19.2632 4.38751C20 5.58985 20 7.22657 20 10.5" />
                                    <path d="M17.4069 14.4036C17.6192 13.8655 18.3808 13.8655 18.5931 14.4036L18.6298 14.4969C19.1482 15.8113 20.1887 16.8518 21.5031 17.3702L21.5964 17.4069C22.1345 17.6192 22.1345 18.3808 21.5964 18.5931L21.5031 18.6298C20.1887 19.1482 19.1482 20.1887 18.6298 21.5031L18.5931 21.5964C18.3808 22.1345 17.6192 22.1345 17.4069 21.5964L17.3702 21.5031C16.8518 20.1887 15.8113 19.1482 14.4969 18.6298L14.4036 18.5931C13.8655 18.3808 13.8655 17.6192 14.4036 17.4069L14.4969 17.3702C15.8113 16.8518 16.8518 15.8113 17.3702 14.4969L17.4069 14.4036Z" />
                                    <path d="M11 7H7V8M11 7H15V8M11 7V15M11 15H10M11 15H12" />
                                </svg>
                                {move || if is_ocr_loading.get() {
                                    view! {
                                        <span class="absolute inset-0 rounded-lg border-2 border-white/20 border-t-white animate-spin pointer-events-none"></span>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }}
                            </div>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Texto OCR · O"
                        </div>
                    </div>

                    <div class="w-6 h-px bg-white/10 my-0.5"></div>

                    <div class="accordion-group-colors">
                        <div class="relative tooltip-trigger w-full flex items-center justify-center">
                            <button
                                class="w-8.5 h-8.5 rounded-lg flex items-center justify-center transition-all duration-150 cursor-pointer active:scale-95 hover:bg-white/5"
                                aria-label="Color y grosor"
                            >
                                <span
                                    class="w-4 h-4 rounded-full border border-white/30 shadow-xs flex items-center justify-center"
                                    style=move || format!("background-color: {}", color.get())
                                >
                                    <span
                                        class="rounded-full bg-white/90"
                                        style=move || format!("width: {}px; height: {}px;", (stroke_width.get() / 3.5).clamp(2.0, 7.0), (stroke_width.get() / 3.5).clamp(2.0, 7.0))
                                    />
                                </span>
                            </button>
                            <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                                {move || format!("Color y grosor ({}px)", stroke_width.get())}
                            </div>
                        </div>

                        <div class="accordion-drawer">
                            <div class="accordion-drawer-inner px-1.5 py-1.5 bg-white/[0.04] rounded-lg border border-white/5 my-0.5 w-full">
                                <div class="grid grid-cols-2 gap-1.5 w-full justify-items-center">
                                    {COLOR_LIST.iter().map(|(c_hex, _)| {
                                        let hex = c_hex.to_string();
                                        let hex_clone = hex.clone();
                                        let hex_aria = hex.clone();
                                        let is_active = move || color.get() == hex;
                                        view! {
                                            <button
                                                class=move || {
                                                    format!(
                                                        "w-3.5 h-3.5 rounded-full transition-all duration-150 cursor-pointer border border-white/15 flex items-center justify-center {}",
                                                        if is_active() { "ring-2 ring-white scale-110" } else { "hover:scale-110 opacity-80 hover:opacity-100" }
                                                    )
                                                }
                                                style=format!("background-color: {}", c_hex)
                                                aria-label=format!("Color {}", hex_aria)
                                                on:click=move |_| { set_color.set(hex_clone.clone()); }
                                            />
                                        }
                                    }).collect_view()}
                                </div>

                                <div class="w-6 h-px bg-white/10 my-1"></div>

                                <div class="flex flex-col gap-1 w-full items-center">
                                    {WIDTHS.iter().map(|(w_val, line_class)| {
                                        let width_val = *w_val;
                                        let is_active = move || stroke_width.get() == width_val;
                                        view! {
                                            <button
                                                class=move || format!(
                                                    "w-full flex items-center justify-center py-1 px-1.5 rounded transition-all duration-150 cursor-pointer {}",
                                                    if is_active() { "bg-white/20 text-white" } else { "text-zinc-500 hover:text-zinc-200 hover:bg-white/5" }
                                                )
                                                aria-label=format!("Grosor {}px", width_val)
                                                on:click=move |_| { set_stroke_width.set(width_val); }
                                            >
                                                <span class=format!("w-full bg-current rounded-full {}", line_class)></span>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <div class="flex flex-col items-center gap-1.5 w-full">
                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || format!(
                                "w-8.5 h-8.5 rounded-lg transition-all duration-150 flex items-center justify-center active:scale-95 {}",
                                if can_undo.get() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5 cursor-pointer" } else { "text-zinc-600 cursor-not-allowed" }
                            )
                            aria-label="Deshacer (Ctrl+Z)"
                            disabled=move || !can_undo.get()
                            on:click=on_undo
                        >
                            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M3 8H15C18.3137 8 21 10.6863 21 14C21 17.3137 18.3137 20 15 20H11" />
                                <path d="M7 4L5.8462 4.87652C3.94873 6.31801 3 7.03875 3 8C3 8.96125 3.94873 9.68199 5.8462 11.1235L7 12" />
                            </svg>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Deshacer · Ctrl+Z"
                        </div>
                    </div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || format!(
                                "w-8.5 h-8.5 rounded-lg transition-all duration-150 flex items-center justify-center active:scale-95 {}",
                                if can_redo.get() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5 cursor-pointer" } else { "text-zinc-600 cursor-not-allowed" }
                            )
                            aria-label="Rehacer (Ctrl+Y)"
                            disabled=move || !can_redo.get()
                            on:click=on_redo
                        >
                            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M21 8H9C5.68629 8 3 10.6863 3 14C3 17.3137 5.68629 20 9 20H13" />
                                <path d="M17 4L18.1538 4.87652C20.0513 6.31801 21 7.03875 21 8C21 8.96125 20.0513 9.68199 18.1538 11.1235L17 12" />
                            </svg>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Rehacer · Ctrl+Y"
                        </div>
                    </div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || format!(
                                "w-8.5 h-8.5 rounded-lg transition-all duration-150 flex items-center justify-center active:scale-95 {}",
                                if can_clear.get() { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5 cursor-pointer" } else { "text-zinc-600 cursor-not-allowed" }
                            )
                            aria-label="Limpiar lienzo"
                            disabled=move || !can_clear.get()
                            on:click=on_clear
                        >
                            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                                <path d="M19.5 5.5L18.8803 15.5251C18.7219 18.0864 18.6428 19.3671 18.0008 20.2879C17.6833 20.7431 17.2747 21.1273 16.8007 21.416C15.8421 22 14.559 22 11.9927 22C9.42312 22 8.1383 22 7.17905 21.4149C6.7048 21.1257 6.296 20.7408 5.97868 20.2848C5.33688 19.3626 5.25945 18.0801 5.10461 15.5152L4.5 5.5" />
                                <path d="M3 5.5H21M16.0557 5.5L15.3731 4.09173C14.9196 3.15626 14.6928 2.68852 14.3017 2.39681C14.215 2.3321 14.1231 2.27454 14.027 2.2247C13.5939 2 13.0741 2 12.0345 2C10.9688 2 10.436 2 9.99568 2.23412C9.8981 2.28601 9.80498 2.3459 9.71729 2.41317C9.32164 2.7167 9.10063 3.20155 8.65861 4.17126L8.05292 5.5" />
                                <path d="M9.5 16.5L9.5 10.5" />
                                <path d="M14.5 16.5L14.5 10.5" />
                            </svg>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Limpiar lienzo"
                        </div>
                    </div>

                    <div class="w-6 h-px bg-white/10 my-0.5"></div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || format!(
                                "w-8.5 h-8.5 rounded-lg transition-all duration-150 flex items-center justify-center cursor-pointer active:scale-95 {}",
                                if copy_success.get() { "text-zinc-100 bg-white/15" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                            )
                            aria-label="Copiar imagen (Ctrl+C)"
                            on:click=on_copy
                        >
                            {move || if copy_success.get() {
                                view! {
                                    <svg class="w-[18px] h-[18px] animate-icon-success" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                        <path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" />
                                        <path d="M8 12.5L10.5 15L16 9" stroke-linecap="round" stroke-linejoin="round" />
                                    </svg>
                                }.into_any()
                            } else {
                                view! {
                                    <svg class="w-[18px] h-[18px] animate-icon-normal" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M11.502 13.0003L20.502 13.0003" />
                                        <path d="M13.5019 10.0003C13.5019 10.0003 10.502 12.2097 10.502 13.0003C10.5019 13.7909 13.502 16.0003 13.502 16.0003" />
                                        <path d="M13.998 2.00027H8.99805C8.16962 2.00027 7.49805 2.67185 7.49805 3.50027C7.49805 4.3287 8.16962 5.00027 8.99805 5.00027H13.998C14.8265 5.00027 15.498 4.3287 15.498 3.50027C15.498 2.67185 14.8265 2.00027 13.998 2.00027Z" />
                                        <path d="M15.4981 3.50027C17.0515 3.54709 17.9781 3.72035 18.6194 4.36164C19.4466 5.18885 19.495 6.49068 19.4979 9.00027M7.49795 3.50027C5.94456 3.54708 5.01802 3.72034 4.37673 4.36163C3.49805 5.24031 3.49805 6.65453 3.49806 9.48296L3.49805 15.9998C3.49805 18.8282 3.49806 20.2424 4.37674 21.1211C5.25541 21.9997 6.66963 21.9997 9.49805 21.9997L13.498 21.9997C16.3265 21.9997 17.7407 21.9997 18.6194 21.1211C19.3877 20.3527 19.4842 19.175 19.4963 17.0003" />
                                    </svg>
                                }.into_any()
                            }}
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            {move || if copy_success.get() { "¡Copiado!" } else { "Copiar imagen · Ctrl+C" }}
                        </div>
                    </div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class=move || format!(
                                "w-8.5 h-8.5 rounded-lg transition-all duration-150 flex items-center justify-center cursor-pointer active:scale-95 {}",
                                if save_success.get() { "text-zinc-100 bg-white/15" } else { "text-zinc-400 hover:text-zinc-100 hover:bg-white/5" }
                            )
                            aria-label="Guardar archivo (Ctrl+S)"
                            on:click=on_save
                        >
                            {move || if save_success.get() {
                                view! {
                                    <svg class="w-[18px] h-[18px] animate-icon-success" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                                        <path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" />
                                        <path d="M8 12.5L10.5 15L16 9" stroke-linecap="round" stroke-linejoin="round" />
                                    </svg>
                                }.into_any()
                            } else {
                                view! {
                                    <svg class="w-[18px] h-[18px] animate-icon-normal" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M15.8787 3H11C7.22876 3 5.34315 3 4.17157 4.17157C3 5.34315 3 7.22876 3 11V13C3 16.7712 3 18.6569 4.17157 19.8284C5.34315 21 7.22876 21 11 21H13C16.7712 21 18.6569 21 19.8284 19.8284C21 18.6569 21 16.7712 21 13V8.12132C21 7.66475 21 7.43646 20.9758 7.2174C20.8924 6.4633 20.5963 5.74846 20.122 5.15629C19.9843 4.98427 19.8228 4.82285 19.5 4.5C19.1772 4.17715 19.0157 4.01573 18.8437 3.87795C18.2515 3.40366 17.5367 3.10757 16.7826 3.02421C16.5635 3 16.3353 3 15.8787 3Z" />
                                        <path d="M17 3.5V4C17 5.88562 17 6.82843 16.4142 7.41421C15.8284 8 14.8856 8 13 8H11C9.11438 8 8.17157 8 7.58579 7.41421C7 6.82843 7 5.88562 7 4V3.5" />
                                        <path d="M17 20.5V17C17 15.1144 17 14.1716 16.4142 13.5858C15.8284 13 14.8856 13 13 13H11C9.11438 13 8.17157 13 7.58579 13.5858C7 14.1716 7 15.1144 7 17V20.5" />
                                    </svg>
                                }.into_any()
                            }}
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            {move || if save_success.get() { "¡Guardado!" } else { "Guardar archivo · Ctrl+S" }}
                        </div>
                    </div>

                    <div class="relative tooltip-trigger w-full flex items-center justify-center">
                        <button
                            class="w-8.5 h-8.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/5 transition-all duration-150 flex items-center justify-center cursor-pointer active:scale-95"
                            aria-label="Acerca de Glint"
                            on:click=move |_| set_show_about.set(true)
                        >
                            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12Z" />
                                <path d="M12 16V12" />
                                <path d="M12.125 8.25H12M12.25 8.25C12.25 8.11193 12.1381 8 12 8C11.8619 8 11.75 8.11193 11.75 8.25C11.75 8.38807 11.8619 8.5 12 8.5C12.1381 8.5 12.25 8.38807 12.25 8.25Z" />
                            </svg>
                        </button>
                        <div class="sidebar-tooltip px-2 py-1 bg-zinc-900 border border-white/10 text-zinc-200 text-[11px] font-medium rounded-lg shadow-xl">
                            "Acerca de Glint"
                        </div>
                    </div>
                </div>
            </aside>

            <main
                class=move || format!(
                    "flex-1 overflow-hidden flex items-center justify-center p-2.5 pl-20 bg-zinc-950 relative {}",
                    if is_panning_ui.get() { "cursor-grabbing" } else { "" }
                )
                on:wheel=on_wheel
            >
                <div
                    class="relative w-fit h-fit max-w-full max-h-full flex items-center justify-center shadow-2xl rounded-sm border border-white/15 bg-zinc-950 group"
                    style=move || {
                        let z = zoom_level.get();
                        let (px, py) = pan_offset.get();
                        if z > 1.01 {
                            format!("position: fixed; left: 0; top: 0; transform: translate({}px, {}px) scale({}); transform-origin: 0 0;", px, py, z)
                        } else {
                            String::new()
                        }
                    }
                >
                    <canvas
                        node_ref=canvas_ref
                        class=move || {
                            let z = zoom_level.get();
                            format!(
                                "cursor-crosshair block max-w-full max-h-[calc(100vh-1.5rem)] object-contain rounded-sm {}",
                                if z > 2.0 { "[image-rendering:pixelated]" } else { "" }
                            )
                        }
                        on:mousedown=on_mouse_down
                        on:mousemove=on_mouse_move
                        on:mouseup=on_mouse_up
                        on:mouseleave=on_mouse_leave
                    />

                    {move || if is_ocr_loading.get() {
                        view! {
                            <div class="absolute inset-0 z-20 overflow-hidden pointer-events-none rounded-sm bg-black/40">
                                <div class="absolute top-4 left-1/2 -translate-x-1/2 flex items-center gap-2 py-1.5 px-3 rounded-lg bg-zinc-900 border border-white/10 text-xs text-zinc-200 shadow-xl font-medium">
                                    <svg class="w-3.5 h-3.5 text-zinc-300 animate-spin" viewBox="0 0 24 24" fill="none">
                                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    <span>"Escaneando texto..."</span>
                                </div>
                            </div>
                        }.into_any()
                    } else if is_ocr_active.get() {
                        let blocks = ocr_blocks.get();
                        let (img_w, img_h) = image_dimensions.get();
                        let safe_w = if img_w > 0.0 { img_w } else { 1.0 };
                        let safe_h = if img_h > 0.0 { img_h } else { 1.0 };
                        let (cx, cy) = (safe_w / 2.0, safe_h / 2.0);
                        let max_r = (cx * cx + cy * cy).sqrt().max(1.0);
                        let sel_set = selected_ocr_indices.get();

                        view! {
                            <div class="absolute inset-0 pointer-events-none z-20">
                                {blocks.into_iter().enumerate().map(|(idx, b)| {
                                    let text_to_copy = b.text.clone();
                                    let bx = b.x + b.w / 2.0;
                                    let by = b.y + b.h / 2.0;
                                    let r = ((bx - cx).powi(2) + (by - cy).powi(2)).sqrt();
                                    let delay = ((r / max_r) * 350.0).clamp(0.0, 350.0) as u32;
                                    let left_pct = (b.x / safe_w) * 100.0;
                                    let top_pct = (b.y / safe_h) * 100.0;
                                    let width_pct = (b.w / safe_w) * 100.0;
                                    let height_pct = (b.h / safe_h) * 100.0;
                                    let is_selected = sel_set.contains(&idx);
                                    let maybe_url = detect_url(&b.text);
                                    let maybe_color = detect_color(&b.text);

                                    let box_class = if is_selected {
                                        "absolute rounded-xs pointer-events-auto group/ocr-box z-30 cursor-pointer ocr-box is-selected"
                                    } else {
                                        "absolute rounded-xs pointer-events-auto group/ocr-box animate-ocr-reveal z-20 cursor-pointer ocr-box"
                                    };

                                    view! {
                                        <div
                                            class=box_class
                                            style=format!(
                                                "left: {:.3}%; top: {:.3}%; width: {:.3}%; height: {:.3}%; animation-delay: {}ms;",
                                                left_pct, top_pct, width_pct, height_pct, delay
                                            )
                                            on:click=move |e| {
                                                e.stop_propagation();
                                                copy_text_direct(text_to_copy.clone());
                                            }
                                            on:contextmenu=move |e| {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                set_selected_ocr_indices.update(|set| {
                                                    if set.contains(&idx) {
                                                        set.remove(&idx);
                                                    } else {
                                                        set.insert(idx);
                                                    }
                                                });
                                            }
                                        >
                                            {if let Some(url_str) = maybe_url {
                                                let url_clone = url_str.clone();
                                                view! {
                                                    <div
                                                        class="absolute -top-7 left-0 z-50 flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-zinc-900 border border-white/20 text-[11px] text-zinc-200 shadow-xl opacity-0 group-hover/ocr-box:opacity-100 transition-opacity pointer-events-auto cursor-pointer hover:bg-zinc-800"
                                                        on:click=move |e| {
                                                            e.stop_propagation();
                                                            open_url(url_clone.clone());
                                                        }
                                                    >
                                                        <svg class="w-3 h-3 text-zinc-300" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                            <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path>
                                                            <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>
                                                        </svg>
                                                        <span>"Abrir enlace"</span>
                                                    </div>
                                                }.into_any()
                                            } else if let Some(color_code) = maybe_color {
                                                view! {
                                                    <div class="absolute -top-7 left-0 z-50 flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-zinc-900 border border-white/20 text-[11px] text-zinc-200 shadow-xl opacity-0 group-hover/ocr-box:opacity-100 transition-opacity pointer-events-auto">
                                                        <span class="w-2.5 h-2.5 rounded-full border border-white/30" style=format!("background-color: {};", color_code) />
                                                        <span class="font-mono text-[10px] text-zinc-300">{color_code}</span>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! { <span></span> }.into_any()
                                            }}
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}

                    <div
                        class="absolute -top-[2px] -left-[2px] w-3.5 h-3.5 border-t-2 border-l-2 border-white/80 hover:border-white rounded-tl cursor-nwse-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::TopLeft, e)
                        }
                    />
                    <div
                        class="absolute -top-[2px] left-1/2 -translate-x-1/2 w-5 h-[2.5px] bg-white/70 hover:bg-white rounded-full cursor-ns-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::Top, e)
                        }
                    />
                    <div
                        class="absolute -top-[2px] -right-[2px] w-3.5 h-3.5 border-t-2 border-r-2 border-white/80 hover:border-white rounded-tr cursor-nesw-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::TopRight, e)
                        }
                    />
                    <div
                        class="absolute top-1/2 -right-[2px] -translate-y-1/2 w-[2.5px] h-5 bg-white/70 hover:bg-white rounded-full cursor-ew-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::Right, e)
                        }
                    />
                    <div
                        class="absolute -bottom-[2px] -right-[2px] w-3.5 h-3.5 border-b-2 border-r-2 border-white/80 hover:border-white rounded-br cursor-nwse-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::BottomRight, e)
                        }
                    />
                    <div
                        class="absolute -bottom-[2px] left-1/2 -translate-x-1/2 w-5 h-[2.5px] bg-white/70 hover:bg-white rounded-full cursor-ns-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::Bottom, e)
                        }
                    />
                    <div
                        class="absolute -bottom-[2px] -left-[2px] w-3.5 h-3.5 border-b-2 border-l-2 border-white/80 hover:border-white rounded-bl cursor-nesw-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::BottomLeft, e)
                        }
                    />
                    <div
                        class="absolute top-1/2 -left-[2px] -translate-y-1/2 w-[2.5px] h-5 bg-white/70 hover:bg-white rounded-full cursor-ew-resize z-30 transition-all hover:scale-125 shadow-xs opacity-0 group-hover:opacity-100"
                        on:mousedown={
                            let s = start_crop_drag.clone();
                            move |e| s(CropHandle::Left, e)
                        }
                    />

                    {move || {
                        let dt = dimension_text.get();
                        let active = is_cropping_active.get();
                        if !dt.is_empty() {
                            view! {
                                <div class=move || {
                                    format!(
                                        "absolute -bottom-7 left-1/2 -translate-x-1/2 bg-zinc-900 text-zinc-300 border border-white/10 px-2.5 py-0.5 rounded-lg text-[10px] font-mono shadow-xl pointer-events-none transition-opacity duration-200 {}",
                                        if active { "opacity-100 ring-1 ring-white/30" } else { "opacity-0 group-hover:opacity-90" }
                                    )
                                }>
                                    {dt.clone()}
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }
                    }}
                </div>

                {move || {
                    let z = zoom_level.get();
                    if z > 1.05 {
                        view! {
                            <button
                                class="absolute top-4 left-20 bg-zinc-900 text-zinc-200 border border-white/15 px-2.5 py-1 rounded-lg text-[11px] font-mono shadow-xl hover:bg-zinc-800 cursor-pointer active:scale-95 flex items-center gap-1.5 z-40 transition-all"
                                aria-label="Restablecer zoom (Ctrl+0)"
                                on:click=reset_zoom
                            >
                                <span>{format!("{:.0}%", z * 100.0)}</span>
                                <span class="text-zinc-500 hover:text-zinc-200">"✕"</span>
                            </button>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}

                {move || {
                    if let Some((cx, cy, hex_code)) = picker_preview.get() {
                        let win_w = web_sys::window()
                            .and_then(|w| w.inner_width().ok())
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1920.0);
                        let left = (cx + 16.0).min(win_w - 130.0);
                        let top = (cy - 60.0).max(10.0);
                        view! {
                            <div
                                class="fixed pointer-events-none z-50 flex flex-col items-center bg-zinc-900 border border-white/20 p-2 rounded-lg shadow-2xl animate-toast"
                                style=format!("left: {}px; top: {}px;", left, top)
                            >
                                <div class="w-20 h-20 rounded-lg border-2 border-white/30 overflow-hidden relative shadow-inner bg-black mb-1.5 flex items-center justify-center">
                                    <canvas
                                        node_ref=loupe_canvas_ref
                                        width="80"
                                        height="80"
                                        class="w-20 h-20 block"
                                    />
                                </div>
                                <div class="flex items-center gap-1.5 bg-zinc-800 px-2 py-0.5 rounded-lg border border-white/10">
                                    <span
                                        class="w-2.5 h-2.5 rounded-full border border-white/30"
                                        style=format!("background-color: {}", hex_code)
                                    />
                                    <span class="text-[10px] font-mono font-medium text-zinc-100">{hex_code}</span>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}

                {move || {
                    let st = status_text.get();
                    if !st.is_empty() {
                        view! {
                            <div class="absolute top-3 left-1/2 -translate-x-1/2 bg-zinc-900 text-zinc-100 border border-white/10 px-3.5 py-1.5 rounded-lg text-xs font-medium shadow-2xl flex items-center gap-2 animate-toast z-50 pointer-events-none">
                                <span class="w-1.5 h-1.5 rounded-full bg-[#98c379] animate-pulse"></span>
                                {st}
                            </div>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}

                {move || if is_ocr_active.get() {
                    let sel_set = selected_ocr_indices.get();
                    let sel_count = sel_set.len();
                    let all_blocks = ocr_blocks.get();

                    let preview_blocks: Vec<OcrBlock> = if sel_count > 0 {
                        all_blocks
                            .iter()
                            .cloned()
                            .enumerate()
                            .filter(|(idx, _)| sel_set.contains(idx))
                            .map(|(_, b)| b)
                            .collect()
                    } else {
                        all_blocks.clone()
                    };
                    let sorted_preview = sort_ocr_blocks(&preview_blocks);
                    let preview_text = sorted_preview
                        .iter()
                        .map(|b| b.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let char_count = preview_text.chars().count();
                    let word_count = if preview_text.is_empty() {
                        0
                    } else {
                        preview_text.split_whitespace().count()
                    };

                    let content_height = if sel_count == 0 {
                        90.0
                    } else {
                        ((preview_text.len() as f64 / 22.0).ceil() * 19.0 + 24.0).clamp(70.0, 240.0)
                    };

                    let c_sel = copy_selected_ocr.clone();
                    let all_blocks_len = all_blocks.len();

                    view! {
                        <div>
                            <div class="fixed bottom-14 right-4 z-40 w-80 bg-zinc-900 border border-white/12 rounded-lg shadow-2xl flex flex-col p-3 animate-dropdown gap-2.5 ocr-preview-card">
                                <div class="flex items-center justify-between">
                                    <div class="flex items-center gap-2">
                                        <span class=format!("w-2 h-2 rounded-full transition-colors duration-200 {}", if sel_count > 0 { "bg-[#98c379]" } else { "bg-zinc-600" }) />
                                        <span class="text-xs font-medium text-zinc-100">
                                            {if sel_count > 0 {
                                                format!("Selección ({} bloques)", sel_count)
                                            } else {
                                                "Selección múltiple".to_string()
                                            }}
                                        </span>
                                    </div>
                                    {if sel_count > 0 {
                                        let c_sel_btn = copy_selected_ocr.clone();
                                        view! {
                                            <button
                                                class="px-2.5 py-1 text-[11px] font-medium rounded-md bg-[#98c379] text-zinc-950 hover:bg-[#8ab56b] transition-colors cursor-pointer flex items-center gap-1 shadow-xs active:scale-95 animate-dropdown"
                                                on:click=move |_| c_sel_btn()
                                            >
                                                <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                    <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path>
                                                    <rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect>
                                                </svg>
                                                "Copiar"
                                            </button>
                                        }.into_any()
                                    } else {
                                        let c_all_btn = copy_all_ocr.clone();
                                        view! {
                                            <button
                                                class="px-2.5 py-1 text-[11px] font-medium rounded-md bg-white/10 text-zinc-300 hover:bg-white/20 hover:text-white transition-colors cursor-pointer flex items-center gap-1 shadow-xs active:scale-95"
                                                on:click=move |_| c_all_btn()
                                            >
                                                "Copiar todo"
                                            </button>
                                        }.into_any()
                                    }}
                                </div>

                                <div
                                    node_ref=preview_scroll_ref
                                    class="overflow-y-auto p-2.5 rounded-lg bg-zinc-950 border border-white/10 font-mono text-[11.5px] text-zinc-200 leading-relaxed whitespace-pre-wrap select-text selection:bg-[#98c379]/30 scroll-smooth ocr-preview-content"
                                    style=format!("height: {:.1}px;", content_height)
                                >
                                    {if sel_count == 0 {
                                        view! {
                                            <div class="h-full flex flex-col items-center justify-center text-center text-zinc-500 gap-1.5 py-1 animate-dropdown">
                                                <svg class="w-5 h-5 text-zinc-600/80" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6">
                                                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                                                    <polyline points="14 2 14 8 20 8"></polyline>
                                                    <line x1="16" y1="13" x2="8" y2="13"></line>
                                                    <line x1="16" y1="17" x2="8" y2="17"></line>
                                                    <polyline points="10 9 9 9 8 9"></polyline>
                                                </svg>
                                                <span class="text-xs text-zinc-400 font-medium">"Ningún texto seleccionado"</span>
                                                <span class="text-[10.5px] text-zinc-500 max-w-[210px] leading-tight">"Usa clic derecho sobre los bloques para encadenar textos"</span>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="animate-dropdown">
                                                {preview_text}
                                            </div>
                                        }.into_any()
                                    }}
                                </div>

                                <div class="flex items-center justify-between text-[10px] text-zinc-400 font-mono pt-1 border-t border-white/10">
                                    <span>{format!("{} caracteres", char_count)}</span>
                                    <span>{format!("{} palabras", word_count)}</span>
                                </div>
                            </div>

                            <div class="fixed bottom-3.5 left-1/2 -translate-x-1/2 z-40 flex items-center flex-nowrap whitespace-nowrap gap-2.5 py-1.5 px-3 rounded-lg bg-zinc-900 border border-white/12 shadow-2xl text-[11px] text-zinc-300 animate-toast">
                                <div class="flex items-center gap-1 shrink-0">
                                    <span class="px-1.5 py-0.5 rounded bg-white/10 text-zinc-200 font-mono text-[10px]">"Clic"</span>
                                    <span>"Copiar"</span>
                                </div>
                                <div class="w-px h-3 bg-white/15 shrink-0" />
                                <div class="flex items-center gap-1 shrink-0">
                                    <span class="px-1.5 py-0.5 rounded bg-white/10 text-zinc-200 font-mono text-[10px]">"Clic Der"</span>
                                    <span>"Seleccionar"</span>
                                </div>
                                <div class="w-px h-3 bg-white/15 shrink-0" />
                                <div
                                    class="flex items-center gap-1 shrink-0 cursor-pointer hover:text-white transition-colors"
                                    on:click=move |_| {
                                        set_selected_ocr_indices.set((0..all_blocks_len).collect());
                                    }
                                >
                                    <span class="px-1.5 py-0.5 rounded bg-white/10 text-zinc-200 font-mono text-[10px]">"Ctrl+A"</span>
                                    <span>"Todo"</span>
                                </div>
                                <div class="w-px h-3 bg-white/15 shrink-0" />
                                {if sel_count > 0 {
                                    view! {
                                        <div class="flex items-center gap-2 shrink-0">
                                            <button
                                                class="px-2.5 py-0.5 rounded bg-[#98c379] text-zinc-950 font-medium text-[11px] hover:bg-[#8ab56b] transition-colors cursor-pointer shrink-0"
                                                on:click=move |_| c_sel()
                                            >
                                                {format!("Copiar {} sel. (Enter)", sel_count)}
                                            </button>
                                            <button
                                                class="text-zinc-400 hover:text-zinc-200 cursor-pointer shrink-0"
                                                on:click=move |_| set_selected_ocr_indices.set(BTreeSet::new())
                                            >
                                                "Desmarcar (Esc)"
                                            </button>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="flex items-center gap-1 shrink-0">
                                            <span class="px-1.5 py-0.5 rounded bg-white/10 text-zinc-200 font-mono text-[10px]">"Esc"</span>
                                            <span>"Salir"</span>
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}

                {move || if show_about.get() {
                    let is_closing = is_about_closing.get();
                    let on_close_modal = move |_| close_about();
                    view! {
                        <div
                            class=move || format!(
                                "fixed inset-0 bg-black/75 flex items-center justify-center z-50 p-4 select-none {}",
                                if is_closing { "animate-overlay-exit" } else { "animate-overlay-enter" }
                            )
                            on:click=on_close_modal
                        >
                            <div
                                class=move || format!(
                                    "bg-zinc-900 border border-white/10 rounded-lg shadow-2xl max-w-[490px] w-full p-5 relative overflow-hidden flex flex-col gap-4 ring-1 ring-white/5 {}",
                                    if is_closing { "animate-modal-exit" } else { "animate-modal-enter" }
                                )
                                on:click=move |e| e.stop_propagation()
                            >
                                <div class="flex items-start justify-between">
                                    <div class="flex items-center gap-3">
                                        <div class="w-10 h-10 rounded-lg bg-zinc-800 border border-white/10 flex items-center justify-center text-zinc-100 shadow-inner shrink-0">
                                            <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                                                <path d="M12 2L14.4 9.6L22 12L14.4 14.4L12 22L9.6 14.4L2 12L9.6 9.6L12 2Z" />
                                            </svg>
                                        </div>
                                        <div class="flex flex-col">
                                            <div class="flex items-center gap-2">
                                                <h2 class="text-sm font-semibold text-zinc-100 tracking-tight">"Glint"</h2>
                                                <span class="text-[10px] font-mono px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-300 border border-white/10 leading-none">"v0.1.0"</span>
                                            </div>
                                            <p class="text-[11px] text-zinc-400 mt-0.5">"Captura y anotación rápida en pantalla"</p>
                                        </div>
                                    </div>
                                    <button
                                        class="text-zinc-400 hover:text-zinc-100 p-1.5 rounded-lg hover:bg-white/5 transition-colors cursor-pointer shrink-0 -mr-1 -mt-1"
                                        aria-label="Cerrar ventana Acerca de"
                                        on:click=on_close_modal
                                    >
                                        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                            <path d="M18 6L12 12M12 12L6 18M12 12L18 18M12 12L6 6" />
                                        </svg>
                                    </button>
                                </div>

                                <div class="h-px bg-white/10"></div>

                                <div class="grid grid-cols-2 gap-2.5">
                                    <div class="bg-white/[0.02] border border-white/5 rounded-lg p-2.5 flex flex-col gap-1.5">
                                        <span class="text-[10px] font-semibold uppercase tracking-wider text-zinc-400 px-1 mb-0.5">"Herramientas"</span>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Lápiz"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"P"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Resaltador"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"H"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Rectángulo"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"R"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Flecha"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"A"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Círculo"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"C"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Censurar / Blur"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"B"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Gotero"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"I"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Texto OCR"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"O"</kbd>
                                        </div>
                                    </div>

                                    <div class="bg-white/[0.02] border border-white/5 rounded-lg p-2.5 flex flex-col gap-1.5">
                                        <span class="text-[10px] font-semibold uppercase tracking-wider text-zinc-400 px-1 mb-0.5">"Acciones"</span>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Copiar imagen"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Ctrl+C"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Guardar archivo"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Ctrl+S"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Deshacer"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Ctrl+Z"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Rehacer"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Ctrl+Y"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Limpiar trazos"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Ctrl+K"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Grosor trazo"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"[ / ]"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Zoom / Pan"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Rueda"</kbd>
                                        </div>
                                        <div class="flex items-center justify-between py-1 px-1.5 rounded-lg hover:bg-white/[0.04] transition-colors">
                                            <span class="text-[11.5px] text-zinc-300">"Cerrar"</span>
                                            <kbd class="px-1.5 py-0.5 rounded-lg bg-white/10 text-zinc-200 font-mono text-[10px] border border-white/10 leading-none">"Esc"</kbd>
                                        </div>
                                    </div>
                                </div>

                                <div class="flex items-center justify-between pt-2 border-t border-white/10 text-[11px] text-zinc-500">
                                    <span>"Rust · Tauri · Leptos"</span>
                                    <span class="text-zinc-400 font-mono">"Esc para cerrar"</span>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </main>
        </div>
    }
}
