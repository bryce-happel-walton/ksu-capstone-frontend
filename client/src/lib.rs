use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use slint::{ModelRc, SharedString, StandardListViewItem, TableColumn, VecModel, Weak};
use strum::VariantArray;
use wasm_bindgen::prelude::*;
use web_sys::{
    MessageEvent, WebSocket,
    wasm_bindgen::{JsCast, prelude::Closure},
};

slint::include_modules!();

#[wasm_bindgen(start)]
pub fn main() {
    let main_window = MainWindow::new().unwrap();

    handle_helpers(main_window.as_weak());
    handle_radar_data(main_window.as_weak());
    handle_image_stream(main_window.as_weak());
    handle_input(main_window.as_weak());
    handle_window(main_window.as_weak());

    main_window.show().unwrap();

    slint::run_event_loop().unwrap();
}

fn handle_radar_data(app_window: Weak<MainWindow>) {
    let radar_data_ws = WebSocket::new(&format!(
        "ws://{}/{}",
        shared::ESP_IP,
        shared::cstr_to_str(shared::RADAR_DATA_URI)
    ))
    .unwrap();
    radar_data_ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    static CURRENT_DATA: LazyLock<Mutex<Vec<Vec<StandardListViewItem>>>> =
        LazyLock::new(|| Mutex::new(Vec::new()));
    set_data_columns(app_window.clone());

    let main_window_weak = app_window.clone();
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
        if let Ok(buf) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(&buf).to_vec();
            web_sys::console::log_1(&format!("[radar_data] received {} bytes", bytes.len()).into());

            if let Some(esp_data) = shared::RadarPayload::from_bytes(&bytes) {
                web_sys::console::log_1(&format!("[radar_data] {esp_data:?}").into());

                let main_window_weak = main_window_weak.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(handle) = main_window_weak.upgrade() {
                        let radar_data = handle.global::<DisplayRadarData>();

                        let mut data = CURRENT_DATA.lock().unwrap();
                        data.clear();

                        let make_item = |text: String| {
                            let mut item = StandardListViewItem::default();
                            item.text = text.into();
                            item
                        };

                        for target in &esp_data.targets[..esp_data.count as usize] {
                            data.push(vec![
                                make_item(format!("{}", target.angle)),
                                make_item(format!("{} m", target.distance)),
                                make_item(if target.direction == 0 {
                                    "Away".into()
                                } else {
                                    "Towards".into()
                                }),
                                make_item(format!("{} km/h", target.speed)),
                                make_item(format!("{}", target.snr)),
                            ]);
                        }

                        let model: ModelRc<ModelRc<StandardListViewItem>> =
                            ModelRc::new(VecModel::from(
                                data.iter()
                                    .map(|row| {
                                        ModelRc::new(VecModel::from(row.clone()))
                                            as ModelRc<StandardListViewItem>
                                    })
                                    .collect::<Vec<_>>(),
                            ));

                        radar_data.set_test_packets(model);
                    }
                });
            }
        }
    });
    radar_data_ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
}

fn handle_image_stream(app_window: Weak<MainWindow>) {
    static PROCESSING: AtomicBool = AtomicBool::new(false);

    let ws = WebSocket::new(&format!(
        "ws://{}/{}",
        shared::ESP_IP,
        shared::cstr_to_str(shared::IMAGE_STREAM_URI)
    ))
    .unwrap();
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let main_window_weak = app_window.clone();
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
        if PROCESSING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // still processing previous frame, ignore incoming data
            return;
        }

        let Ok(buf) = event.data().dyn_into::<js_sys::ArrayBuffer>() else {
            PROCESSING.store(false, Ordering::SeqCst);
            return;
        };

        let uint8 = js_sys::Uint8Array::new(&buf);
        let parts = js_sys::Array::new();
        parts.push(&uint8);
        let blob_opts = web_sys::BlobPropertyBag::new();
        blob_opts.set_type("image/jpeg");
        let blob =
            web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &blob_opts).unwrap();

        // make the browser decode the jpeg
        let promise = web_sys::window()
            .unwrap()
            .create_image_bitmap_with_blob(&blob)
            .unwrap();

        let main_window_weak = main_window_weak.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let clear = || PROCESSING.store(false, Ordering::SeqCst);

            let Ok(val) = wasm_bindgen_futures::JsFuture::from(promise).await else {
                clear();
                return;
            };

            let bitmap: web_sys::ImageBitmap = val.unchecked_into();
            let w = bitmap.width();
            let h = bitmap.height();

            let document = web_sys::window().unwrap().document().unwrap();
            let canvas: web_sys::HtmlCanvasElement =
                document.create_element("canvas").unwrap().unchecked_into();
            canvas.set_width(w);
            canvas.set_height(h);
            let ctx: web_sys::CanvasRenderingContext2d =
                canvas.get_context("2d").unwrap().unwrap().unchecked_into();
            ctx.draw_image_with_image_bitmap(&bitmap, 0.0, 0.0).unwrap();

            let Ok(image_data) = ctx.get_image_data(0.0, 0.0, w as f64, h as f64) else {
                clear();
                return;
            };
            let rgba = image_data.data().0;

            let pixel_buf =
                slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&rgba, w, h);
            clear();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(handle) = main_window_weak.upgrade() {
                    handle
                        .global::<ImageStream>()
                        .set_image(slint::Image::from_rgba8(pixel_buf));
                }
            });
        });
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
}

fn handle_input(app_window: Weak<MainWindow>) {
    thread_local! {
        static INPUT_WS: RefCell<Option<WebSocket>> = const { RefCell::new(None) };
    }

    let ws = WebSocket::new(&format!(
        "ws://{}/{}",
        shared::ESP_IP,
        shared::cstr_to_str(shared::WS_INPUT_URI)
    ))
    .unwrap();
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    INPUT_WS.with(|cell| *cell.borrow_mut() = Some(ws));

    if let Some(handle) = app_window.upgrade() {
        let sender = handle.global::<Sender>();

        sender.on_test_send_data(move || {
            INPUT_WS.with(|cell| {
                if let Some(ws) = cell.borrow().as_ref() {
                    if ws.ready_state() == WebSocket::OPEN {
                        if let Some(handle) = app_window.upgrade() {
                            let sender = handle.global::<Sender>();
                            let data = shared::InputData {
                                display_pattern: shared::TestDisplayPattern::from_repr(
                                    sender.get_led_pattern() as u32,
                                )
                                .unwrap_or(shared::TestDisplayPattern::DISPLAY_PATTERN_CENTERS),
                            };
                            web_sys::console::log_1(&format!("{:?}", data).into());
                            ws.send_with_u8_array(&data.to_bytes()).unwrap();
                        }
                    }
                }
            });
        });
    }
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
    let dpr = window.device_pixel_ratio();
    let w = (window.inner_width().unwrap().as_f64().unwrap() * dpr) as u32;
    let h = (window.inner_height().unwrap().as_f64().unwrap() * dpr) as u32;
    if let Some(handle) = app_window.upgrade() {
        handle.window().set_size(slint::PhysicalSize::new(w, h));
    }
}

fn set_data_columns(handle: Weak<MainWindow>) {
    if let Some(handle) = handle.upgrade() {
        let test_data = handle.global::<DisplayRadarData>();

        let cols = vec!["Angle", "Distance", "Direction", "Speed", "SNR"];

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

fn handle_helpers(handle: Weak<MainWindow>) {
    if let Some(handle) = handle.upgrade() {
        let helper = handle.global::<Helper>();

        helper.set_display_patterns(
            shared::TestDisplayPattern::VARIANTS
                .iter()
                .map(|var| format!("{var}").into())
                .collect::<Vec<SharedString>>()
                .as_slice()
                .into(),
        );
    }
}
