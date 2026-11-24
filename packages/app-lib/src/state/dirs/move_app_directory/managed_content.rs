use crate::ErrorKind;
use crate::state::content_store;
use crate::state::content_store::{
    FileStorageKind, content_file_path, validate_instance_path,
    validate_parent_directories, validate_relative,
};
use crate::state::dirs::{
    INSTANCES_FOLDER_NAME, LEGACY_INSTANCES_FOLDER_NAME,
    OPTIFINE_INSTANCES_FOLDER_NAME,
};
use crate::state::instances::adapters::sqlite::{content_rows, instance_rows};
use anyhow::bail;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use tokio::fs;

const CHECKPOINT_KEY: &str = "store_directory_move_storage";

pub(super) struct ManagedContentMove {
    pub source: PathBuf,
    destination: PathBuf,
    stored_path: PathBuf,
    binding: MovedFileBinding,
}

#[derive(Serialize, Deserialize)]
struct MovedFileBinding {
    file_id: String,
    sha512: String,
    storage_kind: FileStorageKind,
}

pub(super) async fn prepare(
    pool: &SqlitePool,
    from: &Path,
    to: &Path,
) -> crate::Result<Vec<ManagedContentMove>> {
    let mut moves = Vec::new();
    // shortcut: upgrades from 0.0.6 still have the old folder; remove in 0.0.10.
    let source_folder =
        if fs::try_exists(&from.join(INSTANCES_FOLDER_NAME)).await? {
            INSTANCES_FOLDER_NAME
        } else {
            LEGACY_INSTANCES_FOLDER_NAME
        };
    for instance in instance_rows::list_instances(pool).await? {
        validate_instance_path(&instance.path)?;
        let (
            instance_source_folder,
            instance_destination_folder,
            instance_relative_path,
        ) = if let Some(path) = instance.path.strip_prefix("@optifine/") {
            (
                OPTIFINE_INSTANCES_FOLDER_NAME,
                OPTIFINE_INSTANCES_FOLDER_NAME,
                path,
            )
        } else {
            (source_folder, INSTANCES_FOLDER_NAME, instance.path.as_str())
        };
        let files =
            content_rows::get_instance_files(&instance.id, pool).await?;
        for binding in
            content_store::instance_storage(pool, &instance.id).await?
        {
            let file = files
                .iter()
                .find(|file| file.id == binding.file_id)
                .ok_or(ErrorKind::InputError(
                    "managed content has no instance file record".to_string(),
                ))?;
            let relative_path = content_file_path(file);
            validate_relative(&relative_path)?;
            let Some(stored) =
                content_store::find_file(pool, &binding.blob_sha512).await?
            else {
                bail!(ErrorKind::InputError(
                    "managed content has no stored file record".to_string()
                ));
            };
            let stored_path =
                content_store::object_relative_path(&stored.sha512)?;
            moves.push(ManagedContentMove {
                source: from
                    .join(instance_source_folder)
                    .join(instance_relative_path)
                    .join(&relative_path),
                destination: to
                    .join(instance_destination_folder)
                    .join(instance_relative_path)
                    .join(relative_path),
                stored_path: to.join("store/content").join(stored_path),
                binding: MovedFileBinding {
                    file_id: binding.file_id,
                    sha512: binding.blob_sha512,
                    storage_kind: binding.storage_kind,
                },
            });
        }
    }
    Ok(moves)
}

pub(super) async fn checkpoint(
    pool: &SqlitePool,
    files: Vec<ManagedContentMove>,
    from: &Path,
    to: &Path,
) -> crate::Result<Vec<(PathBuf, PathBuf, u64)>> {
    let mut copies = Vec::new();
    let mut bindings = Vec::new();
    for mut file in files {
        validate_parent_directories(from, &file.source).await?;
        validate_parent_directories(to, &file.destination).await?;
        validate_parent_directories(to, &file.stored_path).await?;
        let metadata = match fs::symlink_metadata(&file.source).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if !metadata.is_file() {
            bail!(ErrorKind::InputError(
                "managed content was replaced by a directory or symbolic link"
                    .to_string(),
            ));
        }
        let parent = file.destination.parent().ok_or(ErrorKind::InputError(
            "invalid content destination".to_string(),
        ))?;
        fs::create_dir_all(parent).await?;
        file.binding.storage_kind = FileStorageKind::Copy;
        bindings.push(file.binding);
        copies.push((file.source, file.destination, metadata.len()));
    }
    content_store::set_setting(
        pool,
        CHECKPOINT_KEY,
        &serde_json::to_string(&bindings)?,
    )
    .await?;
    Ok(copies)
}

/// Records how files are stored on the destination filesystem after the app switches to it.
/// Until then, the records must describe the original files so cancelling the move leaves
/// storage accounting and recovery consistent with the directory the app still uses.
pub(super) async fn commit(pool: &SqlitePool) -> crate::Result<()> {
    let Some(checkpoint) = content_store::setting(pool, CHECKPOINT_KEY)
        .await?
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    let bindings: Vec<MovedFileBinding> = serde_json::from_str(&checkpoint)?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
    for binding in bindings {
        content_store::set_file_storage(
            &mut tx,
            &binding.file_id,
            &binding.sha512,
            binding.storage_kind,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn clear_checkpoint(pool: &SqlitePool) -> crate::Result<()> {
    content_store::set_setting(pool, CHECKPOINT_KEY, "").await
}
