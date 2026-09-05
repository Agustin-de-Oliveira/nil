use crate::model::Point;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};

pub fn draw_arrow(ctx: &CanvasRenderingContext2d, from: &Point, to: &Point, width: f64) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = dx.hypot(dy);
    if len < 2.0 {
        return;
    }

    let angle = dy.atan2(dx);
    let head_len = (width * 3.5).clamp(14.0, 56.0).min(len);
    let notch_dist = head_len * 0.72;
    let wing_angle = 0.44;

    if len > notch_dist {
        ctx.begin_path();
        ctx.move_to(from.x, from.y);
        ctx.line_to(
            to.x - notch_dist * angle.cos(),
            to.y - notch_dist * angle.sin(),
        );
        ctx.stroke();
    }

    ctx.begin_path();
    ctx.move_to(to.x, to.y);
    ctx.line_to(
        to.x - head_len * (angle - wing_angle).cos(),
        to.y - head_len * (angle - wing_angle).sin(),
    );
    ctx.line_to(
        to.x - notch_dist * angle.cos(),
        to.y - notch_dist * angle.sin(),
    );
    ctx.line_to(
        to.x - head_len * (angle + wing_angle).cos(),
        to.y - head_len * (angle + wing_angle).sin(),
    );
    ctx.close_path();
    ctx.fill();
}

pub fn apply_pixelate(
    ctx: &CanvasRenderingContext2d,
    img: &HtmlImageElement,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) {
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
    let off_canvas: HtmlCanvasElement =
        match doc.create_element("canvas").ok().and_then(|el| el.dyn_into().ok()) {
            Some(c) => c,
            None => return,
        };
    off_canvas.set_width(small_w as u32);
    off_canvas.set_height(small_h as u32);

    let off_ctx = match off_canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
    {
        Some(c) => c,
        None => return,
    };

    off_ctx.set_image_smoothing_enabled(true);
    let _ = off_ctx
        .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            img, x, y, w, h, 0.0, 0.0, small_w, small_h,
        );

    ctx.save();
    ctx.set_image_smoothing_enabled(false);
    let _ = ctx
        .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &off_canvas, 0.0, 0.0, small_w, small_h, x, y, w, h,
        );
    ctx.restore();
}

pub fn simplify_points_rdp(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut dmax = 0.0;
    let mut index = 0;
    let first = points[0];
    let last = points[points.len() - 1];

    let line_len_sq = (last.x - first.x).powi(2) + (last.y - first.y).powi(2);

    for (i, p) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let d = if line_len_sq < 1e-6 {
            (p.x - first.x).hypot(p.y - first.y)
        } else {
            let t = (((p.x - first.x) * (last.x - first.x) + (p.y - first.y) * (last.y - first.y)) / line_len_sq).clamp(0.0, 1.0);
            let proj_x = first.x + t * (last.x - first.x);
            let proj_y = first.y + t * (last.y - first.y);
            (p.x - proj_x).hypot(p.y - proj_y)
        };

        if d > dmax {
            index = i;
            dmax = d;
        }
    }

    if dmax > epsilon {
        let mut left = simplify_points_rdp(&points[..=index], epsilon);
        let right = simplify_points_rdp(&points[index..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![first, last]
    }
}

pub fn compose_canvases(
    bg_canvas: &HtmlCanvasElement,
    draw_canvas: &HtmlCanvasElement,
) -> Option<String> {
    let doc = web_sys::window()?.document()?;
    let off_canvas: HtmlCanvasElement = doc.create_element("canvas").ok()?.dyn_into().ok()?;
    let w = bg_canvas.width();
    let h = bg_canvas.height();
    off_canvas.set_width(w);
    off_canvas.set_height(h);

    let ctx: CanvasRenderingContext2d = off_canvas
        .get_context("2d")
        .ok()??
        .dyn_into()
        .ok()?;

    let _ = ctx.draw_image_with_html_canvas_element(bg_canvas, 0.0, 0.0);
    let _ = ctx.draw_image_with_html_canvas_element(draw_canvas, 0.0, 0.0);
    off_canvas.to_data_url().ok()
}


