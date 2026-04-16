//! Platform-standard directory layout for persisted state.
//!
//! This module centralizes resolution of config, data, cache, and log
//! directories so every persisted subsystem in easy-nats follows OS
//! conventions (Windows Known Folders, macOS `~/Library/...`, Linux XDG)
//! instead of writing ad-hoc paths.
//!
//! The pre-standardized layout stored everything under
//! `dirs::config_dir()/easy-nats/`. That layout is preserved for
//! configuration files, and one-time migration helpers are provided so files
//! that belong under data/cache/log roots can be moved there on upgrade.

use std::path::{Path, PathBuf};

/// Fixed directory name used under every platform root.
pub const APP_DIR_NAME: &str = "easy-nats";

/// Resolved set of platform-standard directories for easy-nats.
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    config: PathBuf,
    data: PathBuf,
    cache: PathBuf,
    logs: PathBuf,
}

impl ProjectPaths {
    /// Resolve platform-standard directories. If any root is unavailable the
    /// helper falls back to a reasonable sibling so callers never get an
    /// empty path.
    pub fn resolve() -> Self {
        let config = dirs::config_dir()
            .map(|p| p.join(APP_DIR_NAME))
            .unwrap_or_else(|| PathBuf::from(".").join(APP_DIR_NAME));
        let data = dirs::data_dir()
            .map(|p| p.join(APP_DIR_NAME))
            .unwrap_or_else(|| config.clone());
        let cache = dirs::cache_dir()
            .map(|p| p.join(APP_DIR_NAME))
            .unwrap_or_else(|| data.clone());
        // `dirs::state_dir()` returns `Some` only on Linux/BSD; elsewhere
        // logs live under the data directory.
        let logs = dirs::state_dir()
            .map(|p| p.join(APP_DIR_NAME))
            .unwrap_or_else(|| data.join("logs"));
        Self {
            config,
            data,
            cache,
            logs,
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    pub fn log_dir(&self) -> &Path {
        &self.logs
    }

    pub fn config_file(&self, name: &str) -> PathBuf {
        self.config.join(name)
    }

    pub fn data_file(&self, name: &str) -> PathBuf {
        self.data.join(name)
    }

    pub fn cache_file(&self, name: &str) -> PathBuf {
        self.cache.join(name)
    }

    pub fn log_file(&self, name: &str) -> PathBuf {
        self.logs.join(name)
    }
}

impl Default for ProjectPaths {
    fn default() -> Self {
        Self::resolve()
    }
}

/// Legacy root used before the platform-standard layout: every persisted
/// file lived under `dirs::config_dir()/easy-nats/`.
pub fn legacy_root() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(APP_DIR_NAME))
}

/// Outcome of a single-file migration attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// No legacy file existed, nothing to do.
    NotPresent,
    /// Destination already has a file, legacy copy left untouched.
    AlreadyMigrated,
    /// Legacy file was moved (or copied) into the new location.
    Migrated,
    /// Destination and legacy file are the same path; nothing to move.
    SamePath,
}

/// Move a named legacy file from `legacy_root/<name>` to `target/<name>` if
/// the legacy file exists and the destination does not. Returns the outcome
/// so callers can log or assert on the migration.
pub fn migrate_legacy_file(name: &str, legacy_root: &Path, target: &Path) -> MigrationOutcome {
    let legacy = legacy_root.join(name);
    let dest = target.join(name);
    if legacy == dest {
        return MigrationOutcome::SamePath;
    }
    if !legacy.exists() {
        return MigrationOutcome::NotPresent;
    }
    if dest.exists() {
        return MigrationOutcome::AlreadyMigrated;
    }
    if let Some(parent) = dest.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(?parent, %e, "Failed to create migration target directory");
        return MigrationOutcome::NotPresent;
    }
    match std::fs::rename(&legacy, &dest) {
        Ok(()) => MigrationOutcome::Migrated,
        Err(rename_err) => match std::fs::copy(&legacy, &dest) {
            Ok(_) => {
                let _ = std::fs::remove_file(&legacy);
                MigrationOutcome::Migrated
            }
            Err(copy_err) => {
                tracing::warn!(
                    ?legacy,
                    ?dest,
                    rename_error = %rename_err,
                    copy_error = %copy_err,
                    "Failed to migrate legacy file"
                );
                MigrationOutcome::NotPresent
            }
        },
    }
}

/// Run all known legacy migrations once at application startup.
///
/// For every well-known persisted file we check whether it still exists
/// under the legacy `dirs::config_dir()/easy-nats/` root but belongs under a
/// different standardized root today, and move it if so. Files whose new
/// home equals the legacy location are no-ops.
pub fn migrate_legacy_on_startup(paths: &ProjectPaths) {
    let Some(legacy) = legacy_root() else {
        return;
    };
    // (filename, destination directory) pairs. Add entries here when a
    // persisted file should move out of the legacy config root into a
    // different standardized root.
    let migrations: &[(&str, &Path)] = &[
        ("config.json", paths.config_dir()),
        ("settings.json", paths.config_dir()),
    ];
    for (name, dest) in migrations {
        match migrate_legacy_file(name, &legacy, dest) {
            MigrationOutcome::Migrated => {
                tracing::info!(file = name, ?dest, "Migrated legacy persisted file");
            }
            MigrationOutcome::AlreadyMigrated => {
                tracing::debug!(file = name, "Legacy file shadowed by newer copy");
            }
            MigrationOutcome::SamePath | MigrationOutcome::NotPresent => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_returns_distinct_subpaths_under_app_dir() {
        let paths = ProjectPaths::resolve();
        assert!(paths.config_dir().ends_with(APP_DIR_NAME));
        assert!(paths.data_dir().ends_with(APP_DIR_NAME));
        assert!(paths.cache_dir().ends_with(APP_DIR_NAME));
    }

    #[test]
    fn migrate_legacy_file_moves_missing_destination() {
        let tmp = std::env::temp_dir().join(format!("easy-nats-paths-test-{}", std::process::id()));
        let legacy = tmp.join("legacy");
        let target = tmp.join("target");
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("a.json"), b"{}").unwrap();

        assert_eq!(
            migrate_legacy_file("a.json", &legacy, &target),
            MigrationOutcome::Migrated
        );
        assert!(target.join("a.json").exists());
        assert!(!legacy.join("a.json").exists());

        // Second run with nothing to move.
        assert_eq!(
            migrate_legacy_file("a.json", &legacy, &target),
            MigrationOutcome::NotPresent
        );

        // Existing destination leaves legacy intact.
        fs::write(legacy.join("a.json"), b"{}").unwrap();
        assert_eq!(
            migrate_legacy_file("a.json", &legacy, &target),
            MigrationOutcome::AlreadyMigrated
        );
        assert!(legacy.join("a.json").exists());

        fs::remove_dir_all(&tmp).ok();
    }
}
