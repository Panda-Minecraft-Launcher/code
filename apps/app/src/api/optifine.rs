use theseus::install::InstallJobSnapshot;
use theseus::optifine::{
    OptiFineInstallInput, OptiFineVersion, game_versions, install,
    list_versions,
};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("optifine")
        .invoke_handler(tauri::generate_handler![
            optifine_game_versions,
            optifine_versions,
            optifine_install,
        ])
        .build()
}

#[tauri::command]
pub async fn optifine_game_versions() -> Result<Vec<String>, String> {
    game_versions().await.map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn optifine_versions(
    game_version: String,
) -> Result<Vec<OptiFineVersion>, String> {
    list_versions(&game_version)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn optifine_install(
    input: OptiFineInstallInput,
) -> Result<InstallJobSnapshot, String> {
    install(input).await.map_err(|error| error.to_string())
}
