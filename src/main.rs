mod app;
mod builder;
mod keys;
mod lists;
mod model;
mod pdf;
mod store;
#[cfg(not(target_arch = "wasm32"))]
mod term;
mod ui;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> { term::run() }

#[cfg(target_arch = "wasm32")]
fn main() { web::run() }
