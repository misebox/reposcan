use std::collections::HashSet;
use std::path::Path;

use serde_json::Value;

const INDICATOR_FILES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "requirements.txt",
    "Gemfile",
    "build.zig",
    "deno.json",
    "pom.xml",
    "build.gradle",
    "Makefile",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yaml",
];

const JS_SUBDIRS: &[&str] = &["app", "api", "ui", "web", "client", "server", "packages", "apps"];
const RUST_SUBDIRS: &[&str] = &["api", "app", "server"];
const PY_SUBDIRS: &[&str] = &["api", "engine", "app"];

struct Context {
    indicators: HashSet<String>,
    js_deps: HashSet<String>,
    cargo_text: String,
    py_text: String,
    root_files: Vec<String>,
}

pub fn detect(repo: &Path) -> Vec<String> {
    let ctx = build_context(repo);
    let mut tags = Vec::<&'static str>::new();

    // Languages
    if ctx.indicators.contains("Cargo.toml") || repo.join("api/Cargo.toml").exists() {
        tags.push("rust");
    }
    if ctx.js_deps.contains("typescript") {
        tags.push("typescript");
    }
    if ctx.indicators.contains("package.json")
        && !ctx.js_deps.contains("typescript")
        && !ctx.js_deps.is_empty()
    {
        tags.push("javascript");
    }
    if ctx.indicators.contains("pyproject.toml")
        || ctx.indicators.contains("requirements.txt")
        || repo.join("api/pyproject.toml").exists()
        || repo.join("engine/pyproject.toml").exists()
    {
        tags.push("python");
    }
    if ctx.indicators.contains("go.mod") {
        tags.push("go");
    }
    if has_ext(&ctx.root_files, ".c") || has_ext(&ctx.root_files, ".h") {
        tags.push("c");
    }
    if ctx.indicators.contains("Gemfile") {
        tags.push("ruby");
    }
    if ctx.indicators.contains("build.zig") {
        tags.push("zig");
    }
    if has_ext(&ctx.root_files, ".lua") {
        tags.push("lua");
    }
    if has_ext(&ctx.root_files, ".rkt") {
        tags.push("racket");
    }

    // Frameworks - JS/TS
    if ctx.js_deps.contains("solid-js") {
        tags.push("solid");
    }
    if ctx.js_deps.contains("react") {
        tags.push("react");
    }
    if ctx.js_deps.contains("hono") || ctx.js_deps.contains("honox") {
        tags.push("hono");
    }
    if ctx.js_deps.iter().any(|d| d.starts_with("@nestjs/")) {
        tags.push("nestjs");
    }
    if ctx.js_deps.contains("@solidjs/start") {
        tags.push("solidstart");
    }
    if ctx.js_deps.contains("phaser") {
        tags.push("phaser");
    }
    if ctx.js_deps.contains("three") {
        tags.push("three.js");
    }
    if ctx.js_deps.contains("tailwindcss")
        || ctx.js_deps.iter().any(|d| d.starts_with("@tailwindcss/"))
    {
        tags.push("tailwind");
    }
    if ctx.js_deps.contains("drizzle-orm") {
        tags.push("drizzle");
    }
    if ctx.js_deps.contains("playwright") || ctx.js_deps.contains("@playwright/test") {
        tags.push("playwright");
    }

    // Frameworks - Rust (Cargo.toml grep)
    if ctx.cargo_text.contains("axum") {
        tags.push("axum");
    }
    if ctx.cargo_text.contains("macroquad") {
        tags.push("macroquad");
    }
    if ctx.cargo_text.contains("wasm-bindgen") {
        tags.push("wasm");
    }
    if ctx.cargo_text.contains("bevy") {
        tags.push("bevy");
    }

    // Frameworks - Python
    if ctx.py_text.contains("fastapi") {
        tags.push("fastapi");
    }
    if ctx.py_text.contains("pygame") {
        tags.push("pygame");
    }

    // Infra
    if ctx.indicators.contains("Dockerfile")
        || ctx.indicators.contains("compose.yaml")
        || ctx.indicators.contains("docker-compose.yml")
        || ctx.indicators.contains("docker-compose.yaml")
    {
        tags.push("docker");
    }

    tags.into_iter().map(String::from).collect()
}

fn build_context(repo: &Path) -> Context {
    let mut indicators = HashSet::new();
    for f in INDICATOR_FILES {
        if repo.join(f).exists() {
            indicators.insert((*f).to_string());
        }
    }

    let js_deps = collect_js_deps(repo);
    let cargo_text = collect_cargo_text(repo);
    let py_text = collect_py_text(repo);
    let root_files = list_root_files(repo);

    Context {
        indicators,
        js_deps,
        cargo_text,
        py_text,
        root_files,
    }
}

fn collect_js_deps(repo: &Path) -> HashSet<String> {
    let root = read_pkg_deps(&repo.join("package.json"));
    if !root.is_empty() {
        return root;
    }
    let mut all = HashSet::new();
    for sub in JS_SUBDIRS {
        for d in read_pkg_deps(&repo.join(sub).join("package.json")) {
            all.insert(d);
        }
    }
    all
}

fn read_pkg_deps(path: &Path) -> HashSet<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return HashSet::new();
    };
    let mut out = HashSet::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(Value::Object(m)) = v.get(key) {
            for k in m.keys() {
                out.insert(k.clone());
            }
        }
    }
    out
}

fn collect_cargo_text(repo: &Path) -> String {
    if let Ok(s) = std::fs::read_to_string(repo.join("Cargo.toml")) {
        return s;
    }
    for sub in RUST_SUBDIRS {
        if let Ok(s) = std::fs::read_to_string(repo.join(sub).join("Cargo.toml")) {
            return s;
        }
    }
    String::new()
}

fn collect_py_text(repo: &Path) -> String {
    let candidates = ["pyproject.toml", "requirements.txt"];
    for f in candidates {
        if let Ok(s) = std::fs::read_to_string(repo.join(f)) {
            return s;
        }
    }
    for sub in PY_SUBDIRS {
        for f in candidates {
            if let Ok(s) = std::fs::read_to_string(repo.join(sub).join(f)) {
                return s;
            }
        }
    }
    String::new()
}

fn list_root_files(repo: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(repo) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|e| {
            if e.file_type().ok()?.is_file() {
                Some(e.file_name().to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect()
}

fn has_ext(files: &[String], ext: &str) -> bool {
    files.iter().any(|f| f.ends_with(ext))
}

