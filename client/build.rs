use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::{env, fs};

static MANIFEST_DIR: LazyLock<PathBuf> =
    LazyLock::new(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
/// path to the directory that contains `main.slint`
static SLINT_PATH: LazyLock<PathBuf> = LazyLock::new(|| MANIFEST_DIR.join("src/ui"));
static RESOURCES_PATH: LazyLock<PathBuf> = LazyLock::new(|| MANIFEST_DIR.join("resources"));

static SLINT_LIBRARY_PATHS: LazyLock<HashMap<String, PathBuf>> = LazyLock::new(|| {
    HashMap::from([
        // resources
        ("images".to_string(), RESOURCES_PATH.join("images/")),
        ("fonts".to_string(), RESOURCES_PATH.join("fonts/")),
    ])
});

fn update_vscode_slint_libpaths() -> Result<()> {
    const SETTING_NAME: &str = "slint.libraryPaths";

    let workspace_root = MANIFEST_DIR
        .parent()
        .context("server must be inside workspace")?
        .to_path_buf();

    let vscode_settings = workspace_root.join(".vscode/settings.json");
    println!(
        "cargo:rerun-if-changed={}",
        vscode_settings.to_str().unwrap()
    );

    if let Ok(settings) = fs::read_to_string(&vscode_settings) {
        if let Ok(mut settings) = json::parse(settings.as_str()) {
            for (key, _) in settings.clone()[SETTING_NAME].entries() {
                if !SLINT_LIBRARY_PATHS.contains_key(key) {
                    settings[SETTING_NAME].remove(key);
                }
            }

            for (key, path) in SLINT_LIBRARY_PATHS.iter() {
                settings[SETTING_NAME][key] = path
                    .strip_prefix(MANIFEST_DIR.as_path())
                    .unwrap()
                    .to_str()
                    .into();
            }

            fs::write(vscode_settings, json::stringify_pretty(settings, 4)).unwrap();
        }
    }

    Ok(())
}

fn main() {
    let _ = update_vscode_slint_libpaths();

    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(LazyLock::force(&SLINT_LIBRARY_PATHS).clone());
    slint_build::compile_with_config(SLINT_PATH.join("main.slint"), config).unwrap();
}
