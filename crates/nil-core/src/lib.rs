use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub open_editor: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { open_editor: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OcrEntity {
    pub kind: String,
    pub value: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OcrBlock {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub entities: Vec<OcrEntity>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GalleryItem {
    pub path: String,
    pub filename: String,
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    pub preview_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GalleryResponse {
    pub items: Vec<GalleryItem>,
    pub total: usize,
    pub has_more: bool,
}

pub fn get_config_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(home).join(".config").join("screenshot-tool");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn load_config(config_path: &PathBuf) -> Config {
    if let Ok(content) = fs::read_to_string(config_path) {
        if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
            return cfg;
        }
    }
    Config::default()
}

pub fn save_config_file(config_path: &PathBuf, cfg: &Config) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn copy_image_bytes_to_clipboard(bytes: &[u8]) -> Result<(), String> {
    if let Ok(mut child) = Command::new("wl-copy")
        .arg("--type")
        .arg("image/png")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(bytes);
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return Ok(());
            }
        }
    }

    if let Ok(mut child) = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .arg("-t")
        .arg("image/png")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(bytes);
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return Ok(());
            }
        }
    }

    Err("No se pudo copiar la imagen al portapapeles".to_string())
}

pub fn copy_text_bytes_to_clipboard(text: &str) -> Result<(), String> {
    if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return Ok(());
            }
        }
    }

    if let Ok(mut child) = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait() {
            if status.success() {
                return Ok(());
            }
        }
    }

    Err("No se pudo copiar el texto al portapapeles".to_string())
}

pub fn copy_file_to_clipboard(path: &PathBuf) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    copy_image_bytes_to_clipboard(&bytes)
}

pub fn generate_temp_path() -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    PathBuf::from(format!("/tmp/screenshot_{}.png", ts))
}

pub fn get_png_dimensions(path: &PathBuf) -> (u32, u32) {
    if let Ok(bytes) = fs::read(path) {
        if bytes.len() >= 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
            let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
            let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
            return (w, h);
        }
    }
    (960, 600)
}

pub fn get_screenshots_dirs() -> Vec<PathBuf> {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut dirs = Vec::new();
    let home_path = PathBuf::from(&home);

    let spanish_capturas = home_path.join("Im\u{00e1}genes").join("Capturas de pantalla");
    if spanish_capturas.exists() {
        dirs.push(spanish_capturas);
    }
    let english_screenshots = home_path.join("Pictures").join("Screenshots");
    if english_screenshots.exists() {
        dirs.push(english_screenshots);
    }
    let pictures = home_path.join("Pictures");
    if pictures.exists() && !dirs.contains(&pictures) {
        dirs.push(pictures);
    }
    let imagenes = home_path.join("Im\u{00e1}genes");
    if imagenes.exists() && !dirs.contains(&imagenes) {
        dirs.push(imagenes);
    }
    dirs
}

pub fn list_gallery_items(offset: usize, limit: usize) -> Result<GalleryResponse, String> {
    let dirs = get_screenshots_dirs();
    let mut all_files: Vec<(PathBuf, SystemTime)> = Vec::new();

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if ext_lower == "png"
                            || ext_lower == "jpg"
                            || ext_lower == "jpeg"
                            || ext_lower == "webp"
                        {
                            let mtime = entry
                                .metadata()
                                .and_then(|m| m.modified())
                                .unwrap_or(UNIX_EPOCH);
                            all_files.push((path, mtime));
                        }
                    }
                }
            }
        }
    }

    all_files.sort_by(|a, b| b.1.cmp(&a.1));
    all_files.dedup_by(|a, b| a.0 == b.0);

    let total = all_files.len();
    let effective_limit = if limit == 0 { 5 } else { limit };
    let slice = all_files.into_iter().skip(offset).take(effective_limit);

    let mut items = Vec::new();
    for (path, mtime) in slice {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let timestamp = mtime
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (width, height) = get_png_dimensions(&path);
        let preview_base64 = if let Ok(bytes) = fs::read(&path) {
            let enc = base64::engine::general_purpose::STANDARD.encode(&bytes);
            format!("data:image/png;base64,{}", enc)
        } else {
            String::new()
        };

        items.push(GalleryItem {
            path: path.to_string_lossy().to_string(),
            filename,
            timestamp,
            width,
            height,
            preview_base64,
        });
    }

    let has_more = offset + items.len() < total;

    Ok(GalleryResponse {
        items,
        total,
        has_more,
    })
}

pub fn run_tesseract(path_to_process: &PathBuf) -> Result<Vec<OcrBlock>, String> {
    let output = Command::new("tesseract")
        .arg(path_to_process)
        .arg("stdout")
        .arg("tsv")
        .arg("-l")
        .arg("eng+spa")
        .arg("--psm")
        .arg("11")
        .arg("-c")
        .arg("preserve_interword_spaces=1")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => Command::new("tesseract")
            .arg(path_to_process)
            .arg("stdout")
            .arg("tsv")
            .arg("--psm")
            .arg("11")
            .output()
            .map_err(|e| format!("Tesseract no disponible: {}", e))?,
    };

    let tsv_str = String::from_utf8_lossy(&output.stdout);

    struct LineAccumulator {
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        words: Vec<String>,
        entities: Vec<OcrEntity>,
    }

    let mut lines_map: BTreeMap<(i32, i32, i32), LineAccumulator> = BTreeMap::new();

    for row in tsv_str.lines().skip(1) {
        let cols: Vec<&str> = row.split('\t').collect();
        if cols.len() >= 12 {
            let level: i32 = cols[0].parse().unwrap_or(0);
            let block_num: i32 = cols[2].parse().unwrap_or(0);
            let par_num: i32 = cols[3].parse().unwrap_or(0);
            let line_num: i32 = cols[4].parse().unwrap_or(0);
            let left: f64 = cols[6].parse().unwrap_or(0.0);
            let top: f64 = cols[7].parse().unwrap_or(0.0);
            let width: f64 = cols[8].parse().unwrap_or(0.0);
            let height: f64 = cols[9].parse().unwrap_or(0.0);
            let conf: f64 = cols[10].parse().unwrap_or(0.0);
            let text = cols[11].trim().to_string();

            if level == 5 && conf > 15.0 && !text.is_empty() && width > 2.0 && height > 2.0 {
                let key = (block_num, par_num, line_num);
                let right = left + width;
                let bottom = top + height;

                let mut entity_opt = None;
                let clean_url = text.trim_matches(|c: char| {
                    c == '"'
                        || c == '\''
                        || c == '`'
                        || c == '('
                        || c == ')'
                        || c == '['
                        || c == ']'
                        || c == '<'
                        || c == '>'
                        || c == '{'
                        || c == '}'
                        || c == ','
                        || c == ';'
                        || c == '.'
                });
                if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
                    entity_opt = Some(OcrEntity {
                        kind: "url".to_string(),
                        value: clean_url.to_string(),
                        x: left,
                        y: top,
                        w: width,
                        h: height,
                    });
                } else if clean_url.starts_with("www.") {
                    entity_opt = Some(OcrEntity {
                        kind: "url".to_string(),
                        value: format!("https://{}", clean_url),
                        x: left,
                        y: top,
                        w: width,
                        h: height,
                    });
                } else if (clean_url.contains(".com")
                    || clean_url.contains(".org")
                    || clean_url.contains(".io")
                    || clean_url.contains(".dev")
                    || clean_url.contains(".net")
                    || clean_url.contains(".app")
                    || clean_url.contains(".ar"))
                    && clean_url.contains('.')
                    && !clean_url.contains('@')
                    && clean_url.len() >= 4
                {
                    entity_opt = Some(OcrEntity {
                        kind: "url".to_string(),
                        value: format!("https://{}", clean_url),
                        x: left,
                        y: top,
                        w: width,
                        h: height,
                    });
                } else {
                    let clean_col = text.trim_matches(|c: char| {
                        c == '"'
                            || c == '\''
                            || c == '`'
                            || c == '('
                            || c == ')'
                            || c == '['
                            || c == ']'
                            || c == '{'
                            || c == '}'
                            || c == ','
                            || c == ';'
                            || c == ':'
                    });
                    if clean_col.starts_with('#') {
                        let hex = &clean_col[1..];
                        if (hex.len() == 3 || hex.len() == 4 || hex.len() == 6 || hex.len() == 8)
                            && hex.chars().all(|c| c.is_ascii_hexdigit())
                        {
                            entity_opt = Some(OcrEntity {
                                kind: "color".to_string(),
                                value: clean_col.to_string(),
                                x: left,
                                y: top,
                                w: width,
                                h: height,
                            });
                        }
                    } else if (clean_col.starts_with("rgb(")
                        || clean_col.starts_with("rgba(")
                        || clean_col.starts_with("hsl(")
                        || clean_col.starts_with("hsla("))
                        && clean_col.ends_with(')')
                    {
                        entity_opt = Some(OcrEntity {
                            kind: "color".to_string(),
                            value: clean_col.to_string(),
                            x: left,
                            y: top,
                            w: width,
                            h: height,
                        });
                    }
                }

                if let Some(acc) = lines_map.get_mut(&key) {
                    acc.min_x = acc.min_x.min(left);
                    acc.min_y = acc.min_y.min(top);
                    acc.max_x = acc.max_x.max(right);
                    acc.max_y = acc.max_y.max(bottom);
                    acc.words.push(text);
                    if let Some(ent) = entity_opt {
                        acc.entities.push(ent);
                    }
                } else {
                    lines_map.insert(
                        key,
                        LineAccumulator {
                            min_x: left,
                            min_y: top,
                            max_x: right,
                            max_y: bottom,
                            words: vec![text],
                            entities: entity_opt.into_iter().collect(),
                        },
                    );
                }
            }
        }
    }

    let mut blocks = Vec::new();
    for (_, acc) in lines_map {
        let text = acc.words.join(" ");
        let w = acc.max_x - acc.min_x;
        let h = acc.max_y - acc.min_y;
        if !text.is_empty() && w > 4.0 && h > 4.0 {
            blocks.push(OcrBlock {
                text,
                x: acc.min_x,
                y: acc.min_y,
                w,
                h,
                entities: acc.entities,
            });
        }
    }

    Ok(blocks)
}

fn get_hyprland_focused_output() -> Option<String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    if let Ok(monitors) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(array) = monitors.as_array() {
            for m in array {
                if m.get("focused")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    None
}

fn get_sway_focused_output() -> Option<String> {
    let output = Command::new("swaymsg")
        .args(["-t", "get_outputs", "-r"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    if let Ok(monitors) = serde_json::from_str::<serde_json::Value>(&json_str) {
        if let Some(array) = monitors.as_array() {
            for m in array {
                if m.get("focused")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    if let Some(name) = m.get("name").and_then(|v| v.as_str()) {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn capture_area_image(target_path: &PathBuf) -> bool {
    if let Ok(output) = Command::new("slurp").output() {
        if output.status.success() {
            let geometry = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !geometry.is_empty() {
                if let Ok(status) = Command::new("grim")
                    .arg("-g")
                    .arg(&geometry)
                    .arg(target_path)
                    .status()
                {
                    if status.success() && target_path.exists() {
                        return true;
                    }
                }
            }
        }
    }

    if let Ok(status) = Command::new("gnome-screenshot")
        .arg("-a")
        .arg("-f")
        .arg(target_path)
        .status()
    {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    if let Ok(output) = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell.Screenshot",
            "--object-path",
            "/org/gnome/Shell/Screenshot",
            "--method",
            "org.gnome.Shell.Screenshot.SelectArea",
        ])
        .output()
    {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            let numbers: Vec<i32> = out_str
                .split(|c: char| !c.is_numeric() && c != '-')
                .filter_map(|s| s.parse::<i32>().ok())
                .collect();
            if numbers.len() >= 4 {
                let (x, y, w, h) = (numbers[0], numbers[1], numbers[2], numbers[3]);
                if w > 0 && h > 0 {
                    let path_str = target_path.to_string_lossy().to_string();
                    if let Ok(status) = Command::new("gdbus")
                        .args([
                            "call",
                            "--session",
                            "--dest",
                            "org.gnome.Shell.Screenshot",
                            "--object-path",
                            "/org/gnome/Shell/Screenshot",
                            "--method",
                            "org.gnome.Shell.Screenshot.ScreenshotArea",
                            &x.to_string(),
                            &y.to_string(),
                            &w.to_string(),
                            &h.to_string(),
                            "false",
                            &path_str,
                        ])
                        .status()
                    {
                        if status.success() && target_path.exists() {
                            return true;
                        }
                    }
                }
            }
        }
    }

    if let Ok(status) = Command::new("maim").arg("-s").arg(target_path).status() {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    if let Ok(status) = Command::new("scrot").arg("-s").arg(target_path).status() {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    false
}

pub fn capture_full_image(target_path: &PathBuf) -> bool {
    let focused_output = get_hyprland_focused_output().or_else(get_sway_focused_output);

    if let Some(output_name) = focused_output {
        if let Ok(status) = Command::new("grim")
            .arg("-o")
            .arg(&output_name)
            .arg(target_path)
            .status()
        {
            if status.success() && target_path.exists() {
                return true;
            }
        }
    }

    if let Ok(status) = Command::new("grim").arg(target_path).status() {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    let path_str = target_path.to_string_lossy().to_string();
    if let Ok(status) = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.gnome.Shell.Screenshot",
            "--object-path",
            "/org/gnome/Shell/Screenshot",
            "--method",
            "org.gnome.Shell.Screenshot.Screenshot",
            "true",
            "false",
            &path_str,
        ])
        .status()
    {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    if let Ok(status) = Command::new("gnome-screenshot")
        .arg("-f")
        .arg(target_path)
        .status()
    {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    if let Ok(status) = Command::new("maim").arg(target_path).status() {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    if let Ok(status) = Command::new("scrot").arg(target_path).status() {
        if status.success() && target_path.exists() {
            return true;
        }
    }

    false
}

pub fn notify_and_maybe_edit(path: &PathBuf, title: &str, body: &str) -> bool {
    let _ = copy_file_to_clipboard(path);

    let output = Command::new("notify-send")
        .arg("-a")
        .arg("Glint")
        .arg("-i")
        .arg(path)
        .arg("-A")
        .arg("edit=Editar")
        .arg("-t")
        .arg("5000")
        .arg(title)
        .arg(body)
        .output();

    if let Ok(out) = output {
        let choice = String::from_utf8_lossy(&out.stdout).trim().to_string();
        choice == "edit" || choice == "0" || choice == "default"
    } else {
        false
    }
}

pub fn get_hold_lock_path() -> PathBuf {
    PathBuf::from("/tmp/glint_hold.lock")
}
