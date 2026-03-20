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
    let main_window = MainWindow::new().unwrap();

    handle_test_data(main_window.as_weak());
    handle_image_stream(main_window.as_weak());
    handle_window(main_window.as_weak());

    main_window.show().unwrap();

    slint::run_event_loop().unwrap();
}

fn handle_test_data(app_window: Weak<MainWindow>) {
    let window = web_sys::window().unwrap();
    let test_data_ws = WebSocket::new(&format!(
        "ws://{}:{}/{}",
        shared::SERVER_IP,
        window.location().port().unwrap(),
        shared::SERVER_WS_TEST_DATA_DIR
    ))
    .unwrap();
    test_data_ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    static CURRENT_DATA: LazyLock<Mutex<Vec<Vec<StandardListViewItem>>>> =
        LazyLock::new(|| Mutex::new(Vec::new()));
    set_data_columns(app_window.clone());

    let main_window_weak = app_window.clone();
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
    test_data_ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
}

fn handle_image_stream(app_window: Weak<MainWindow>) {
    let window = web_sys::window().unwrap();
    let test_data_ws = WebSocket::new(&format!(
        "ws://{}:{}/{}",
        shared::SERVER_IP,
        window.location().port().unwrap(),
        shared::SERVER_WS_IMAGE_STREAM_DIR
    ))
    .unwrap();
    test_data_ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let main_window_weak = app_window.clone();
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
        if let Ok(buf) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&buf).to_vec();

            if let Some(rgba) =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
                    .ok()
                    .map(|img| img.to_rgba8())
            {
                let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                );
                if let Some(handle) = main_window_weak.upgrade() {
                    handle
                        .global::<ImageStream>()
                        .set_image(slint::Image::from_rgba8(buf));
                }
            }
        }
    });
    test_data_ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
}

fn handle_window(app_window: Weak<MainWindow>) {
    let window = web_sys::window().unwrap();
    let weak_app_window = app_window.clone();
    let initial_size_callback = Closure::<dyn FnMut(f64)>::once(move |_: f64| {
        update_window_size(weak_app_window);
    });
    window
        .request_animation_frame(initial_size_callback.as_ref().unchecked_ref())
        .unwrap();
    initial_size_callback.forget();

    let main_window_weak = app_window.clone();
    let on_resize = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::UiEvent| {
        update_window_size(main_window_weak.clone());
    });
    window
        .add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())
        .unwrap();
    on_resize.forget();
}

fn update_window_size(app_window: Weak<MainWindow>) {
    let window = web_sys::window().unwrap();
    let w = window.inner_width().unwrap().as_f64().unwrap() as u32;
    let h = window.inner_height().unwrap().as_f64().unwrap() as u32;
    if let Some(handle) = app_window.upgrade() {
        handle.window().set_size(slint::PhysicalSize::new(w, h));
    }
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
