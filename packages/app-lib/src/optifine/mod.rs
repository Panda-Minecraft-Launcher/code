//! Native OptiFine version discovery and installation.

use crate::install::{
    InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter,
};
use crate::state::{
    CacheValue, CachedEntry, CreateInstance, JavaVersion, ModLoader,
    SearchResultV3, SearchResultsV3, State,
};
use crate::util::fetch::{FetchProgressFn, fetch_file, fetch_json};
use crate::util::io;
use crate::{ErrorKind, Result};
use anyhow::{Context, bail};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::future::Future;
use std::io::Read as _;
use std::path::Path;
use std::pin::Pin;

const BMCL_API: &str = "https://bmclapi2.bangbang93.com/optifine";
const GAME_VERSIONS_CACHE_KEY: &str = "optifine:game_versions";
const LAUNCHWRAPPER_LIBRARY: &str = "net.minecraft:launchwrapper:1.12";
const LAUNCHWRAPPER_MAIN_CLASS: &str = "net.minecraft.launchwrapper.Launch";
const OPTIFINE_TWEAKER: &str = "optifine.OptiFineTweaker";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptiFineVersion {
    pub game_version: String,
    pub version: String,
    pub label: String,
    pub filename: String,
    pub preview: bool,
    pub forge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptiFineInstallInput {
    pub game_version: String,
    pub version: Option<String>,
}

#[derive(Deserialize)]
struct RawGameVersion {
    mcversion: String,
}

#[derive(Deserialize)]
struct RawOptiFineVersion {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    patch: String,
    filename: String,
    forge: Option<String>,
}

pub async fn game_versions() -> Result<Vec<String>> {
    let state = State::get().await?;
    if let Some(cached) = CachedEntry::get_search_results_v3(
        GAME_VERSIONS_CACHE_KEY,
        None,
        &state.pool,
        &state.fetch_semaphore,
    )
    .await?
    {
        let versions = cached
            .result
            .hits
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !versions.is_empty() {
            return Ok(versions);
        }
    }

    let raw: Vec<RawGameVersion> = fetch_json(
        Method::GET,
        &format!("{BMCL_API}/versionList"),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await
    .context("fetching OptiFine game versions")?;

    let mut versions = raw
        .into_iter()
        .map(|version| version.mcversion)
        .collect::<Vec<_>>();
    versions.sort_by(|a, b| natural_cmp(b, a));
    versions.dedup();
    cache_values(
        &state,
        GAME_VERSIONS_CACHE_KEY,
        versions.iter().cloned().map(Value::String).collect(),
    )
    .await?;
    Ok(versions)
}

pub async fn list_versions(game_version: &str) -> Result<Vec<OptiFineVersion>> {
    let state = State::get().await?;
    let key = cache_key(game_version);
    if let Some(cached) = CachedEntry::get_search_results_v3(
        &key,
        None,
        &state.pool,
        &state.fetch_semaphore,
    )
    .await?
    {
        let versions = cached
            .result
            .hits
            .into_iter()
            .filter_map(|value| serde_json::from_value(value).ok())
            .collect::<Vec<OptiFineVersion>>();
        if !versions.is_empty() {
            return Ok(versions);
        }
    }

    let raw: Vec<RawOptiFineVersion> = fetch_json(
        Method::GET,
        &format!("{BMCL_API}/{game_version}"),
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
    )
    .await
    .with_context(|| {
        format!("fetching OptiFine versions for Minecraft {game_version}")
    })?;

    let mut versions = raw
        .into_iter()
        .map(|version| OptiFineVersion {
            game_version: game_version.to_string(),
            version: format!("{}_{}", version.r#type, version.patch),
            label: label(&version.r#type, &version.patch),
            preview: is_preview_patch(&version.patch)
                || version
                    .filename
                    .to_ascii_lowercase()
                    .starts_with("preview_"),
            forge: version.forge.filter(|forge| forge != "N/A"),
            filename: version.filename,
        })
        .collect::<Vec<_>>();
    sort_versions(&mut versions);
    cache_values(
        &state,
        &key,
        versions
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<Vec<_>, _>>()?,
    )
    .await?;
    Ok(versions)
}

pub async fn install(
    input: OptiFineInstallInput,
) -> Result<crate::install::InstallJobSnapshot> {
    let state = State::get().await?;
    let game_version = input.game_version.clone();
    let version_label = input
        .version
        .as_deref()
        .and_then(|version| version.rsplit_once('_'))
        .map(|(type_, patch)| label(type_, patch));
    let name = version_label.as_deref().map_or_else(
        || format!("{game_version} OptiFine"),
        |label| format!("{game_version} {label} OptiFine"),
    );
    let folder_name = version_label.as_deref().map_or_else(
        || format!("{game_version}_OptiFine"),
        |label| format!("{game_version}_{}_OptiFine", label.replace(' ', "_")),
    );
    let instance = crate::state::create_instance(
        CreateInstance {
            name,
            path: Some(sanitize_name(&folder_name)),
            game_version: game_version.clone(),
            loader: ModLoader::OptiFine,
            loader_version: Some("optifine".to_string()),
            icon_path: None,
            icon_config: None,
            link: crate::state::InstanceLink::Unmanaged,
        },
        &state,
    )
    .await?;

    crate::install::runner::install_optifine(
        instance.id,
        game_version,
        input.version,
    )
    .await
}

pub(crate) async fn install_into_instance(
    instance_id: &str,
    game_version: String,
    version: Option<String>,
    reporter: InstallProgressReporter,
) -> Result<()> {
    let state = State::get().await?;
    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            ErrorKind::InputError(format!("unknown instance {instance_id}"))
        })?;
    let instance_path = instance.instance.path.clone();
    let instance_dir = state.directories.instance_path(&instance_path);
    io::create_dir_all(&instance_dir).await?;
    let version_id = format!("{game_version}-optifine");
    let details = InstallPhaseDetails::Minecraft {
        game_version: game_version.clone(),
        loader: ModLoader::OptiFine,
    };

    reporter
        .update(InstallPhaseId::ResolvingMinecraft, None, details.clone())
        .await?;

    let vanilla_dir = state.directories.version_dir(&game_version);
    let vanilla_jar = vanilla_dir.join(format!("{game_version}.jar"));
    let vanilla_json = vanilla_dir.join(format!("{game_version}.json"));
    if !vanilla_jar.is_file() || !vanilla_json.is_file() {
        let mut context =
            crate::state::instances::commands::get_instance_launch_context(
                instance_id,
                &state.pool,
            )
            .await?
            .ok_or_else(|| {
                ErrorKind::InputError(format!("unknown instance {instance_id}"))
            })?;
        context.applied_content_set.loader = ModLoader::Vanilla;
        context.applied_content_set.loader_version = None;
        crate::launcher::install_minecraft_with_reporter(
            &context,
            false,
            Some(reporter.clone()),
            &state,
        )
        .await?;
    }

    let versions = list_versions(&game_version).await?;
    let selected = match version {
        Some(wanted) => versions
            .iter()
            .find(|candidate| candidate.version == wanted)
            .cloned()
            .ok_or_else(|| {
                ErrorKind::LauncherError(format!(
                    "unknown OptiFine version {wanted} for Minecraft {game_version}"
                ))
            })?,
        None => select_default(&versions).cloned().ok_or_else(|| {
            ErrorKind::LauncherError(format!(
                "no OptiFine versions found for Minecraft {game_version}"
            ))
        })?,
    };

    let (installer_type, patch) =
        selected.version.rsplit_once('_').ok_or_else(|| {
            ErrorKind::LauncherError(format!(
                "invalid OptiFine version {}",
                selected.version
            ))
        })?;
    let download_url =
        format!("{BMCL_API}/{game_version}/{installer_type}/{patch}");
    reporter
        .update(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: 0,
                total: 0,
                secondary: None,
            }),
            details.clone(),
        )
        .await?;
    let progress_details = details.clone();
    let mut progress_fn = {
        let reporter = reporter.clone();
        move |downloaded: u64,
              total: u64|
              -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
            let reporter = reporter.clone();
            let details = progress_details.clone();
            Box::pin(async move {
                reporter
                    .update(
                        InstallPhaseId::DownloadingContent,
                        Some(InstallProgress {
                            current: downloaded,
                            total: total.max(downloaded),
                            secondary: None,
                        }),
                        details,
                    )
                    .await
            })
        }
    };
    let installer = fetch_file(
        &download_url,
        None,
        None,
        None,
        &state.fetch_semaphore,
        &state.pool,
        Some(&mut progress_fn as &mut FetchProgressFn<'_>),
        &state.directories.store_staging_dir(),
    )
    .await
    .with_context(|| format!("downloading OptiFine from {download_url}"))?;
    reporter
        .update(
            InstallPhaseId::DownloadingContent,
            Some(InstallProgress {
                current: installer.size,
                total: installer.size,
                secondary: None,
            }),
            details.clone(),
        )
        .await?;
    verify_zip(installer.path())?;

    reporter
        .update(
            InstallPhaseId::RunningLoaderProcessors,
            None,
            details.clone(),
        )
        .await?;
    let patched_jar = instance_dir.join(format!("{version_id}.jar"));
    if has_zip_entry(installer.path(), "optifine/Patcher.class")? {
        let java =
            java_binary(&state, java_major(&vanilla_json).await?).await?;
        let output = tokio::process::Command::new(&java)
            .arg("-cp")
            .arg(installer.path())
            .arg("optifine.Patcher")
            .arg(&vanilla_jar)
            .arg(installer.path())
            .arg(&patched_jar)
            .output()
            .await
            .with_context(|| {
                format!("running OptiFine Patcher with {}", java.display())
            })?;
        if !output.status.success() {
            bail!(ErrorKind::LauncherError(format!(
                "OptiFine Patcher failed: {}",
                output_text(&output.stdout, &output.stderr)
            )));
        }
    } else {
        io::copy(installer.path(), &patched_jar).await?;
    }
    verify_optifine_jar(&patched_jar)?;

    reporter
        .update(InstallPhaseId::Finalizing, None, details)
        .await?;
    drop(installer);
    write_version_json(
        &state,
        &game_version,
        &version_id,
        &instance_path,
        &vanilla_json,
    )
    .await?;
    Ok(())
}

fn cache_key(game_version: &str) -> String {
    format!("optifine:{game_version}")
}

async fn cache_values(
    state: &State,
    key: &str,
    hits: Vec<Value>,
) -> Result<()> {
    let total_hits = hits.len() as u32;
    let mut entry = CacheValue::SearchResultsV3(SearchResultsV3 {
        search: key.to_string(),
        result: SearchResultV3 {
            hits,
            page: 0,
            hits_per_page: 0,
            total_hits,
        },
    })
    .get_entry();
    entry.set_alias(key.to_string());
    let entries = [entry];
    CachedEntry::upsert_many(&entries, &state.pool).await
}

fn is_preview_patch(patch: &str) -> bool {
    patch.split('_').any(|segment| {
        let segment = segment.to_ascii_lowercase();
        segment.starts_with("pre") || segment.starts_with("alpha")
    })
}

fn label(type_: &str, patch: &str) -> String {
    let combined = if patch.is_empty() {
        type_.to_string()
    } else {
        format!("{type_}_{patch}")
    };
    // shortcut: only HD_U_ prefixes are stripped; add other schemes when they appear.
    combined
        .strip_prefix("HD_U_")
        .unwrap_or(&combined)
        .replace('_', " ")
}

fn sort_versions(versions: &mut [OptiFineVersion]) {
    versions.sort_by(|a, b| {
        natural_cmp(&b.version, &a.version)
            .then_with(|| natural_cmp(&b.label, &a.label))
            .then_with(|| b.filename.cmp(&a.filename))
    });
}

fn select_default(versions: &[OptiFineVersion]) -> Option<&OptiFineVersion> {
    let mut ordered = versions.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        natural_cmp(&b.version, &a.version)
            .then_with(|| natural_cmp(&b.label, &a.label))
    });
    ordered
        .iter()
        .copied()
        .find(|version| !version.preview)
        .or_else(|| ordered.first().copied())
}

fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut ai, mut bi) = (0, 0);
    while ai < a.len() && bi < b.len() {
        if a[ai].is_ascii_digit() && b[bi].is_ascii_digit() {
            let (a_start, b_start) = (ai, bi);
            while ai < a.len() && a[ai].is_ascii_digit() {
                ai += 1;
            }
            while bi < b.len() && b[bi].is_ascii_digit() {
                bi += 1;
            }
            let a_number = std::str::from_utf8(&a[a_start..ai])
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(u128::MAX);
            let b_number = std::str::from_utf8(&b[b_start..bi])
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(u128::MAX);
            if a_number != b_number {
                return a_number.cmp(&b_number);
            }
        } else {
            let order = a[ai].cmp(&b[bi]);
            if order != Ordering::Equal {
                return order;
            }
            ai += 1;
            bi += 1;
        }
    }
    a.len().cmp(&b.len())
}

fn verify_zip(path: &Path) -> Result<()> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut magic = [0u8; 2];
    file.read_exact(&mut magic)
        .with_context(|| format!("reading ZIP header of {}", path.display()))?;
    if magic != *b"PK" {
        bail!(ErrorKind::LauncherError(format!(
            "{} is not a ZIP archive",
            path.display()
        )));
    }
    Ok(())
}

fn has_zip_entry(path: &Path, entry: &str) -> Result<bool> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading ZIP archive {}", path.display()))?;
    for index in 0..archive.len() {
        if archive.by_index(index)?.name()? == entry {
            return Ok(true);
        }
    }
    Ok(false)
}

fn verify_optifine_jar(path: &Path) -> Result<()> {
    if !has_zip_entry(path, "optifine/OptiFineTweaker.class")? {
        bail!(ErrorKind::LauncherError(format!(
            "{} is missing optifine/OptiFineTweaker.class",
            path.display()
        )));
    }
    Ok(())
}

fn output_text(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    }
}

async fn java_major(vanilla_json: &Path) -> Result<u32> {
    let bytes = io::read(vanilla_json).await?;
    let version: Value = serde_json::from_slice(&bytes)?;
    Ok(version
        .get("javaVersion")
        .and_then(|java| java.get("majorVersion"))
        .and_then(Value::as_u64)
        .unwrap_or(8) as u32)
}

async fn java_binary(
    state: &State,
    major_version: u32,
) -> Result<std::path::PathBuf> {
    let path = if let Some(java) =
        JavaVersion::get(major_version, &state.pool).await?
    {
        java.path
    } else {
        let path = crate::api::jre::auto_install_java(major_version).await?;
        let java = crate::api::jre::check_jre(path).await?;
        java.upsert(&state.pool).await?;
        java.path
    };
    Ok(std::path::PathBuf::from(path))
}

async fn write_version_json(
    state: &State,
    game_version: &str,
    version_id: &str,
    instance_path: &str,
    vanilla_json: &Path,
) -> Result<()> {
    let bytes = io::read(vanilla_json).await?;
    let mut version: Value = serde_json::from_slice(&bytes)?;
    version["id"] = json!(version_id);
    version["mainClass"] = json!(LAUNCHWRAPPER_MAIN_CLASS);

    if let Some(game) = version
        .get_mut("arguments")
        .and_then(|arguments| arguments.get_mut("game"))
        .and_then(Value::as_array_mut)
    {
        game.push(json!("--tweakClass"));
        game.push(json!(OPTIFINE_TWEAKER));
    } else if let Some(arguments) =
        version.get("minecraftArguments").and_then(Value::as_str)
    {
        let mut arguments = arguments.to_string();
        arguments.push_str(" --tweakClass ");
        arguments.push_str(OPTIFINE_TWEAKER);
        version["minecraftArguments"] = json!(arguments);
    }

    let libraries = version
        .get_mut("libraries")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            ErrorKind::LauncherError(format!(
                "vanilla version JSON for {game_version} has no libraries"
            ))
        })?;
    if !libraries.iter().any(|library| {
        library.get("name").and_then(Value::as_str)
            == Some(LAUNCHWRAPPER_LIBRARY)
    }) {
        libraries.push(json!({ "name": LAUNCHWRAPPER_LIBRARY }));
    }

    let path = state
        .directories
        .version_dir_for_instance(version_id, instance_path)
        .join(format!("{version_id}.json"));
    io::write(&path, &serde_json::to_vec(&version)?).await?;
    Ok(())
}

fn sanitize_name(input: &str) -> String {
    input
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || matches!(character, '_' | '-' | '.')
            {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_end_matches(['.', ' ', '_'])
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        OptiFineVersion, is_preview_patch, label, natural_cmp, select_default,
        sort_versions,
    };
    use std::cmp::Ordering;

    fn version(version: &str, preview: bool) -> OptiFineVersion {
        OptiFineVersion {
            game_version: "26.2".to_string(),
            version: version.to_string(),
            label: version.rsplit('_').next().unwrap_or_default().to_string(),
            filename: format!("{version}.jar"),
            preview,
            forge: None,
        }
    }

    #[test]
    fn natural_order_handles_preview_numbers() {
        assert_eq!(natural_cmp("pre10", "pre2"), Ordering::Greater);
        assert_eq!(natural_cmp("J9", "J8"), Ordering::Greater);
        assert_eq!(natural_cmp("K2_pre5", "K2_pre10"), Ordering::Less);
    }

    #[test]
    fn sorts_numerically_descending() {
        let mut versions = vec![
            version("HD_U_K2_pre2", true),
            version("HD_U_K2_pre10", true),
            version("HD_U_J9", false),
        ];
        sort_versions(&mut versions);
        let versions = versions
            .iter()
            .map(|version| version.version.as_str())
            .collect::<Vec<_>>();
        assert_eq!(versions, ["HD_U_K2_pre10", "HD_U_K2_pre2", "HD_U_J9"]);
    }

    #[test]
    fn prefers_stable_but_falls_back_to_preview() {
        assert!(is_preview_patch("M6_pre2"));
        assert!(is_preview_patch("K2_pre5"));
        assert!(is_preview_patch("J9_pre3"));
        assert!(is_preview_patch("preview2"));
        assert!(!is_preview_patch("M5"));
        assert!(!is_preview_patch("J9"));

        let from_patch = |patch: &str| {
            version(&format!("HD_U_{patch}"), is_preview_patch(patch))
        };
        let m5 = from_patch("M5");
        let j9 = from_patch("J9");
        let k2_pre5 = from_patch("K2_pre5");
        assert_eq!(
            select_default(&[k2_pre5.clone(), m5.clone()])
                .unwrap()
                .version,
            m5.version
        );
        assert_eq!(
            select_default(&[k2_pre5.clone(), j9.clone()])
                .unwrap()
                .version,
            j9.version
        );
        let mut previews = vec![from_patch("K2_pre2"), k2_pre5.clone()];
        sort_versions(&mut previews);
        assert_eq!(select_default(&previews).unwrap().version, k2_pre5.version);
    }

    #[test]
    fn labels_drop_the_common_opting_prefix() {
        assert_eq!(label("HD_U_K2", "pre5"), "K2 pre5");
        assert_eq!(label("HD_U", "J9"), "J9");
    }
}
