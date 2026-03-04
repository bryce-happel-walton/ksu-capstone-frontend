use anyhow::{Context, Result};
use std::{env, fs};
use std::{path::PathBuf, process::Command};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=../client/index.html");
    println!("cargo:rerun-if-changed=../client/resources");
    println!("cargo:rerun-if-changed=../client/src");
    println!("cargo:rerun-if-changed=../client/Cargo.toml");
    println!("cargo:rerun-if-changed=../client/build.rs");

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("server must be inside workspace")?
        .to_path_buf();

    let client_crate_dir = workspace_root.join("client");

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let staged_web_dir = out_dir.join("web");
    let staged_pkg_dir = staged_web_dir.join("pkg");

    if staged_web_dir.exists() {
        let _ = fs::remove_dir_all(&staged_web_dir);
    }
    fs::create_dir_all(&staged_pkg_dir)?;

    let app_html = client_crate_dir.join("index.html");
    fs::copy(&app_html, staged_web_dir.join("index.html"))
        .with_context(|| format!("copy {:?}", app_html))?;

    let isolated_target_dir = out_dir.join("wasm_target");

    let mut cmd = Command::new("wasm-pack");
    cmd.current_dir(&workspace_root).args([
        "build",
        client_crate_dir.to_string_lossy().as_ref(),
        "--release",
        "--target",
        "web",
        "--out-dir",
        staged_pkg_dir.to_string_lossy().as_ref(),
    ]);

    cmd.env("CARGO_TARGET_DIR", &isolated_target_dir);

    for k in [
        "CARGO_BUILD_TARGET",
        "TARGET",
        "HOST",
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
    ] {
        cmd.env_remove(k);
    }

    for (k, _) in env::vars() {
        if k.starts_with("CARGO_FEATURE_") {
            cmd.env_remove(&k);
        }
        if k.starts_with("CARGO_CFG_") {
            cmd.env_remove(&k);
        }
    }

    let status = cmd.status().context("failed to execute wasm-pack")?;
    if !status.success() {
        anyhow::bail!("wasm-pack build failed");
    }

    Ok(())
}
