//! rust-agent Web Frontend
//!
//! Leptos-based WASM frontend for the agent interface.

mod app;
mod pages;
mod components;
mod api;

pub use app::App;

use wasm_bindgen::prelude::*;

/// WASM entry point
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
