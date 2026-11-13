// SPDX-License-Identifier: GPL-3.0-or-later

//! Embeds every `descriptions/*.json` present at build time (the real models; never
//! `descriptions/fake/`) for the completion generator (design spec 6.2): a new model's file
//! needs no CLI code.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by cargo"));
    let dir = manifest.join("../../descriptions");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let mut out = String::from(
        "/// The description files of the real models at build time: (model key, JSON).\n\
         pub static EMBEDDED: &[(&str, &str)] = &[\n",
    );
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
        let key = file
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a UTF-8 file name");
        let path = fs::canonicalize(file).expect("the description file exists");
        out.push_str(&format!(
            "    ({key:?}, include_str!({:?})),\n",
            path.display().to_string()
        ));
    }
    out.push_str("];\n");
    let target =
        PathBuf::from(env::var_os("OUT_DIR").expect("set by cargo")).join("descriptions.rs");
    fs::write(target, out).expect("write descriptions.rs");
}
