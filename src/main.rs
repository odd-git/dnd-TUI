#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
mod app;
mod builder;
mod keys;
mod lists;
mod model;
mod pdf;
mod store;
#[cfg(not(target_arch = "wasm32"))]
mod term;
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // the terminal drawing is unused in the browser
mod ui;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // the web page is its only user
mod view;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> { term::run() }

#[cfg(target_arch = "wasm32")]
fn main() { web::run() }
