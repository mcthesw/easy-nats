use prost_reflect::DescriptorPool;
use std::path::{Path, PathBuf};

/// Recursively collect all `.proto` files under `dir`.
fn collect_proto_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_recursive(dir, &mut files).map_err(|e| format!("Failed to scan directory: {e}"))?;
    if files.is_empty() {
        return Err(format!("No .proto files found in {}", dir.display()));
    }
    Ok(files)
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            out.push(path);
        }
    }
    Ok(())
}

/// Compile all `.proto` files in `dir` into a `DescriptorPool` and return
/// the list of user-defined message type names (fully-qualified).
pub(crate) fn compile_schema_dir(dir: &Path) -> Result<(DescriptorPool, Vec<String>), String> {
    let proto_files = collect_proto_files(dir)?;

    // Build paths relative to the include root
    let relative: Vec<PathBuf> = proto_files
        .iter()
        .filter_map(|p| p.strip_prefix(dir).ok().map(|r| r.to_path_buf()))
        .collect();

    let fds = protox::compile(&relative, &[dir.to_path_buf()])
        .map_err(|e| format!("Proto compilation failed: {e}"))?;

    let pool = DescriptorPool::from_file_descriptor_set(fds)
        .map_err(|e| format!("Failed to build descriptor pool: {e}"))?;

    let types = collect_message_types(&pool);
    Ok((pool, types))
}

/// Extract all user-defined message types from the pool, filtering out
/// synthetic types like map entry messages.
fn collect_message_types(pool: &DescriptorPool) -> Vec<String> {
    let mut types: Vec<String> = pool
        .all_messages()
        .filter(|msg| !msg.is_map_entry())
        .map(|msg| msg.full_name().to_string())
        .collect();
    types.sort();
    types
}
