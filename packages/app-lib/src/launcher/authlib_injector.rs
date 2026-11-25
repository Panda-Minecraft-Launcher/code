use crate::util::fetch::REQWEST_CLIENT;
use anyhow::Context;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

const API_ROOT: &str = "https://authserver.ely.by/api/authlib-injector";
const LATEST_URL: &str =
    "https://authlib-injector.yushi.moe/artifact/latest.json";
const JAR_NAME: &str = "authlib-injector.jar";

#[derive(Deserialize)]
struct LatestArtifact {
    download_url: String,
    checksums: Option<Checksums>,
}

#[derive(Deserialize)]
struct Checksums {
    sha256: Option<String>,
}

pub async fn add_arguments(
    command: &mut Command,
    cache_dir: &Path,
    resource_dir: &Path,
) -> crate::Result<()> {
    let jar_path = prepare_jar(cache_dir, resource_dir).await?;
    command.arg(format!("-javaagent:{}={API_ROOT}", jar_path.display()));
    command.arg("-Dauthlibinjector.noShowServerName");

    if let Some(prefetched) = prefetched_metadata().await {
        command.arg(format!(
            "-Dauthlibinjector.yggdrasil.prefetched={prefetched}"
        ));
    }

    Ok(())
}

pub async fn prefetched_metadata() -> Option<String> {
    match fetch(API_ROOT).await {
        Ok(metadata) => Some(BASE64_STANDARD.encode(metadata)),
        Err(error) => {
            tracing::debug!(%error, "skipping authlib-injector metadata prefetch");
            None
        }
    }
}

pub async fn prepare_jar(
    cache_dir: &Path,
    resource_dir: &Path,
) -> crate::Result<PathBuf> {
    fs::create_dir_all(cache_dir).await?;
    let cached_path = cache_dir.join(JAR_NAME);

    if !cached_path.is_file() {
        let latest: LatestArtifact =
            serde_json::from_slice(&fetch(LATEST_URL).await?)?;
        let jar = fetch(&latest.download_url).await?;

        if let Some(expected) = latest
            .checksums
            .as_ref()
            .and_then(|checksums| checksums.sha256.as_deref())
        {
            let actual = sha256_hex(&jar);
            if !actual.eq_ignore_ascii_case(expected) {
                anyhow::bail!(
                    "authlib-injector SHA-256 mismatch: expected {expected}, got {actual}"
                );
            }
        }

        let temp_path = cached_path.with_extension("jar.tmp");
        fs::write(&temp_path, &jar).await?;
        fs::rename(&temp_path, &cached_path).await?;
    }

    let resource_path = resource_dir.join(JAR_NAME);
    fs::copy(&cached_path, &resource_path).await?;
    Ok(resource_path)
}

async fn fetch(url: &str) -> crate::Result<Bytes> {
    REQWEST_CLIENT
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await
        .context("reading authlib-injector response")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LatestArtifact, sha256_hex};

    #[test]
    fn sha256_hex_matches_known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn parses_latest_artifact_checksum() {
        let artifact: LatestArtifact = serde_json::from_str(
            r#"{"build_number":56,"version":"1.2.8","download_url":"https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar","checksums":{"sha256":"9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10"}}"#,
        )
        .unwrap();
        assert_eq!(
            artifact.checksums.unwrap().sha256.unwrap(),
            "9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10"
        );
    }
}
