//! Theseus directory information
pub(crate) mod move_app_directory;
use crate::ErrorKind;
use crate::state::{Settings, State};
use crate::util::fetch::IoSemaphore;
use anyhow::Context;
use std::path::{Path, PathBuf};
use tokio::fs;

pub const CACHES_FOLDER_NAME: &str = "caches";
pub const LAUNCHER_LOGS_FOLDER_NAME: &str = "launcher_logs";
pub const INSTANCES_FOLDER_NAME: &str = "instances";
// shortcut: 0.0.6 folder name, needed for upgrades until 0.0.10; remove then.
pub const LEGACY_INSTANCES_FOLDER_NAME: &str = "profiles";
pub const METADATA_FOLDER_NAME: &str = "meta";
pub const SYNCED_OPTIONS_FOLDER_NAME: &str = "synced-options";
pub const STORE_FOLDER_NAME: &str = "store";

#[derive(Clone, Debug)]
pub struct DirectoryInfo {
    pub settings_dir: PathBuf, // Base settings directory- app database
    pub config_dir: PathBuf, // Base config directory- instances, minecraft downloads, etc. Changeable as a setting.
    pub app_identifier: String,
}

impl DirectoryInfo {
    pub fn global_handle_if_ready() -> Option<Self> {
        State::get_if_initialized().map(|state| state.directories.clone())
    }

    pub fn get_initial_settings_dir(&self) -> Option<PathBuf> {
        Self::initial_settings_dir_path(&self.app_identifier)
    }

    // Get the settings directory
    // init() is not needed for this function
    pub fn initial_settings_dir_path(app_identifier: &str) -> Option<PathBuf> {
        Self::env_path("THESEUS_CONFIG_DIR")
            .or_else(|| Some(dirs::home_dir()?.join(app_identifier)))
    }

    /// Get all paths needed for Theseus to operate properly
    #[tracing::instrument]
    pub async fn init(
        config_dir: Option<String>,
        app_identifier: &str,
    ) -> crate::Result<Self> {
        let settings_dir = Self::initial_settings_dir_path(app_identifier)
            .ok_or(ErrorKind::FSError(
                "could not find valid settings dir".to_string(),
            ))?;

        fs::create_dir_all(&settings_dir)
            .await
            .context(ErrorKind::FSError(
                "error creating Theseus config directory".to_string(),
            ))?;

        let config_dir =
            config_dir.map_or_else(|| settings_dir.clone(), PathBuf::from);

        Self::rename_legacy_instances_folder(&config_dir).await?;

        Ok(Self {
            settings_dir,
            config_dir,
            app_identifier: app_identifier.to_owned(),
        })
    }

    /// Renames the 0.0.6 `profiles` folder to `instances` on first launch.
    /// shortcut: upgrade path only; remove in 0.0.10.
    async fn rename_legacy_instances_folder(
        config_dir: &Path,
    ) -> crate::Result<()> {
        let legacy = config_dir.join(LEGACY_INSTANCES_FOLDER_NAME);
        let current = config_dir.join(INSTANCES_FOLDER_NAME);

        if !fs::try_exists(&legacy).await? {
            return Ok(());
        }
        if fs::try_exists(&current).await? {
            tracing::warn!(
                "Both '{}' and '{}' exist; keeping both to avoid data loss",
                LEGACY_INSTANCES_FOLDER_NAME,
                INSTANCES_FOLDER_NAME,
            );
            return Ok(());
        }

        fs::rename(&legacy, &current)
            .await
            .context(ErrorKind::FSError(
                "could not rename the legacy instances folder".to_string(),
            ))?;
        tracing::info!(
            "Renamed '{}' to '{}'",
            LEGACY_INSTANCES_FOLDER_NAME,
            INSTANCES_FOLDER_NAME
        );
        Ok(())
    }

    pub fn store_dir(&self) -> PathBuf {
        self.config_dir.join(STORE_FOLDER_NAME)
    }

    pub fn content_store_dir(&self) -> PathBuf {
        self.store_dir().join("content")
    }

    pub fn store_staging_dir(&self) -> PathBuf {
        self.store_dir().join("staging")
    }

    /// Get the Minecraft instance metadata directory
    #[inline]
    pub fn metadata_dir(&self) -> PathBuf {
        self.config_dir.join(METADATA_FOLDER_NAME)
    }

    pub fn install_backups_dir(&self) -> PathBuf {
        self.metadata_dir().join("install_job_backups")
    }

    /// Get the Minecraft java versions metadata directory
    #[inline]
    pub fn java_versions_dir(&self) -> PathBuf {
        self.config_dir.join("java_versions")
    }

    /// Get the Minecraft versions metadata directory
    #[inline]
    pub fn versions_dir(&self) -> PathBuf {
        self.config_dir.join("versions")
    }

    /// Get the metadata directory for a given version
    #[inline]
    pub fn version_dir(&self, version: &str) -> PathBuf {
        self.versions_dir().join(version)
    }

    /// Get the Minecraft libraries metadata directory
    #[inline]
    pub fn libraries_dir(&self) -> PathBuf {
        self.metadata_dir().join("libraries")
    }

    /// Get the Minecraft assets metadata directory
    #[inline]
    pub fn assets_dir(&self) -> PathBuf {
        self.metadata_dir().join("assets")
    }

    /// Get the assets index directory
    #[inline]
    pub fn assets_index_dir(&self) -> PathBuf {
        self.assets_dir().join("indexes")
    }

    /// Get the assets objects directory
    #[inline]
    pub fn objects_dir(&self) -> PathBuf {
        self.assets_dir().join("objects")
    }

    /// Get the directory for a specific object
    #[inline]
    pub fn object_dir(&self, hash: &str) -> PathBuf {
        self.objects_dir().join(&hash[..2]).join(hash)
    }

    /// Get the Minecraft log config's directory
    #[inline]
    pub fn log_configs_dir(&self) -> PathBuf {
        self.metadata_dir().join("log_configs")
    }

    /// Get the Minecraft legacy assets metadata directory
    #[inline]
    pub fn legacy_assets_dir(&self) -> PathBuf {
        self.metadata_dir().join("resources")
    }

    /// Get the Minecraft legacy assets metadata directory
    #[inline]
    pub fn natives_dir(&self) -> PathBuf {
        self.metadata_dir().join("natives")
    }

    /// Get the natives directory for a version of Minecraft
    #[inline]
    pub fn version_natives_dir(&self, version: &str) -> PathBuf {
        self.natives_dir().join(version)
    }

    /// Get the directory containing instance icons
    #[inline]
    pub fn icon_dir(&self) -> PathBuf {
        self.config_dir.join("icons")
    }

    /// Get the instances directory
    #[inline]
    pub fn instances_dir(&self) -> PathBuf {
        self.config_dir.join(INSTANCES_FOLDER_NAME)
    }

    #[inline]
    pub fn synced_options_dir(&self) -> PathBuf {
        self.config_dir.join(SYNCED_OPTIONS_FOLDER_NAME)
    }

    /// Gets the logs dir for a given instance path
    #[inline]
    pub fn instance_logs_dir(&self, instance_path: &str) -> PathBuf {
        self.instances_dir().join(instance_path).join("logs")
    }

    /// Gets the crash reports dir for a given instance path
    #[inline]
    pub fn crash_reports_dir(&self, instance_path: &str) -> PathBuf {
        self.instances_dir()
            .join(instance_path)
            .join("crash-reports")
    }

    #[inline]
    pub fn launcher_logs_dir(&self) -> Option<PathBuf> {
        self.get_initial_settings_dir()
            .map(|d| d.join(LAUNCHER_LOGS_FOLDER_NAME))
    }

    #[inline]
    pub fn launcher_logs_dir_path(app_identifier: &str) -> Option<PathBuf> {
        Self::initial_settings_dir_path(app_identifier)
            .map(|d| d.join(LAUNCHER_LOGS_FOLDER_NAME))
    }

    /// Get the cache directory for Theseus
    #[inline]
    pub fn caches_dir(&self) -> PathBuf {
        self.config_dir.join(CACHES_FOLDER_NAME)
    }

    /// Get path from environment variable
    #[inline]
    fn env_path(name: &str) -> Option<PathBuf> {
        std::env::var_os(name).map(PathBuf::from)
    }

    #[tracing::instrument(skip(settings, pool, _io_semaphore))]
    pub async fn move_launcher_directory(
        settings: &mut Settings,
        pool: &sqlx::SqlitePool,
        _io_semaphore: &IoSemaphore,
        app_identifier: &str,
    ) -> crate::Result<()> {
        Self::try_move_launcher_directory(settings, pool, app_identifier)
            .await
            .context(ErrorKind::DirectoryMoveFailed)
    }

    async fn try_move_launcher_directory(
        settings: &mut Settings,
        pool: &sqlx::SqlitePool,
        app_identifier: &str,
    ) -> crate::Result<()> {
        let initial = DirectoryInfo::initial_settings_dir_path(app_identifier)
            .ok_or(ErrorKind::FSError(
                "could not find the app directory".to_string(),
            ))?;
        let destination = settings
            .custom_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| initial.clone());
        let previous = settings.prev_custom_dir.as_ref().map(PathBuf::from);
        let previous_root = match &previous {
            Some(previous) => Some(fs::canonicalize(previous).await?),
            None => None,
        };
        fs::create_dir_all(&destination).await?;
        let destination_root = fs::canonicalize(&destination).await?;
        let moving = previous_root
            .as_ref()
            .is_some_and(|root| root != &destination_root);
        let pending_move =
            super::content_store::setting(pool, "store_directory_move")
                .await?
                .filter(|checkpoint| !checkpoint.is_empty())
                .map(|checkpoint| {
                    serde_json::from_str::<(PathBuf, PathBuf)>(&checkpoint)
                })
                .transpose()?;
        let settings_root = fs::canonicalize(&initial).await?;
        let mut locked_roots = std::collections::HashSet::from([settings_root]);
        let mut move_locks = Vec::new();
        let pending_roots = pending_move
            .as_ref()
            .into_iter()
            .flat_map(|(from, to)| [from, to]);
        for root in previous
            .iter()
            .chain(std::iter::once(&destination))
            .chain(pending_roots)
        {
            let root = fs::canonicalize(root).await?;
            if locked_roots.insert(root.clone()) {
                move_locks.push(
                    super::content_store::ContentStore::lock_process(&root)
                        .await?,
                );
            }
        }
        if let Some(previous) = &previous
            && moving
        {
            move_app_directory::move_app_directory(
                previous,
                &destination,
                pool,
            )
            .await?;
        }
        if !moving {
            move_app_directory::resume_completed_move(&destination, pool)
                .await?;
        }
        settings.custom_dir = Some(destination.to_string_lossy().into_owned());
        settings.prev_custom_dir.clone_from(&settings.custom_dir);
        settings.update(pool).await?;
        if let Some(previous) = &previous
            && moving
        {
            move_app_directory::finish_app_directory_move(
                previous,
                &destination,
                pool,
            )
            .await?;
        }
        drop(move_locks);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DirectoryInfo, INSTANCES_FOLDER_NAME, LEGACY_INSTANCES_FOLDER_NAME,
    };

    #[tokio::test]
    async fn renames_legacy_instances_folder_preserving_contents() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join(LEGACY_INSTANCES_FOLDER_NAME);
        let instance = legacy.join("my-instance");
        std::fs::create_dir_all(&instance).unwrap();
        std::fs::write(instance.join("mods"), "keep").unwrap();

        DirectoryInfo::rename_legacy_instances_folder(root.path())
            .await
            .unwrap();

        assert!(!legacy.exists());
        assert_eq!(
            std::fs::read_to_string(
                root.path()
                    .join(INSTANCES_FOLDER_NAME)
                    .join("my-instance")
                    .join("mods")
            )
            .unwrap(),
            "keep"
        );
    }

    #[tokio::test]
    async fn keeps_both_folders_when_instances_already_exists() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join(LEGACY_INSTANCES_FOLDER_NAME);
        let current = root.path().join(INSTANCES_FOLDER_NAME);
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("old.txt"), "old").unwrap();
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(current.join("new.txt"), "new").unwrap();

        DirectoryInfo::rename_legacy_instances_folder(root.path())
            .await
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(legacy.join("old.txt")).unwrap(),
            "old"
        );
        assert_eq!(
            std::fs::read_to_string(current.join("new.txt")).unwrap(),
            "new"
        );
    }
}
