use crate::api::Result;
use anyhow::anyhow;

#[derive(Default)]
pub struct PendingUpdateData(());

#[tauri::command]
pub fn check_github_update() -> Result<Option<()>> {
    Ok(None)
}

#[tauri::command]
pub fn enqueue_update_for_installation() -> Result<()> {
    updates_are_disabled()
}

fn updates_are_disabled() -> Result<()> {
    Err(anyhow!("updates are disabled in this build").into())
}

#[tauri::command]
pub fn remove_enqueued_update() {}
