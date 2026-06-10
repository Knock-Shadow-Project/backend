use std::{env, fs, path::Path, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=frontend/src");
    println!("cargo:rerun-if-changed=frontend/index.html");
    println!("cargo:rerun-if-changed=frontend/package.json");
    println!("cargo:rerun-if-changed=frontend/vite.config.ts");

    let frontend_dir = Path::new("frontend");
    let dist_html = frontend_dir.join("dist/index.html");

    // If dist/ is already present (e.g. pre-built inside Docker via an
    // explicit RUN step), skip pnpm entirely. Otherwise run install + build
    // (normal local-dev workflow).
    if !dist_html.exists() {
        let install = Command::new("pnpm")
            .args(["--dir", frontend_dir.to_str().unwrap(), "install", "--frozen-lockfile"])
            .status()
            .unwrap_or_else(|e| panic!("build.rs: failed to spawn pnpm install: {e}"));
        if !install.success() {
            panic!("build.rs: pnpm install failed (exit {install})");
        }

        let build = Command::new("pnpm")
            .args(["--dir", frontend_dir.to_str().unwrap(), "build"])
            .status()
            .unwrap_or_else(|e| panic!("build.rs: failed to spawn pnpm build: {e}"));
        if !build.success() {
            panic!("build.rs: pnpm build failed (exit {build})");
        }
    }

    let html = fs::read(&dist_html).unwrap_or_else(|e| {
        panic!("build.rs: cannot read {}: {e}", dist_html.display())
    });

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest = Path::new(&out_dir).join("index.min.html");
    fs::write(&dest, &html)
        .unwrap_or_else(|e| panic!("build.rs: cannot write {}: {e}", dest.display()));
}
