use std::sync::{LazyLock, Mutex};

use slint::{ModelRc, StandardListViewItem, TableColumn, VecModel, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{
    MessageEvent, WebSocket,
    wasm_bindgen::{JsCast, prelude::Closure},
};

slint::include_modules!();

#[wasm_bindgen(start)]
pub fn main() {
    let window = web_sys::window().unwrap();
    let port = window.location().port().unwrap();
    let ws = WebSocket::new(&format!("ws://127.0.0.1:{port}/{}", shared::WEB_SOCKET_DIR)).unwrap();
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let main_window = MainWindow::new().unwrap();

    static CURRENT_DATA: LazyLock<Mutex<Vec<Vec<StandardListViewItem>>>> =
        LazyLock::new(|| Mutex::new(Vec::new()));
    set_data_columns(main_window.as_weak());

    let main_window_weak = main_window.as_weak();
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
        if let Ok(buf) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&buf).to_vec();
            if let Some(esp_data) = shared::TestData::from_bytes(&bytes) {
                let hello = shared::TestData::str_from_chars(&esp_data.hello);
                let beep = esp_data.beep;
                let boop = esp_data.boop;

                if let Some(handle) = main_window_weak.upgrade() {
                    let test_data = handle.global::<TestData>();

                    let mut data = CURRENT_DATA.lock().unwrap();
                    data.push(vec![
                        {
                            let mut item = StandardListViewItem::default();
                            item.text = hello.into();
                            item
                        },
                        {
                            let mut item = StandardListViewItem::default();
                            item.text = format!("{}", beep).into();
                            item
                        },
                        {
                            let mut item = StandardListViewItem::default();
                            item.text = format!("{}", boop).into();
                            item
                        },
                    ]);

                    let model: ModelRc<ModelRc<StandardListViewItem>> =
                        ModelRc::new(VecModel::from(
                            data.iter()
                                .map(|row| {
                                    ModelRc::new(VecModel::from(row.clone()))
                                        as ModelRc<StandardListViewItem>
                                })
                                .collect::<Vec<_>>(),
                        ));

                    test_data.set_test_packets(model);
                }
            }
        }
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let inner_width = window.inner_width().unwrap().as_f64().unwrap() as u32;
    let inner_height = window.inner_height().unwrap().as_f64().unwrap() as u32;
    main_window
        .window()
        .set_size(slint::PhysicalSize::new(inner_width, inner_height));

    let main_window_weak = main_window.as_weak();
    let on_resize = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::UiEvent| {
        let window = web_sys::window().unwrap();
        let w = window.inner_width().unwrap().as_f64().unwrap() as u32;
        let h = window.inner_height().unwrap().as_f64().unwrap() as u32;
        if let Some(handle) = main_window_weak.upgrade() {
            handle.window().set_size(slint::PhysicalSize::new(w, h));
        }
    });
    window
        .add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())
        .unwrap();
    on_resize.forget();

    main_window.show().unwrap();
    slint::run_event_loop().unwrap();
}

fn set_data_columns(handle: Weak<MainWindow>) {
    if let Some(handle) = handle.upgrade() {
        let test_data = handle.global::<TestData>();

        let cols = vec!["Hello", "Beep", "Boop"];

        test_data.set_test_packet_columns(
            cols.iter()
                .map(|x| {
                    let mut col = TableColumn::default();
                    col.title = String::from(*x).into();
                    col
                })
                .collect::<Vec<TableColumn>>()
                .as_slice()
                .into(),
        );
    }
}
