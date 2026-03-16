use std::sync::{LazyLock, Mutex};

use slint::{ModelRc, StandardListViewItem, TableColumn, VecModel, Weak};
use wasm_bindgen::prelude::*;
use web_sys::{
    ErrorEvent, MessageEvent, WebSocket, console,
    wasm_bindgen::{JsCast, prelude::Closure},
};

slint::include_modules!();

#[wasm_bindgen(start)]
pub fn main() {
    let window = web_sys::window().unwrap();
    let port = window.location().port().unwrap();
    let ws = WebSocket::new(&format!("ws://127.0.0.1:{port}/{}", shared::WEB_SOCKET_DIR)).unwrap();

    let main_window = MainWindow::new().unwrap();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        console::log_1(&format!("error event: {e:?}").into());
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    static CURRENT_DATA: LazyLock<Mutex<Vec<Vec<StandardListViewItem>>>> =
        LazyLock::new(|| Mutex::new(Vec::new()));
    set_data_columns(main_window.as_weak());

    let main_window_weak = main_window.as_weak();
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
        if let Some(txt) = event.data().as_string() {
            if let Ok(esp_data) = serde_json::from_str::<shared::EspData>(&txt) {
                if let Some(handle) = main_window_weak.upgrade() {
                    let test_data = handle.global::<TestData>();

                    let mut data = CURRENT_DATA.lock().unwrap();
                    data.push(vec![
                        {
                            let mut hello_view_item = StandardListViewItem::default();
                            hello_view_item.text = esp_data.hello.into();
                            hello_view_item
                        },
                        {
                            let mut beep_view_item = StandardListViewItem::default();
                            beep_view_item.text = format!("{}", esp_data.beep).into();
                            beep_view_item
                        },
                        {
                            let mut boop_view_item = StandardListViewItem::default();
                            boop_view_item.text = format!("{}", esp_data.boop).into();
                            boop_view_item
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

    let onerror = Closure::wrap(Box::new(move |event: web_sys::ErrorEvent| {
        console::log_1(&format!("WS error: {:?}", event.message()).into());
    }) as Box<dyn FnMut(_)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

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
