//! The Minecraft version a resource root or a saved world targets.
//!
//! One profile decides which world, rendering and block-behaviour rules apply:
//! [`Profile::Modern`] is the pinned vanilla version ([`crate::VANILLA_VERSION`]),
//! [`Profile::V1_7_10`] is the era the port reads and plays. A prepared resource
//! root names its profile in the extraction's manifest, a saved world in the
//! shape of its `level.dat`, and [`Profile::resolve`] refuses a pair that
//! disagrees instead of guessing which side to trust.
//!
//! Profiles are named after the Minecraft version they target, not after the
//! era's place in a sequence, so a 1.6.4 profile can be added beside these
//! without renaming either.

use crate::nbt::Tag;
use std::{fmt, io, path::Path, path::PathBuf};

/// The manifest `sunrise-prepare` writes beside a prepared 1.7.10 folder.
pub const ERA_MANIFEST: &str = "assets-1.7.10.json";

/// The version [`Profile::V1_7_10`] names.
pub const V1_7_10_VERSION: &str = "1.7.10";

/// The Minecraft version whose rules a resource root or a world follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Profile {
    /// The pinned vanilla version, [`crate::VANILLA_VERSION`].
    #[default]
    Modern,
    /// Minecraft 1.7.10, the era resource export's rules.
    V1_7_10,
}

impl Profile {
    /// Every profile this build has.
    pub const ALL: [Self; 2] = [Self::Modern, Self::V1_7_10];

    /// The Minecraft version the profile targets.
    pub const fn version(self) -> &'static str {
        match self {
            Self::Modern => crate::VANILLA_VERSION,
            Self::V1_7_10 => V1_7_10_VERSION,
        }
    }

    /// The profile named by a Minecraft version, when this build has one.
    pub fn of_version(version: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.version() == version)
    }

    /// The profile a prepared resource root declares: [`ERA_MANIFEST`] beside
    /// it names the era, and a root without one is [`Self::Modern`].
    pub fn of_resource_root(root: &Path) -> Result<Self, ProfileError> {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Manifest {
            minecraft: String,
        }

        let path = root.join(ERA_MANIFEST);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::Modern),
            Err(error) => {
                return Err(ProfileError::Manifest {
                    path,
                    why: error.to_string(),
                });
            }
        };
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|error| ProfileError::Manifest {
                path: path.clone(),
                why: error.to_string(),
            })?;
        Self::of_version(&manifest.minecraft).ok_or(ProfileError::Unknown {
            version: manifest.minecraft,
        })
    }

    /// The profile a world's `level.dat` `Data` compound declares: 1.7.10's
    /// saves carry `RandomSeed` and `version` and none of the DataVersion that
    /// 1.8 introduced, so every other shape is [`Self::Modern`]'s.
    pub fn of_level_data(data: &Tag) -> Self {
        if data.get("DataVersion").is_none()
            && data.get("RandomSeed").is_some()
            && data.get("version").is_some()
        {
            Self::V1_7_10
        } else {
            Self::Modern
        }
    }

    /// Resolve a resource root's profile against the world's, when the world
    /// has one. The world decides when they agree; a 1.7.10 world opened with
    /// 26.3 resources (or the other way round) is a mismatch to report.
    pub fn resolve(root: Self, world: Option<Self>) -> Result<Self, ProfileError> {
        match world {
            Some(world) if world != root => Err(ProfileError::Mismatch { root, world }),
            Some(world) => Ok(world),
            None => Ok(root),
        }
    }
}

/// Why a resource root or a world could not be given one profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    /// The root's manifest did not read, or named a version this build lacks.
    Manifest { path: PathBuf, why: String },
    /// The root's manifest names a Minecraft this build has no profile for.
    Unknown { version: String },
    /// The world and the resource root target different versions.
    Mismatch { root: Profile, world: Profile },
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest { path, why } => write!(f, "{}: {why}", path.display()),
            Self::Unknown { version } => write!(
                f,
                "Minecraft {version} is not a version this build has a profile for ({})",
                Profile::ALL.map(Profile::version).join(", ")
            ),
            Self::Mismatch { root, world } => write!(
                f,
                "the world was saved in {} but the resources are {}; open it with the {} \
                 resources",
                world.version(),
                root.version(),
                world.version()
            ),
        }
    }
}

impl std::error::Error for ProfileError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nbt::Tag;

    fn root(manifest: Option<&str>) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sunrise-profile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(manifest) = manifest {
            std::fs::write(dir.join(ERA_MANIFEST), manifest).unwrap();
        }
        dir
    }

    fn era_level() -> Tag {
        Tag::compound([("RandomSeed", Tag::Long(1)), ("version", Tag::Int(19133))])
    }

    fn modern_level() -> Tag {
        Tag::compound([("DataVersion", Tag::Int(crate::version::DATA_VERSION))])
    }

    #[test]
    fn a_modern_root_resolves_to_the_modern_profile() {
        let dir = root(None);
        assert_eq!(Profile::of_resource_root(&dir).unwrap(), Profile::Modern);
        assert_eq!(
            Profile::resolve(Profile::Modern, None).unwrap(),
            Profile::Modern
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_era_root_resolves_to_the_era_profile() {
        let dir = root(Some(r#"{"minecraft":"1.7.10"}"#));
        assert_eq!(Profile::of_resource_root(&dir).unwrap(), Profile::V1_7_10);
        assert_eq!(
            Profile::resolve(Profile::V1_7_10, Some(Profile::of_level_data(&era_level()))).unwrap(),
            Profile::V1_7_10
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_era_world_with_modern_resources_is_a_mismatch() {
        let dir = root(None);
        let root_profile = Profile::of_resource_root(&dir).unwrap();
        let error =
            Profile::resolve(root_profile, Some(Profile::of_level_data(&era_level()))).unwrap_err();
        assert!(matches!(
            error,
            ProfileError::Mismatch {
                root: Profile::Modern,
                world: Profile::V1_7_10
            }
        ));
        assert!(error.to_string().contains("1.7.10"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_modern_world_with_era_resources_is_a_mismatch() {
        let dir = root(Some(r#"{"minecraft":"1.7.10"}"#));
        let root_profile = Profile::of_resource_root(&dir).unwrap();
        let error = Profile::resolve(root_profile, Some(Profile::of_level_data(&modern_level())))
            .unwrap_err();
        assert!(matches!(
            error,
            ProfileError::Mismatch {
                root: Profile::V1_7_10,
                world: Profile::Modern
            }
        ));
        assert!(error.to_string().contains("26.3"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
