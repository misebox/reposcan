use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;
use serde_yaml::Value;

use crate::util::run;

const COMPOSE_CANDIDATES: &[&str] = &["compose.yaml", "docker-compose.yml", "docker-compose.yaml"];

#[derive(Debug, Default)]
pub struct DockerInfo {
    pub has_dockerfile: bool,
    pub compose_file: Option<String>,
    pub compose_ports: Vec<u16>,
    pub compose_running: bool,
}

pub async fn collect(repo: &Path, skip_runtime: bool) -> DockerInfo {
    let mut info = DockerInfo::default();
    info.has_dockerfile = repo.join("Dockerfile").exists();

    info.compose_file = COMPOSE_CANDIDATES
        .iter()
        .find(|f| repo.join(f).exists())
        .map(|s| s.to_string());

    if let Some(name) = &info.compose_file {
        if let Ok(text) = std::fs::read_to_string(repo.join(name)) {
            info.compose_ports = parse_compose_ports(&text);
        }
        if !skip_runtime {
            info.compose_running = compose_running(repo).await;
        }
    }

    info
}

#[derive(Deserialize)]
struct ComposeFile {
    #[serde(default)]
    services: serde_yaml::Mapping,
}

pub fn parse_compose_ports(yaml_text: &str) -> Vec<u16> {
    let parsed: ComposeFile = match serde_yaml::from_str(yaml_text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (_svc, body) in parsed.services {
        let ports = match body.get("ports") {
            Some(Value::Sequence(seq)) => seq,
            _ => continue,
        };
        for entry in ports {
            if let Some(p) = host_port_from_entry(entry) {
                if seen.insert(p) {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

fn host_port_from_entry(entry: &Value) -> Option<u16> {
    match entry {
        Value::String(s) => parse_port_string(s),
        Value::Number(n) => n.as_u64().and_then(|v| u16::try_from(v).ok()),
        Value::Mapping(map) => {
            // long syntax: { target, published, protocol }
            if let Some(p) = map.get(Value::String("published".into())) {
                return port_from_value(p);
            }
            if let Some(p) = map.get(Value::String("target".into())) {
                return port_from_value(p);
            }
            None
        }
        _ => None,
    }
}

fn port_from_value(v: &Value) -> Option<u16> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|x| u16::try_from(x).ok()),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn parse_port_string(s: &str) -> Option<u16> {
    // Forms: "8080", "8080:80", "127.0.0.1:8080:80", "8080:80/tcp"
    let s = s.split('/').next().unwrap_or(s);
    let parts: Vec<&str> = s.split(':').collect();
    let host_part = match parts.len() {
        1 => parts[0],
        2 => parts[0],     // host:container
        3 => parts[1],     // ip:host:container
        _ => return None,
    };
    host_part.parse().ok()
}

async fn compose_running(repo: &Path) -> bool {
    let Some(out) = run(repo, "docker", &["compose", "ps", "--format", "json"]).await else {
        return false;
    };
    if !out.ok() {
        return false;
    }
    // `docker compose ps --format json` may emit either a JSON array or NDJSON.
    let trimmed = out.stdout.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return false;
    }
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<serde_json::Value>>(trimmed)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
    }
    trimmed.lines().any(|l| !l.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_short_and_long_syntax() {
        let yaml = r#"
services:
  web:
    image: nginx
    ports:
      - "8080:80"
      - "127.0.0.1:9000:9000"
      - "3000"
      - "5432:5432/tcp"
  api:
    image: api
    ports:
      - target: 4000
        published: 4001
        protocol: tcp
      - target: 5000
"#;
        let mut ports = parse_compose_ports(yaml);
        ports.sort();
        assert_eq!(ports, vec![3000, 4001, 5000, 5432, 8080, 9000]);
    }

    #[test]
    fn empty_or_invalid_yaml_returns_empty() {
        assert!(parse_compose_ports("").is_empty());
        assert!(parse_compose_ports("services: {}").is_empty());
    }
}
