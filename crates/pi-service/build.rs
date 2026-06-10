//! Minifica el dashboard embebido (`src/static/index.html`) en tiempo de
//! compilación y deja el resultado en `OUT_DIR/index.min.html`, que
//! `api.rs` incluye con `include_str!`.
//!
//! Minifica HTML + CSS inline (lightningcss) + JS inline (minify-js). Si el
//! minificador de JS llegara a romper el script, basta con poner
//! `minify_js: false` aquí: el HTML/CSS se seguirían minificando.

use std::{env, fs, path::Path};

fn main() {
    // Recompilar solo cuando cambie el dashboard.
    println!("cargo:rerun-if-changed=src/static/index.html");

    let src_path = "src/static/index.html";
    let raw =
        fs::read(src_path).unwrap_or_else(|e| panic!("build.rs: no se pudo leer {src_path}: {e}"));

    let cfg = minify_html::Cfg {
        minify_css: true,
        minify_js: true,
        ..minify_html::Cfg::default()
    };
    let minified = minify_html::minify(&raw, &cfg);

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR no definido");
    let dest = Path::new(&out_dir).join("index.min.html");
    fs::write(&dest, &minified)
        .unwrap_or_else(|e| panic!("build.rs: no se pudo escribir {}: {e}", dest.display()));
}
