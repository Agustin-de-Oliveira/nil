use crate::model::{CropHandle, DrawingItem, Point, COMMON_ASPECT_RATIOS};

pub fn dist_to_segment_sq(p: &Point, a: &Point, b: &Point) -> f64 {
    let l2 = (b.x - a.x).powi(2) + (b.y - a.y).powi(2);
    if l2 <= 0.0001 {
        return (p.x - a.x).powi(2) + (p.y - a.y).powi(2);
    }
    let t = (((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2).clamp(0.0, 1.0);
    let proj_x = a.x + t * (b.x - a.x);
    let proj_y = a.y + t * (b.y - a.y);
    (p.x - proj_x).powi(2) + (p.y - proj_y).powi(2)
}

pub fn hit_test_item(item: &DrawingItem, p: &Point) -> bool {
    let margin = 10.0f64;
    let margin_sq = margin * margin;
    match item {
        DrawingItem::Arrow { start, end, .. } | DrawingItem::Line { start, end, .. } => {
            dist_to_segment_sq(p, start, end) <= margin_sq
        }
        DrawingItem::Rectangle { start, end, .. } => {
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);
            let w = max_x - min_x;
            let h = max_y - min_y;
            if w <= 16.0 || h <= 16.0 {
                p.x >= min_x - margin && p.x <= max_x + margin && p.y >= min_y - margin && p.y <= max_y + margin
            } else {
                let p_tl = Point { x: min_x, y: min_y };
                let p_tr = Point { x: max_x, y: min_y };
                let p_br = Point { x: max_x, y: max_y };
                let p_bl = Point { x: min_x, y: max_y };
                dist_to_segment_sq(p, &p_tl, &p_tr) <= margin_sq
                    || dist_to_segment_sq(p, &p_tr, &p_br) <= margin_sq
                    || dist_to_segment_sq(p, &p_br, &p_bl) <= margin_sq
                    || dist_to_segment_sq(p, &p_bl, &p_tl) <= margin_sq
            }
        }
        DrawingItem::Circle { start, end, .. } => {
            let cx = (start.x + end.x) / 2.0;
            let cy = (start.y + end.y) / 2.0;
            let rx = ((start.x - end.x).abs() / 2.0).max(0.1);
            let ry = ((start.y - end.y).abs() / 2.0).max(0.1);
            let min_r = rx.min(ry);
            if min_r <= 12.0 {
                ((p.x - cx) / rx).powi(2) + ((p.y - cy) / ry).powi(2) <= 1.2
            } else {
                let norm_dist = (((p.x - cx) / rx).powi(2) + ((p.y - cy) / ry).powi(2)).sqrt();
                let dist_from_edge = (norm_dist - 1.0).abs() * min_r;
                dist_from_edge <= margin
            }
        }
        DrawingItem::Blur { start, end } => {
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);
            p.x >= min_x - 4.0 && p.x <= max_x + 4.0 && p.y >= min_y - 4.0 && p.y <= max_y + 4.0
        }
        DrawingItem::Freehand { points, width, .. } => {
            let r_sq = ((width / 2.0).max(margin)).powi(2);
            if points.len() == 1 {
                (p.x - points[0].x).powi(2) + (p.y - points[0].y).powi(2) <= r_sq
            } else {
                points.windows(2).any(|w| dist_to_segment_sq(p, &w[0], &w[1]) <= r_sq)
            }
        }
    }
}

pub fn get_item_bounds(item: &DrawingItem) -> (f64, f64, f64, f64) {
    match item {
        DrawingItem::Blur { start, end }
        | DrawingItem::Arrow { start, end, .. }
        | DrawingItem::Line { start, end, .. }
        | DrawingItem::Rectangle { start, end, .. }
        | DrawingItem::Circle { start, end, .. } => {
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);
            (min_x, min_y, (max_x - min_x).max(1.0), (max_y - min_y).max(1.0))
        }
        DrawingItem::Freehand { points, .. } => {
            if points.is_empty() {
                return (0.0, 0.0, 1.0, 1.0);
            }
            let mut min_x = points[0].x;
            let mut max_x = points[0].x;
            let mut min_y = points[0].y;
            let mut max_y = points[0].y;
            for p in points.iter().skip(1) {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
            (min_x, min_y, (max_x - min_x).max(1.0), (max_y - min_y).max(1.0))
        }
    }
}

pub fn snap_crop_ratio(
    mut min_x: f64,
    mut min_y: f64,
    mut max_x: f64,
    mut max_y: f64,
    handle: CropHandle,
    total_w: f64,
    total_h: f64,
) -> (f64, f64, f64, f64, bool) {
    let cur_w = max_x - min_x;
    let cur_h = max_y - min_y;
    if cur_w < 20.0 || cur_h < 20.0 {
        return (min_x, min_y, max_x, max_y, false);
    }
    let cur_ratio = cur_w / cur_h;

    let mut best_ratio = COMMON_ASPECT_RATIOS[0].1;
    let mut min_diff = f64::MAX;

    for &(_, target_ratio) in COMMON_ASPECT_RATIOS {
        let diff = (cur_ratio - target_ratio).abs();
        if diff < min_diff {
            min_diff = diff;
            best_ratio = target_ratio;
        }
    }

    let expected_w = cur_h * best_ratio;
    let expected_h = cur_w / best_ratio;

    match handle {
        CropHandle::Right | CropHandle::BottomRight | CropHandle::TopRight => {
            let snapped_w = expected_w.min(total_w - min_x);
            max_x = min_x + snapped_w;
        }
        CropHandle::Left | CropHandle::BottomLeft | CropHandle::TopLeft => {
            let snapped_w = expected_w.min(max_x);
            min_x = max_x - snapped_w;
        }
        CropHandle::Bottom => {
            let snapped_h = expected_h.min(total_h - min_y);
            max_y = min_y + snapped_h;
        }
        CropHandle::Top => {
            let snapped_h = expected_h.min(max_y);
            min_y = max_y - snapped_h;
        }
    }
    (min_x, min_y, max_x, max_y, true)
}

pub fn calculate_aspect_ratio_str(w: f64, h: f64) -> String {
    let w_u = w.round() as u32;
    let h_u = h.round() as u32;
    if w_u == 0 || h_u == 0 {
        return String::new();
    }
    let ratio = w / h;
    for &((rw, rh), val) in COMMON_ASPECT_RATIOS {
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
    if rw <= 16 && rh <= 16 {
        format!("{} × {} · {}:{}", w_u, h_u, rw, rh)
    } else {
        format!("{} × {}", w_u, h_u)
    }
}
