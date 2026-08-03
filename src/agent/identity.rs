use std::path::Path;

use super::{catalog::AgentCatalog, AgentKind};

pub fn identify_name(value: &str) -> Option<AgentKind> {
    AgentCatalog::identify_alias(&normalized_basename(value))
}

/// Identify direct binaries and common JS/Python package wrappers without
/// treating arbitrary `-c`/`-e` source text as a process identity.
pub fn identify_process(comm: &str, argv: &[String]) -> Option<AgentKind> {
    if let Some(kind) = identify_name(comm) {
        return Some(kind);
    }
    if let Some(kind) = argv.first().and_then(|arg| identify_name(arg)) {
        return Some(kind);
    }

    if !is_package_runtime(comm) {
        return None;
    }
    if argv
        .get(1)
        .is_some_and(|arg| matches!(arg.as_str(), "-c" | "-e" | "--eval"))
    {
        return None;
    }

    // Only inspect the structural script/module slot. Scanning every runtime
    // argument would turn `node app.js --prompt /tmp/codex` into a false hit.
    let mut args = argv.iter().skip(1);
    while let Some(arg) = args.next() {
        if matches!(arg.as_str(), "-m" | "--module") {
            return args.next().and_then(|value| identify_structural_arg(value));
        }
        if !arg.starts_with('-') {
            return identify_structural_arg(arg);
        }
    }
    None
}

fn identify_structural_arg(value: &str) -> Option<AgentKind> {
    identify_name(value).or_else(|| {
        let path = value.to_ascii_lowercase().replace('\\', "/");
        AgentCatalog::identify_package(&path)
    })
}

fn is_package_runtime(comm: &str) -> bool {
    matches!(
        normalized_basename(comm).as_str(),
        "node" | "nodejs" | "bun" | "deno" | "python" | "python3" | "uv"
    )
}

fn normalized_basename(value: &str) -> String {
    let normalized = value.trim().replace('\\', "/");
    let basename = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&normalized)
        .to_ascii_lowercase();
    let mut name = basename
        .strip_suffix(".exe")
        .or_else(|| basename.strip_suffix(".cmd"))
        .or_else(|| basename.strip_suffix(".ps1"))
        .unwrap_or(&basename)
        .trim_start_matches('.')
        .to_string();
    for suffix in ["-wrapped", "_wrapped"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            name = stripped.to_string();
        }
    }
    name
}
