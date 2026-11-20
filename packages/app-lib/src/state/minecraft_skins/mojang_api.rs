use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::bail;
use bytes::Bytes;
use futures::TryStream;
use reqwest::{Body, StatusCode, multipart::Part};
use serde::Deserialize;
use serde_json::json;
use url::Url;
use uuid::Uuid;

use super::MinecraftSkinVariant;
use crate::{
    data::Credentials,
    state::{
        AccountProvider, MINECRAFT_SERVICES_USER_AGENT, MinecraftCape,
        MinecraftCharacterExpressionState, MinecraftProfile, MinecraftSkin,
        PROFILE_CACHE, ProfileCacheEntry,
    },
    util::fetch::INSECURE_REQWEST_CLIENT,
};

const ELY_BY_SKIN_SYSTEM_URL: &str = "http://skinsystem.ely.by";

#[derive(Deserialize)]
struct ElyByTextures {
    #[serde(rename = "SKIN")]
    skin: Option<ElyByTexture>,
    #[serde(rename = "CAPE")]
    cape: Option<ElyByTexture>,
}

#[derive(Deserialize)]
struct ElyByTexture {
    url: Url,
    metadata: Option<ElyByTextureMetadata>,
}

#[derive(Deserialize)]
struct ElyByTextureMetadata {
    model: Option<String>,
}

/// Fetches the profile textures exposed by ely.by's skin system.
///
/// Ely.by keys textures by username and has no documented upload API, so this
/// is read-only: skins and capes must be changed on ely.by itself.
pub async fn fetch_ely_by_online_profile(
    credentials: &Credentials,
) -> Option<Arc<MinecraftProfile>> {
    let username = &credentials.offline_profile.name;
    let response = INSECURE_REQWEST_CLIENT
        .get(format!("{ELY_BY_SKIN_SYSTEM_URL}/textures/{username}"))
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .ok()?;

    let textures = if response.status() == StatusCode::NO_CONTENT {
        None
    } else {
        Some(response.json::<ElyByTextures>().await.ok()?)
    };

    let profile = ely_by_profile_from_textures(credentials, textures);

    PROFILE_CACHE
        .lock()
        .await
        .insert(profile.id, ProfileCacheEntry::Hit(Arc::clone(&profile)));

    Some(profile)
}

fn ely_by_profile_from_textures(
    credentials: &Credentials,
    textures: Option<ElyByTextures>,
) -> Arc<MinecraftProfile> {
    let username = &credentials.offline_profile.name;
    let skins = textures
        .as_ref()
        .and_then(|textures| textures.skin.as_ref())
        .map(|skin| MinecraftSkin {
            id: ely_by_texture_id(&skin.url),
            state: MinecraftCharacterExpressionState::Active,
            url: Arc::new(ely_by_skin_url(username)),
            texture_key: None,
            variant: if skin
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.model.as_deref())
                == Some("slim")
            {
                MinecraftSkinVariant::Slim
            } else {
                MinecraftSkinVariant::Classic
            },
            name: None,
        })
        .into_iter()
        .collect();

    let capes = textures
        .as_ref()
        .and_then(|textures| textures.cape.as_ref())
        .map(|cape| MinecraftCape {
            id: ely_by_texture_id(&cape.url),
            state: MinecraftCharacterExpressionState::Active,
            url: Arc::new(cape.url.clone()),
            name: Arc::from("Ely.by cape"),
        })
        .into_iter()
        .collect();

    Arc::new(MinecraftProfile {
        id: credentials.offline_profile.id,
        name: username.clone(),
        skins,
        capes,
        fetch_time: Some(Instant::now()),
    })
}

pub fn ely_by_skin_url(username: &str) -> Url {
    Url::parse(&format!("{ELY_BY_SKIN_SYSTEM_URL}/skins/{username}.png"))
        .expect("valid ely.by skin URL")
}

fn ely_by_texture_id(url: &Url) -> Uuid {
    let mut hasher = DefaultHasher::new();
    url.as_str().hash(&mut hasher);
    let first = hasher.finish();
    first.hash(&mut hasher);
    Uuid::from_u128(((first as u128) << 64) | hasher.finish() as u128)
}

/// Provides operations for interacting with capes on a Minecraft player profile.
pub struct MinecraftCapeOperation;

impl MinecraftCapeOperation {
    pub async fn equip(
        credentials: &Credentials,
        cape_id: Uuid,
    ) -> crate::Result<()> {
        if matches!(&credentials.provider, AccountProvider::ElyBy) {
            bail!(
                "Capes are managed on ely.by and cannot be changed from the launcher"
            );
        }

        update_profile_cache_from_response(
            INSECURE_REQWEST_CLIENT
				.put("https://api.minecraftservices.com/minecraft/profile/capes/active")
				.header("Content-Type", "application/json; charset=utf-8")
				.header("Accept", "application/json")
				.header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
				.bearer_auth(&credentials.access_token)
				.json(&json!({
					"capeId": cape_id.hyphenated(),
                }))
                .send()
                .await
                .and_then(|response| response.error_for_status())?
        )
        .await;

        Ok(())
    }

    pub async fn unequip_any(credentials: &Credentials) -> crate::Result<()> {
        if matches!(&credentials.provider, AccountProvider::ElyBy) {
            bail!(
                "Capes are managed on ely.by and cannot be changed from the launcher"
            );
        }

        update_profile_cache_from_response(
			INSECURE_REQWEST_CLIENT
				.delete("https://api.minecraftservices.com/minecraft/profile/capes/active")
				.header("Accept", "application/json")
				.header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
				.bearer_auth(&credentials.access_token)
				.send()
				.await
                .and_then(|response| response.error_for_status())?
        )
        .await;

        Ok(())
    }
}

/// Provides operations for interacting with skins on a Minecraft player profile.
pub struct MinecraftSkinOperation;

impl MinecraftSkinOperation {
    pub async fn equip<TextureStream>(
        credentials: &Credentials,
        texture: TextureStream,
        variant: MinecraftSkinVariant,
    ) -> crate::Result<Option<Arc<MinecraftProfile>>>
    where
        TextureStream: TryStream + Send + 'static,
        TextureStream::Error: Into<Box<dyn Error + Send + Sync>>,
        Bytes: From<TextureStream::Ok>,
    {
        if matches!(&credentials.provider, AccountProvider::ElyBy) {
            bail!(
                "Skins are managed on ely.by and cannot be changed from the launcher"
            );
        }

        let form = reqwest::multipart::Form::new()
            .text(
                "variant",
                match variant {
                    MinecraftSkinVariant::Slim => "slim",
                    MinecraftSkinVariant::Classic => "classic",
                    _ => {
                        bail!("cannot equip skin of unknown model variant");
                    }
                },
            )
            .part(
                "file",
                Part::stream(Body::wrap_stream(texture))
                    .mime_str("image/png")?
                    .file_name("skin.png"),
            );

        let profile = update_profile_cache_from_response(
            INSECURE_REQWEST_CLIENT
                .post(
                    "https://api.minecraftservices.com/minecraft/profile/skins",
                )
                .header("Accept", "application/json")
                .header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
                .bearer_auth(&credentials.access_token)
                .multipart(form)
                .send()
                .await
                .and_then(|response| response.error_for_status())?,
        )
        .await;

        Ok(profile)
    }

    pub async fn unequip_any(credentials: &Credentials) -> crate::Result<()> {
        if matches!(&credentials.provider, AccountProvider::ElyBy) {
            bail!(
                "Skins are managed on ely.by and cannot be changed from the launcher"
            );
        }

        update_profile_cache_from_response(
			INSECURE_REQWEST_CLIENT
				.delete("https://api.minecraftservices.com/minecraft/profile/skins/active")
				.header("Accept", "application/json")
				.header("User-Agent", MINECRAFT_SERVICES_USER_AGENT)
				.bearer_auth(&credentials.access_token)
				.send()
				.await
                .and_then(|response| response.error_for_status())?
        )
        .await;

        Ok(())
    }
}

async fn update_profile_cache_from_response(
    response: reqwest::Response,
) -> Option<Arc<MinecraftProfile>> {
    let Some(mut profile) = response.json::<MinecraftProfile>().await.ok()
    else {
        tracing::warn!(
            "Failed to parse player profile from skin or cape operation response, not updating profile cache"
        );
        return None;
    };

    profile.fetch_time = Some(Instant::now());
    let profile = Arc::new(profile);

    PROFILE_CACHE
        .lock()
        .await
        .insert(profile.id, ProfileCacheEntry::Hit(Arc::clone(&profile)));

    Some(profile)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn credentials() -> Credentials {
        Credentials {
            provider: AccountProvider::ElyBy,
            offline_profile: MinecraftProfile {
                id: Uuid::nil(),
                name: "ErickSkrauch".into(),
                ..MinecraftProfile::default()
            },
            access_token: String::new(),
            refresh_token: String::new(),
            expires: Utc::now(),
            active: true,
        }
    }

    #[test]
    fn parses_ely_by_textures() {
        let textures = serde_json::from_str(
            r#"{"SKIN":{"url":"http://skinsystem.ely.by/skins/ErickSkrauch.png","metadata":{"model":"slim"}},"CAPE":{"url":"http://skinsystem.ely.by/cloaks/ErickSkrauch.png"}}"#,
        )
        .unwrap();
        let profile =
            ely_by_profile_from_textures(&credentials(), Some(textures));

        let skin = profile.current_skin().unwrap();
        assert_eq!(skin.variant, MinecraftSkinVariant::Slim);
        assert_eq!(
            skin.url.as_str(),
            "http://skinsystem.ely.by/skins/ErickSkrauch.png"
        );
        assert_eq!(
            profile.current_cape().unwrap().url.as_str(),
            "http://skinsystem.ely.by/cloaks/ErickSkrauch.png"
        );
    }

    #[test]
    fn no_textures_means_no_skin_or_cape() {
        let profile = ely_by_profile_from_textures(&credentials(), None);
        assert!(profile.skins.is_empty());
        assert!(profile.capes.is_empty());
    }
}
