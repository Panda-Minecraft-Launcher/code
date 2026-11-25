use serde::{Deserialize, Serialize};
use std::path::Path;

use super::instances::ContentSourceKind;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceInstallStage {
    Installed,
    MinecraftInstalling,
    PackInstalled,
    PackInstalling,
    NotInstalled,
}

impl InstanceInstallStage {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::Installed => "installed",
            Self::MinecraftInstalling => "minecraft_installing",
            Self::PackInstalled => "pack_installed",
            Self::PackInstalling => "pack_installing",
            Self::NotInstalled => "not_installed",
        }
    }

    pub fn from_str(val: &str) -> Self {
        match val {
            "installed" => Self::Installed,
            "minecraft_installing" => Self::MinecraftInstalling,
            "installing" => Self::MinecraftInstalling,
            "pack_installed" => Self::PackInstalled,
            "pack_installing" => Self::PackInstalling,
            "not_installed" => Self::NotInstalled,
            _ => Self::NotInstalled,
        }
    }
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd,
)]
#[serde(rename_all = "snake_case")]
pub enum LauncherFeatureVersion {
    None,
    MigratedServerLastPlayTime,
    MigratedLaunchHooks,
}

impl LauncherFeatureVersion {
    pub const MOST_RECENT: Self = Self::MigratedLaunchHooks;

    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::None => "none",
            Self::MigratedServerLastPlayTime => {
                "migrated_server_last_play_time"
            }
            Self::MigratedLaunchHooks => "migrated_launch_hooks",
        }
    }

    pub fn from_str(val: &str) -> Self {
        match val {
            "none" => Self::None,
            "migrated_server_last_play_time" => {
                Self::MigratedServerLastPlayTime
            }
            "migrated_launch_hooks" => Self::MigratedLaunchHooks,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize)]
#[cfg_attr(
    feature = "export-ts",
    derive(ts_rs::TS, postcard_bindgen::PostcardBindings)
)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    Vanilla,
    Forge,
    Fabric,
    Quilt,
    NeoForge,
}

impl ModLoader {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Self::Vanilla => "vanilla",
            Self::Forge => "forge",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::NeoForge => "neoforge",
        }
    }

    pub fn as_meta_str(&self) -> &'static str {
        match *self {
            Self::Vanilla => "vanilla",
            Self::Forge => "forge",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::NeoForge => "neo",
        }
    }

    pub fn from_string(val: &str) -> Self {
        match val {
            "vanilla" => Self::Vanilla,
            "forge" => Self::Forge,
            "fabric" => Self::Fabric,
            "quilt" => Self::Quilt,
            "neoforge" => Self::NeoForge,
            _ => Self::Vanilla,
        }
    }
}

// 0.0.9 persisted "optifine"; keep those instances readable by treating it as vanilla.
impl<'de> Deserialize<'de> for ModLoader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ModLoaderVisitor;

        impl<'de> serde::de::Visitor<'de> for ModLoaderVisitor {
            type Value = ModLoader;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                formatter.write_str("a mod loader")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(ModLoader::from_string(value))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::EnumAccess<'de>,
            {
                use serde::de::VariantAccess;
                let (variant, access): (String, _) = data.variant()?;
                access.unit_variant()?;
                Ok(ModLoader::from_string(&variant))
            }
        }

        deserializer.deserialize_enum(
            "ModLoader",
            &[
                "vanilla", "forge", "fabric", "quilt", "neoforge", "optifine",
            ],
            ModLoaderVisitor,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentFile {
    #[serde(skip)]
    pub on_disk_path: String,
    pub hash: String,
    pub file_name: String,
    pub enabled: bool,
    pub locked: bool,
    pub size: u64,
    pub metadata: Option<FileMetadata>,
    pub update_version_id: Option<String>,
    pub project_type: ProjectType,
    pub source_kind: Option<ContentSourceKind>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub project_id: String,
    pub version_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    DataPack,
    ResourcePack,
    #[serde(alias = "shader")]
    ShaderPack,
}

impl ProjectType {
    pub fn get_from_loaders(loaders: Vec<String>) -> Option<Self> {
        if loaders
            .iter()
            .any(|x| ["fabric", "forge", "quilt", "neoforge"].contains(&&**x))
        {
            Some(ProjectType::Mod)
        } else if loaders.iter().any(|x| x == "datapack") {
            Some(ProjectType::DataPack)
        } else if loaders.iter().any(|x| ["iris", "optifine"].contains(&&**x)) {
            Some(ProjectType::ShaderPack)
        } else if loaders
            .iter()
            .any(|x| ["vanilla", "canvas", "minecraft"].contains(&&**x))
        {
            Some(ProjectType::ResourcePack)
        } else {
            None
        }
    }

    pub fn get_from_version_file(
        loaders: Vec<String>,
        file_name: &str,
    ) -> Option<Self> {
        let is_zip = Path::new(file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
        if is_zip && loaders.iter().any(|x| x == "datapack") {
            return Some(ProjectType::DataPack);
        }
        Self::get_from_loaders(loaders)
    }

    pub fn get_from_parent_folder(path: impl AsRef<Path>) -> Option<Self> {
        match path
            .as_ref()
            .parent()?
            .file_name()?
            .to_str()
            .unwrap_or_default()
        {
            "mods" => Some(ProjectType::Mod),
            "datapacks" => Some(ProjectType::DataPack),
            "resourcepacks" => Some(ProjectType::ResourcePack),
            "shaderpacks" => Some(ProjectType::ShaderPack),
            _ => None,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::DataPack => "datapack",
            ProjectType::ResourcePack => "resourcepack",
            ProjectType::ShaderPack => "shader",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "mod" | "mods" => Some(ProjectType::Mod),
            "datapack" | "datapacks" => Some(ProjectType::DataPack),
            "resourcepack" | "resourcepacks" => Some(ProjectType::ResourcePack),
            "shader" | "shaderpack" | "shaderpacks" => {
                Some(ProjectType::ShaderPack)
            }
            _ => None,
        }
    }

    pub fn get_folder(&self) -> &'static str {
        match self {
            ProjectType::Mod => "mods",
            ProjectType::DataPack => "datapacks",
            ProjectType::ResourcePack => "resourcepacks",
            ProjectType::ShaderPack => "shaderpacks",
        }
    }

    pub fn get_loaders(&self) -> &'static [&'static str] {
        match self {
            ProjectType::Mod => &["fabric", "forge", "quilt", "neoforge"],
            ProjectType::DataPack => &["datapack"],
            ProjectType::ResourcePack => &["vanilla", "canvas", "minecraft"],
            ProjectType::ShaderPack => &["iris", "optifine"],
        }
    }

    pub fn iterator() -> impl Iterator<Item = ProjectType> {
        [
            ProjectType::Mod,
            ProjectType::DataPack,
            ProjectType::ResourcePack,
            ProjectType::ShaderPack,
        ]
        .iter()
        .copied()
    }
}

impl From<ProjectType> for modrinth_content_management::ContentType {
    fn from(project_type: ProjectType) -> Self {
        match project_type {
            ProjectType::Mod => Self::Mod,
            ProjectType::DataPack => Self::DataPack,
            ProjectType::ResourcePack => Self::ResourcePack,
            ProjectType::ShaderPack => Self::Shader,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModLoader;

    #[derive(serde::Deserialize)]
    struct Holder {
        loader: ModLoader,
    }

    #[test]
    fn legacy_optifine_loader_deserializes_as_vanilla() {
        let holder: Holder =
            serde_json::from_str(r#"{"loader":"optifine"}"#).unwrap();
        assert_eq!(holder.loader, ModLoader::Vanilla);

        let holder: Holder =
            serde_json::from_str(r#"{"loader":"forge"}"#).unwrap();
        assert_eq!(holder.loader, ModLoader::Forge);
    }
}
