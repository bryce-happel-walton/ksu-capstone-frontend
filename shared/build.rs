#[derive(Debug)]
struct AddDerives;

impl bindgen::callbacks::ParseCallbacks for AddDerives {
    fn add_derives(&self, info: &bindgen::callbacks::DeriveInfo<'_>) -> Vec<String> {
        if matches!(info.kind, bindgen::callbacks::TypeKind::Enum) {
            vec![
                "strum::VariantArray".to_string(),
                "strum::EnumIter".to_string(),
                "strum::Display".to_string(),
            ]
        } else {
            vec![]
        }
    }
}

fn main() {
    const SUBMODULE: &str = "ksu-capstone-embedded";
    let headers = vec![
        format!("{SUBMODULE}/shared/shared_lib.h"),
        format!("{SUBMODULE}/managed_components/espressif__esp32-camera/driver/include/sensor.h"),
    ];

    let mut builder = bindgen::Builder::default()
        .derive_debug(true)
        .derive_default(true)
        .derive_eq(true)
        .derive_partialeq(true)
        .layout_tests(false)
        .parse_callbacks(Box::new(AddDerives))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    for header in &headers {
        builder = builder.header(header);
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("Couldn't write bindings");
}
