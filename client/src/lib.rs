#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

slint::include_modules!();

#[cfg_attr(target_family = "wasm", wasm_bindgen(start))]
pub fn main() {
    // Usual application code
    let main_window = MainWindow::new().unwrap();
    main_window.run().unwrap();
}
