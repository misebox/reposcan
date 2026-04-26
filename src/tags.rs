use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use serde_json::Value;

const MAX_WALK_FILES: usize = 10_000;
const MAX_WALK_DEPTH: usize = 6;

const PRUNE_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "dist",
    "build",
    ".next",
    ".nuxt",
    "target",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
    ".turbo",
    ".vercel",
    "coverage",
    "out",
];

const JS_SUBDIRS: &[&str] = &[
    "app", "api", "ui", "web", "client", "server", "packages", "apps",
];
const RUST_SUBDIRS: &[&str] = &["api", "app", "server"];
const PY_SUBDIRS: &[&str] = &["api", "engine", "app"];
const GO_SUBDIRS: &[&str] = &["cmd", "api", "app", "server"];

// File extension (lowercase, no dot) → tag.
const EXT_TAGS: &[(&str, &str)] = &[
    ("rs", "rust"),
    ("ts", "typescript"),
    ("tsx", "typescript"),
    ("js", "javascript"),
    ("jsx", "javascript"),
    ("mjs", "javascript"),
    ("cjs", "javascript"),
    ("py", "python"),
    ("pyi", "python"),
    ("ipynb", "jupyter"),
    ("go", "go"),
    ("c", "c"),
    ("h", "c"),
    ("cpp", "cpp"),
    ("cc", "cpp"),
    ("cxx", "cpp"),
    ("hpp", "cpp"),
    ("hh", "cpp"),
    ("hxx", "cpp"),
    ("rb", "ruby"),
    ("zig", "zig"),
    ("lua", "lua"),
    ("rkt", "racket"),
    ("dsp", "faust"),
    ("sh", "shell"),
    ("bash", "shell"),
    ("zsh", "shell"),
    ("fish", "shell"),
    ("ps1", "powershell"),
    ("html", "html"),
    ("htm", "html"),
    ("vue", "vue"),
    ("svelte", "svelte"),
    ("astro", "astro"),
    ("java", "java"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("swift", "swift"),
    ("hs", "haskell"),
    ("ex", "elixir"),
    ("exs", "elixir"),
    ("erl", "erlang"),
    ("elm", "elm"),
    ("cr", "crystal"),
    ("nim", "nim"),
    ("dart", "dart"),
    ("ml", "ocaml"),
    ("mli", "ocaml"),
    ("scala", "scala"),
    ("clj", "clojure"),
    ("cljs", "clojure"),
    ("cljc", "clojure"),
    ("php", "php"),
    ("pl", "perl"),
    ("jl", "julia"),
    ("nix", "nix"),
    ("tf", "terraform"),
    ("hcl", "terraform"),
    ("proto", "protobuf"),
    ("graphql", "graphql"),
    ("gql", "graphql"),
    ("sql", "sql"),
    ("scss", "sass"),
    ("sass", "sass"),
    ("css", "css"),
    ("vim", "vim"),
];

// Basename → tag (path-independent).
const FILE_TAGS: &[(&str, &str)] = &[
    ("Cargo.toml", "rust"),
    ("go.mod", "go"),
    ("Gemfile", "ruby"),
    ("build.zig", "zig"),
    ("Makefile", "make"),
    ("makefile", "make"),
    ("CMakeLists.txt", "cmake"),
    ("meson.build", "meson"),
    ("flake.nix", "nix"),
    ("default.nix", "nix"),
    ("shell.nix", "nix"),
    ("Vagrantfile", "vagrant"),
    ("Chart.yaml", "helm"),
    ("Pulumi.yaml", "pulumi"),
    ("composer.json", "php"),
    ("mix.exs", "elixir"),
    ("Package.swift", "swift"),
    ("pubspec.yaml", "dart"),
    ("shard.yml", "crystal"),
    ("dune-project", "ocaml"),
    ("build.sbt", "scala"),
    ("project.clj", "clojure"),
    ("deps.edn", "clojure"),
    ("Pipfile", "pipenv"),
    ("poetry.lock", "poetry"),
    ("uv.lock", "uv"),
    ("pdm.lock", "pdm"),
    ("pnpm-lock.yaml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("bun.lockb", "bun"),
    ("bun.lock", "bun"),
    ("package-lock.json", "npm"),
    ("deno.json", "deno"),
    ("deno.lock", "deno"),
    (".editorconfig", "editorconfig"),
    ("biome.json", "biome"),
    (".envrc", "direnv"),
    ("Brewfile", "homebrew"),
    ("mise.toml", "mise"),
    (".tool-versions", "asdf"),
    ("Dockerfile", "docker"),
    ("compose.yaml", "docker"),
    ("docker-compose.yml", "docker"),
    ("docker-compose.yaml", "docker"),
    ("Procfile", "heroku"),
    ("netlify.toml", "netlify"),
    ("vercel.json", "vercel"),
    ("wrangler.toml", "cloudflare"),
    ("fly.toml", "fly.io"),
    ("railway.toml", "railway"),
    ("render.yaml", "render"),
    ("requirements.txt", "python"),
    ("pyproject.toml", "python"),
    ("environment.yml", "conda"),
    ("Pipfile.lock", "pipenv"),
    (".gitlab-ci.yml", "gitlab-ci"),
    (".travis.yml", "travis-ci"),
    ("Jenkinsfile", "jenkins"),
    ("renovate.json", "renovate"),
];

// Substring on a relative path (matches anywhere in the repo).
const PATH_SUBSTR_TAGS: &[(&str, &str)] = &[
    (".github/workflows/", "github-actions"),
    (".github/dependabot.yml", "dependabot"),
    (".circleci/config", "circleci"),
    ("k8s/", "kubernetes"),
    ("kubernetes/", "kubernetes"),
    ("helm/", "helm"),
    ("terraform/", "terraform"),
    ("ansible/", "ansible"),
];

// JS exact dep name → tag.
const JS_DEP_TAGS: &[(&str, &str)] = &[
    ("typescript", "typescript"),
    ("solid-js", "solid"),
    ("@solidjs/start", "solidstart"),
    ("react", "react"),
    ("react-dom", "react"),
    ("react-native", "react-native"),
    ("vue", "vue"),
    ("nuxt", "nuxt"),
    ("next", "nextjs"),
    ("astro", "astro"),
    ("svelte", "svelte"),
    ("@sveltejs/kit", "sveltekit"),
    ("preact", "preact"),
    ("alpinejs", "alpine"),
    ("htmx.org", "htmx"),
    ("lit", "lit"),
    ("hono", "hono"),
    ("honox", "hono"),
    ("phaser", "phaser"),
    ("three", "three.js"),
    ("pixi.js", "pixi"),
    ("d3", "d3"),
    ("tailwindcss", "tailwind"),
    ("drizzle-orm", "drizzle"),
    ("prisma", "prisma"),
    ("@prisma/client", "prisma"),
    ("typeorm", "typeorm"),
    ("mongoose", "mongoose"),
    ("sequelize", "sequelize"),
    ("express", "express"),
    ("fastify", "fastify"),
    ("koa", "koa"),
    ("elysia", "elysia"),
    ("zustand", "zustand"),
    ("jotai", "jotai"),
    ("mobx", "mobx"),
    ("redux", "redux"),
    ("@reduxjs/toolkit", "redux"),
    ("zod", "zod"),
    ("playwright", "playwright"),
    ("@playwright/test", "playwright"),
    ("cypress", "cypress"),
    ("jest", "jest"),
    ("vitest", "vitest"),
    ("mocha", "mocha"),
    ("vite", "vite"),
    ("webpack", "webpack"),
    ("rollup", "rollup"),
    ("esbuild", "esbuild"),
    ("electron", "electron"),
    ("expo", "expo"),
    ("framer-motion", "framer-motion"),
    ("styled-components", "styled-components"),
    ("@biomejs/biome", "biome"),
    ("eslint", "eslint"),
    ("prettier", "prettier"),
    ("oxlint", "oxlint"),
];

// JS dep prefix → tag.
const JS_DEP_PREFIX_TAGS: &[(&str, &str)] = &[
    ("@nestjs/", "nestjs"),
    ("@tailwindcss/", "tailwind"),
    ("@angular/", "angular"),
    ("@remix-run/", "remix"),
    ("@builder.io/qwik", "qwik"),
    ("@trpc/", "trpc"),
    ("@mui/", "mui"),
    ("@chakra-ui/", "chakra"),
    ("@radix-ui/", "radix-ui"),
    ("@emotion/", "emotion"),
    ("@tauri-apps/", "tauri"),
    ("@capacitor/", "capacitor"),
    ("@ionic/", "ionic"),
    ("@aws-sdk/", "aws-sdk"),
    ("@google-cloud/", "gcp-sdk"),
];

// Cargo.toml substring → tag.
const CARGO_SUBSTR_TAGS: &[(&str, &str)] = &[
    ("axum", "axum"),
    ("actix-web", "actix"),
    ("rocket", "rocket"),
    ("warp", "warp"),
    ("poem", "poem"),
    ("tide", "tide"),
    ("leptos", "leptos"),
    ("yew", "yew"),
    ("dioxus", "dioxus"),
    ("sycamore", "sycamore"),
    ("tauri", "tauri"),
    ("iced", "iced"),
    ("egui", "egui"),
    ("diesel", "diesel"),
    ("sqlx", "sqlx"),
    ("sea-orm", "sea-orm"),
    ("ratatui", "ratatui"),
    ("crossterm", "crossterm"),
    ("nannou", "nannou"),
    ("ggez", "ggez"),
    ("bevy", "bevy"),
    ("macroquad", "macroquad"),
    ("wasm-bindgen", "wasm"),
    ("wgpu", "wgpu"),
    ("burn", "burn"),
    ("candle", "candle"),
    ("polars", "polars"),
];

// Python pyproject.toml / requirements.txt substring → tag.
const PY_SUBSTR_TAGS: &[(&str, &str)] = &[
    ("fastapi", "fastapi"),
    ("django", "django"),
    ("flask", "flask"),
    ("starlette", "starlette"),
    ("pydantic", "pydantic"),
    ("pygame", "pygame"),
    ("pandas", "pandas"),
    ("numpy", "numpy"),
    ("scipy", "scipy"),
    ("matplotlib", "matplotlib"),
    ("torch", "pytorch"),
    ("tensorflow", "tensorflow"),
    ("jax", "jax"),
    ("transformers", "transformers"),
    ("langchain", "langchain"),
    ("openai", "openai"),
    ("anthropic", "anthropic"),
    ("huggingface", "huggingface"),
    ("sqlalchemy", "sqlalchemy"),
    ("alembic", "alembic"),
    ("celery", "celery"),
    ("requests", "requests"),
    ("httpx", "httpx"),
    ("aiohttp", "aiohttp"),
    ("pytest", "pytest"),
    ("ruff", "ruff"),
    ("streamlit", "streamlit"),
    ("polars", "polars"),
];

// go.mod substring → tag.
const GO_SUBSTR_TAGS: &[(&str, &str)] = &[
    ("gin-gonic/gin", "gin"),
    ("labstack/echo", "echo"),
    ("gofiber/fiber", "fiber"),
    ("go-chi/chi", "chi"),
    ("gorilla/mux", "gorilla"),
    ("jinzhu/gorm", "gorm"),
    ("gorm.io/gorm", "gorm"),
    ("entgo.io/ent", "ent"),
    ("urfave/cli", "urfave-cli"),
    ("spf13/cobra", "cobra"),
    ("spf13/viper", "viper"),
];

struct Context {
    extensions: HashSet<String>,
    paths: Vec<String>,
    basenames: HashSet<String>,
    js_deps: HashSet<String>,
    cargo_text: String,
    py_text: String,
    go_text: String,
}

pub fn detect(repo: &Path) -> Vec<String> {
    let ctx = build_context(repo);
    let mut tags: BTreeSet<&'static str> = BTreeSet::new();

    for (ext, tag) in EXT_TAGS {
        if ctx.extensions.contains(*ext) {
            tags.insert(tag);
        }
    }
    for (file, tag) in FILE_TAGS {
        if ctx.basenames.contains(*file) {
            tags.insert(tag);
        }
    }
    for (sub, tag) in PATH_SUBSTR_TAGS {
        if ctx.paths.iter().any(|p| p.contains(sub)) {
            tags.insert(tag);
        }
    }
    for (dep, tag) in JS_DEP_TAGS {
        if ctx.js_deps.contains(*dep) {
            tags.insert(tag);
        }
    }
    for (prefix, tag) in JS_DEP_PREFIX_TAGS {
        if ctx.js_deps.iter().any(|d| d.starts_with(*prefix)) {
            tags.insert(tag);
        }
    }
    for (substr, tag) in CARGO_SUBSTR_TAGS {
        if ctx.cargo_text.contains(*substr) {
            tags.insert(tag);
        }
    }
    for (substr, tag) in PY_SUBSTR_TAGS {
        if ctx.py_text.contains(*substr) {
            tags.insert(tag);
        }
    }
    for (substr, tag) in GO_SUBSTR_TAGS {
        if ctx.go_text.contains(*substr) {
            tags.insert(tag);
        }
    }

    tags.into_iter().map(String::from).collect()
}

fn build_context(repo: &Path) -> Context {
    let mut extensions = HashSet::new();
    let mut paths = Vec::new();
    let mut basenames = HashSet::new();

    let prune: HashSet<&str> = PRUNE_DIRS.iter().copied().collect();
    let walker = ignore::WalkBuilder::new(repo)
        .max_depth(Some(MAX_WALK_DEPTH))
        .standard_filters(true)
        .hidden(false) // include .github, .envrc etc.
        .filter_entry(move |e| {
            let name = e.file_name().to_string_lossy().to_string();
            !prune.contains(name.as_str())
        })
        .build();

    for (count, result) in walker.enumerate() {
        if count >= MAX_WALK_FILES {
            break;
        }
        let Ok(entry) = result else { continue };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        if let Ok(rel) = path.strip_prefix(repo) {
            paths.push(rel.to_string_lossy().to_string());
        }
        if let Some(name) = path.file_name() {
            basenames.insert(name.to_string_lossy().to_string());
        }
        if let Some(ext) = path.extension() {
            extensions.insert(ext.to_string_lossy().to_lowercase());
        }
    }

    Context {
        extensions,
        paths,
        basenames,
        js_deps: collect_js_deps(repo),
        cargo_text: collect_cargo_text(repo),
        py_text: collect_py_text(repo),
        go_text: collect_go_text(repo),
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
    for key in ["dependencies", "devDependencies", "peerDependencies"] {
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

fn collect_go_text(repo: &Path) -> String {
    if let Ok(s) = std::fs::read_to_string(repo.join("go.mod")) {
        return s;
    }
    for sub in GO_SUBDIRS {
        if let Ok(s) = std::fs::read_to_string(repo.join(sub).join("go.mod")) {
            return s;
        }
    }
    String::new()
}
