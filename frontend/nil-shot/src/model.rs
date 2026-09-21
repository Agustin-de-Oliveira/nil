use leptos::prelude::*;
use serde::{Deserialize, Serialize};

pub const COLOR_LIST: &[(&str, &str)] = &[
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

pub const WIDTHS: &[(f64, &str)] = &[
    (2.5, "h-[2px]"),
    (5.0, "h-[4px]"),
    (10.0, "h-[7px]"),
    (18.0, "h-[12px]"),
];

pub const COMMON_ASPECT_RATIOS: &[((u32, u32), f64)] = &[
    ((16, 9), 16.0 / 9.0),
    ((16, 10), 16.0 / 10.0),
    ((4, 3), 4.0 / 3.0),
    ((3, 2), 3.0 / 2.0),
    ((1, 1), 1.0),
    ((9, 16), 9.0 / 16.0),
    ((4, 5), 4.0 / 5.0),
    ((3, 4), 3.0 / 4.0),
    ((2, 3), 2.0 / 3.0),
    ((21, 9), 21.0 / 9.0),
];

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
pub struct AppConfig {
    #[serde(default = "default_open_editor", alias = "openEditor")]
    pub open_editor: bool,
    #[serde(default, alias = "closeOnCopy")]
    pub close_on_copy: bool,
    #[serde(default = "default_stroke_width", alias = "defaultStrokeWidth")]
    pub default_stroke_width: f64,
    #[serde(default = "default_color", alias = "defaultColor")]
    pub default_color: String,
}

fn default_open_editor() -> bool {
    true
}

fn default_stroke_width() -> f64 {
    5.0
}

fn default_color() -> String {
    "#e06c75".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            open_editor: true,
            close_on_copy: false,
            default_stroke_width: 5.0,
            default_color: "#e06c75".to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct SetConfigArgs {
    pub config: AppConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OcrEntity {
    pub kind: String,
    pub value: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OcrBlock {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub entities: Vec<OcrEntity>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GalleryItem {
    pub path: String,
    pub filename: String,
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    pub preview_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GalleryResponse {
    pub items: Vec<GalleryItem>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct GetGalleryArgs {
    pub offset: usize,
    pub limit: usize,
}

#[derive(Serialize)]
pub struct LoadGalleryArgs {
    pub path: String,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Pen,
    Highlighter,
    Arrow,
    Line,
    Rectangle,
    Circle,
    Blur,
    Picker,
}

pub fn render_tool_icon(tool: Tool) -> impl IntoView {
    match tool {
        Tool::Select => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 4l7.07 17 2.51-7.39L21 11.07 4 4z" />
            </svg>
        }.into_any(),
        Tool::Pen => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3.49977 18.9853V20.5H5.01449C6.24074 20.5 6.85387 20.5 7.40518 20.2716C7.9565 20.0433 8.39004 19.6097 9.25713 18.7426L19.1211 8.87868C20.0037 7.99612 20.4449 7.55483 20.4937 7.01325C20.5018 6.92372 20.5018 6.83364 20.4937 6.74411C20.4449 6.20253 20.0037 5.76124 19.1211 4.87868C18.2385 3.99612 17.7972 3.55483 17.2557 3.50605C17.1661 3.49798 17.0761 3.49798 16.9865 3.50605C16.4449 3.55483 16.0037 3.99612 15.1211 4.87868L5.25713 14.7426C4.39004 15.6097 3.9565 16.0433 3.72813 16.5946C3.49977 17.1459 3.49977 17.759 3.49977 18.9853Z" />
                <path d="M13.5 6.5L17.5 10.5" />
            </svg>
        }.into_any(),
        Tool::Highlighter => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M6.6777 16.2071L8.79289 18.3223M6.6777 16.2071L2.5 20.5H6.5L8.79289 18.3223M6.6777 16.2071C6.28717 15.8166 6.29534 15.1872 6.63537 14.752C7.42742 13.7383 7.71531 12.8216 7.79924 12.1382C7.89158 11.3863 8.07366 10.5734 8.60933 10.0377L9.50122 9.14828M8.79289 18.3223C9.18342 18.7128 9.81278 18.7047 10.248 18.3646C11.2617 17.5726 12.1784 17.2847 12.8618 17.2008C13.6137 17.1084 14.4266 16.9263 14.9623 16.3907L15.8517 15.4988M15.8517 15.4988L9.50122 9.14828M15.8517 15.4988C16.2422 15.8893 16.8754 15.8893 17.2659 15.4988L21.5 11.2647M9.50122 9.14828C9.1107 8.75776 9.1107 8.12459 9.50122 7.73407L13.7353 3.5" />
            </svg>
        }.into_any(),
        Tool::Rectangle => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3.89124 3.89124C5.28249 2.5 7.52166 2.5 12 2.5C16.4783 2.5 18.7175 2.5 20.1088 3.89124C21.5 5.28249 21.5 7.52166 21.5 12C21.5 16.4783 21.5 18.7175 20.1088 20.1088C18.7175 21.5 16.4783 21.5 12 21.5C7.52166 21.5 5.28249 21.5 3.89124 20.1088C2.5 18.7175 2.5 16.4783 2.5 12C2.5 7.52166 2.5 5.28249 3.89124 3.89124Z" />
            </svg>
        }.into_any(),
        Tool::Arrow => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 12L4 12" />
                <path d="M18.5859 13.6026L17.6194 14.3639C16.0536 15.5974 15.2707 16.2141 14.6354 15.9328C14 15.6515 14 14.6881 14 12.7613L14 11.2387C14 9.31191 14 8.34853 14.6354 8.06721C15.2707 7.7859 16.0536 8.40264 17.6194 9.63612L18.5858 10.3974C19.5286 11.1401 20 11.5115 20 12C20 12.4885 19.5286 12.8599 18.5859 13.6026Z" />
            </svg>
        }.into_any(),
        Tool::Line => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M5 19L19 5" />
            </svg>
        }.into_any(),
        Tool::Circle => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" />
            </svg>
        }.into_any(),
        Tool::Blur => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                <path d="M6.43385 6.51953C4.22009 7.89049 2.93281 9.86457 2.31858 11.0339C2.10621 11.4382 2.00003 11.6403 2 12.0082C1.99997 12.3761 2.10584 12.5777 2.3176 12.981C3.32862 14.9066 6.16702 19.0195 11.9669 19.0195C14.2454 19.0195 16.0669 18.3848 17.5 17.4972" stroke-linejoin="round" />
                <path d="M9.87868 9.87868C9.33579 10.4216 9 11.1716 9 12C9 13.6569 10.3431 15 12 15C12.8284 15 13.5784 14.6642 14.1213 14.1213" />
                <path d="M2 2L22 22" stroke-linejoin="round" />
                <path d="M10 5.14847C10.5934 5.05255 11.224 5 11.8936 5C17.7747 5 20.6528 9.05385 21.6779 10.9517C21.8927 11.3492 22 11.548 22 11.9106C22 12.2733 21.8921 12.4727 21.6765 12.8717C21.3678 13.4428 20.8916 14.2085 20.2167 15" stroke-linejoin="round" />
            </svg>
        }.into_any(),
        Tool::Picker => view! {
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M19 11L12 4M19 11L13 17L9 13L15 7L19 11ZM19 11L21 9L15 3L13 5M9 13L5 17L3 21L7 19L11 15" />
            </svg>
        }.into_any(),
    }
}

pub fn tool_info(tool: Tool) -> (&'static str, &'static str) {
    match tool {
        Tool::Select => ("Seleccionar (1)", "Seleccionar · 1"),
        Tool::Pen => ("Lápiz (2)", "Lápiz · 2"),
        Tool::Highlighter => ("Resaltar (2)", "Resaltar · 2"),
        Tool::Rectangle => ("Rectángulo (3)", "Rectángulo · 3"),
        Tool::Arrow => ("Flecha (3)", "Flecha · 3"),
        Tool::Line => ("Línea (3)", "Línea · 3"),
        Tool::Circle => ("Círculo (3)", "Círculo · 3"),
        Tool::Blur => ("Censurar (B)", "Censurar · B"),
        Tool::Picker => ("Gotero (I)", "Gotero · I"),
    }
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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
    Line {
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

impl DrawingItem {
    pub fn translate(&mut self, dx: f64, dy: f64) {
        match self {
            DrawingItem::Blur { start, end }
            | DrawingItem::Arrow { start, end, .. }
            | DrawingItem::Line { start, end, .. }
            | DrawingItem::Rectangle { start, end, .. }
            | DrawingItem::Circle { start, end, .. } => {
                start.x += dx;
                start.y += dy;
                end.x += dx;
                end.y += dy;
            }
            DrawingItem::Freehand { points, .. } => {
                for pt in points.iter_mut() {
                    pt.x += dx;
                    pt.y += dy;
                }
            }
        }
    }

    pub fn rotate_90(&mut self, clockwise: bool, width: f64, height: f64) {
        match self {
            DrawingItem::Blur { start, end }
            | DrawingItem::Arrow { start, end, .. }
            | DrawingItem::Line { start, end, .. }
            | DrawingItem::Rectangle { start, end, .. }
            | DrawingItem::Circle { start, end, .. } => {
                let (sx, sy) = (start.x, start.y);
                let (ex, ey) = (end.x, end.y);
                if clockwise {
                    start.x = height - sy;
                    start.y = sx;
                    end.x = height - ey;
                    end.y = ex;
                } else {
                    start.x = sy;
                    start.y = width - sx;
                    end.x = ey;
                    end.y = width - ex;
                }
            }
            DrawingItem::Freehand { points, .. } => {
                for pt in points.iter_mut() {
                    let (px, py) = (pt.x, pt.y);
                    if clockwise {
                        pt.x = height - py;
                        pt.y = px;
                    } else {
                        pt.x = py;
                        pt.y = width - px;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum HistoryAction {
    Add(DrawingItem),
    Delete {
        index: usize,
        item: DrawingItem,
    },
    Clear(Vec<DrawingItem>),
    Move {
        index: usize,
        prev: DrawingItem,
        new: DrawingItem,
    },
    Modify {
        index: usize,
        prev: DrawingItem,
        new: DrawingItem,
    },
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
    Rotate {
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

pub fn set_item_color(item: &mut DrawingItem, new_color: String) {
    match item {
        DrawingItem::Freehand { color, .. }
        | DrawingItem::Arrow { color, .. }
        | DrawingItem::Line { color, .. }
        | DrawingItem::Rectangle { color, .. }
        | DrawingItem::Circle { color, .. } => {
            *color = new_color;
        }
        DrawingItem::Blur { .. } => {}
    }
}

pub fn set_item_width(item: &mut DrawingItem, new_width: f64) {
    match item {
        DrawingItem::Freehand { width, .. }
        | DrawingItem::Arrow { width, .. }
        | DrawingItem::Line { width, .. }
        | DrawingItem::Rectangle { width, .. }
        | DrawingItem::Circle { width, .. } => {
            *width = new_width;
        }
        DrawingItem::Blur { .. } => {}
    }
}

#[derive(Default, Clone)]
pub struct CanvasSession {
    pub is_drawing: bool,
    pub start_point: Option<Point>,
    pub current_points: Vec<Point>,
    pub hovered_item_index: Option<usize>,
    pub drag_item_state: Option<(usize, DrawingItem, Point, f64)>,
    pub is_panning: bool,
    pub pan_start_mouse: (f64, f64),
    pub pan_initial: (f64, f64),
    pub crop_handle: Option<CropHandle>,
    pub crop_start_mouse: Point,
    pub current_crop_rect: Rect,
}

