mod app;
mod canvas;
mod hit_test;
mod model;
mod ocr;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
