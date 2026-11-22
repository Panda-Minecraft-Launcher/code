use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value;

use crate::api::Result;

const CF_BASE_URL: &str = "https://api.curseforge.com/v1";

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("curseforge")
        .invoke_handler(tauri::generate_handler![
            curseforge_status,
            curseforge_request
        ])
        .build()
}

fn api_key() -> Option<String> {
    let runtime = std::env::var("CURSEFORGE_KEY").ok();
    let built = option_env!("CURSEFORGE_KEY").map(str::to_owned);
    let runtime_set = runtime.is_some();
    let built_set = built.is_some();
    let key = runtime.or(built).filter(|key| !key.trim().is_empty());
    tracing::info!(
        runtime = runtime_set,
        built = built_set,
        accepted = key.is_some(),
        "curseforge key lookup"
    );
    key
}

fn validate_request(method: &str, path: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = path.split('/').collect();
    let is_get = method.eq_ignore_ascii_case("GET");
    let is_post = method.eq_ignore_ascii_case("POST");
    let is_id = |value: &&str| value.parse::<u32>().is_ok();

    let allowed = match parts.as_slice() {
        ["mods", "search"] => is_get,
        ["categories"] => is_get,
        ["mods"] => is_post,
        ["mods", "files"] => is_post,
        ["mods", id] if is_id(id) => is_get,
        ["mods", id, "description"] if is_id(id) => is_get,
        ["mods", id, "files"] if is_id(id) => is_get,
        ["mods", id, "files", file_id] if is_id(id) && is_id(file_id) => is_get,
        _ => false,
    };

    if !allowed {
        return Err(anyhow!(
            "unsupported CurseForge request: {method} /{path}"
        ));
    }
    Ok(())
}

fn enforce_mod_scope(
    query: &mut HashMap<String, String>,
    body: &Option<Value>,
) -> anyhow::Result<()> {
    for (key, expected) in [("gameId", 432_u64), ("classId", 6_u64)] {
        let query_ok = query
            .get(key)
            .map(|value| value.parse::<u64>().ok() == Some(expected))
            .unwrap_or(true);
        let body_ok = body
            .as_ref()
            .and_then(|body| body.get(key))
            .map(|value| {
                value.as_u64() == Some(expected)
                    || value
                        .as_str()
                        .and_then(|value| value.parse::<u64>().ok())
                        == Some(expected)
            })
            .unwrap_or(true);
        if !query_ok || !body_ok {
            return Err(anyhow!("CurseForge {key} must be {expected}"));
        }
    }

    query
        .entry("gameId".to_string())
        .or_insert_with(|| "432".to_string());
    query
        .entry("classId".to_string())
        .or_insert_with(|| "6".to_string());
    Ok(())
}

#[tauri::command]
pub async fn curseforge_status() -> bool {
    api_key().is_some()
}

#[tauri::command]
pub async fn curseforge_request(
    method: String,
    path: String,
    query: Option<HashMap<String, String>>,
    body: Option<Value>,
) -> Result<Value> {
    validate_request(&method, &path)?;
    let key = api_key()
        .ok_or_else(|| anyhow!("Waiting for approval from CurseForge"))?;
    let mut query = query.unwrap_or_default();
    enforce_mod_scope(&mut query, &body)?;

    let mut url = reqwest::Url::parse(&format!("{CF_BASE_URL}/{path}"))?;
    url.query_pairs_mut().extend_pairs(query);

    let mut request = reqwest::Client::new()
        .request(method.parse()?, url)
        .header("x-api-key", key);
    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("CurseForge HTTP {status}: {body}").into());
    }
    Ok(response.json::<Value>().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_only_mod_endpoints_with_matching_methods() {
        assert!(validate_request("GET", "mods/search").is_ok());
        assert!(validate_request("GET", "mods/123/files/456").is_ok());
        assert!(validate_request("POST", "mods/files").is_ok());
        assert!(validate_request("GET", "categories").is_ok());
        assert!(validate_request("POST", "mods/search").is_err());
        assert!(validate_request("GET", "modpacks/search").is_err());
        assert!(validate_request("GET", "mods/../search").is_err());
        assert!(validate_request("GET", "mods/not-a-number").is_err());
    }

    #[test]
    fn enforces_minecraft_mods_scope() {
        let mut query =
            HashMap::from([("classId".to_string(), "4471".to_string())]);
        assert!(enforce_mod_scope(&mut query, &None).is_err());

        let mut query = HashMap::new();
        let body = serde_json::json!({ "classId": 4471 });
        assert!(enforce_mod_scope(&mut query, &Some(body)).is_err());

        let mut query = HashMap::new();
        enforce_mod_scope(&mut query, &None).unwrap();
        assert_eq!(query.get("gameId").map(String::as_str), Some("432"));
        assert_eq!(query.get("classId").map(String::as_str), Some("6"));
    }
}
