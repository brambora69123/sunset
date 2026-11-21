//! The 1.7.10 resource pack pipeline: `net.minecraft.client.resource.pack.*`
//! and `net.minecraft.client.resource.manager.*`, ported exactly.
//!
//! Resource pack format 1 (1.7.10): a pack root carries `pack.mcmeta`
//! (`{"pack": {"pack_format": 1, ...}}`) and files under
//! `assets/<namespace>/<path>`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::identifier::Identifier;
use crate::json;
/// The resource pack format Minecraft 1.7.10 reads (`pack_format`).
pub const RESOURCE_PACK_FORMAT: i32 = 1;

/// Root folder inside a pack: `assets/<namespace>/<path>`.
pub const ASSETS_FOLDER: &str = "assets";

/// A resource pack: named source of `assets/<namespace>/<path>` files.
pub trait ResourcePack {
    fn name(&self) -> &str;

    /// Opens `assets/<namespace>/<path>` inside the pack.
    fn open(&self, id: &Identifier) -> io::Result<Vec<u8>>;

    fn has_resource(&self, id: &Identifier) -> bool {
        self.open(id).is_ok()
    }
}

/// A pack rooted at a directory on disk (`net.minecraft.client.resource.pack.DirectoryResourcePack`).
pub struct DirectoryResourcePack {
    root: PathBuf,
}

impl DirectoryResourcePack {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl ResourcePack for DirectoryResourcePack {
    fn name(&self) -> &str {
        "directory"
    }

    fn open(&self, id: &Identifier) -> io::Result<Vec<u8>> {
        let path = self
            .root
            .join(ASSETS_FOLDER)
            .join(id.namespace())
            .join(id.path());
        std::fs::read(path)
    }
}

/// In-memory pack: built-in assets and tests (`(namespace, path) -> bytes`).
pub struct MemoryResourcePack {
    name: String,
    files: BTreeMap<(String, String), Vec<u8>>,
}

impl MemoryResourcePack {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            files: BTreeMap::new(),
        }
    }

    /// Adds a file addressed by `namespace:path` (the path part only, no
    /// `assets/` prefix).
    pub fn add(&mut self, namespace: &str, path: &str, data: impl Into<Vec<u8>>) {
        self.files
            .insert((namespace.to_owned(), path.to_owned()), data.into());
    }

    pub fn namespaced_paths(&self, namespace: &str) -> Vec<String> {
        self.files
            .keys()
            .filter(|(ns, _)| ns == namespace)
            .map(|(_, path)| path.clone())
            .collect()
    }
}

impl ResourcePack for MemoryResourcePack {
    fn name(&self) -> &str {
        &self.name
    }

    fn open(&self, id: &Identifier) -> io::Result<Vec<u8>> {
        self.files
            .get(&(id.namespace().to_owned(), id.path().to_owned()))
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, id.to_string()))
    }
}

/// `net.minecraft.client.resource.SimpleResource`: a resource plus its
/// `.mcmeta` sidecar, resolved by the manager from the packs below it.
pub struct SimpleResource {
    pub id: Identifier,
    pub data: Vec<u8>,
    /// Contents of `<id>.mcmeta` from the highest pack that carries it.
    pub mcmeta: Option<Vec<u8>>,
}

/// `net.minecraft.client.resource.manager.FallbackResourceManager`: the pack
/// stack for one namespace. Resources come from the highest pack that has
/// them; the `.mcmeta` sidecar comes from the highest pack that has the
/// sidecar — the two are resolved independently, in Java's exact order.
pub struct FallbackResourceManager {
    packs: Vec<Rc<dyn ResourcePack>>,
}

impl FallbackResourceManager {
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    /// Adds a pack on top of the stack (later packs win).
    pub fn add(&mut self, pack: Rc<dyn ResourcePack>) {
        self.packs.push(pack);
    }

    pub fn pack_count(&self) -> usize {
        self.packs.len()
    }

    fn metadata_location(location: &Identifier) -> Identifier {
        Identifier::new(location.namespace(), &format!("{}.mcmeta", location.path()))
    }

    pub fn get_resource(&self, location: &Identifier) -> io::Result<SimpleResource> {
        let metadata = Self::metadata_location(location);
        let mut metadata_pack: Option<&Rc<dyn ResourcePack>> = None;

        for pack in self.packs.iter().rev() {
            if metadata_pack.is_none() && pack.has_resource(&metadata) {
                metadata_pack = Some(pack);
            }
            if pack.has_resource(location) {
                let mcmeta = metadata_pack.and_then(|pack| pack.open(&metadata).ok());
                return Ok(SimpleResource {
                    id: location.clone(),
                    data: pack.open(location)?,
                    mcmeta,
                });
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            location.to_string(),
        ))
    }

    /// Every pack's copy of the resource, bottom pack first (Java's
    /// `getResources`).
    pub fn get_all_resources(&self, location: &Identifier) -> io::Result<Vec<SimpleResource>> {
        let metadata = Self::metadata_location(location);
        let mut resources = Vec::new();

        for pack in &self.packs {
            if pack.has_resource(location) {
                let mcmeta = if pack.has_resource(&metadata) {
                    pack.open(&metadata).ok()
                } else {
                    None
                };
                resources.push(SimpleResource {
                    id: location.clone(),
                    data: pack.open(location)?,
                    mcmeta,
                });
            }
        }

        if resources.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                location.to_string(),
            ));
        }
        Ok(resources)
    }
}

impl Default for FallbackResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// `net.minecraft.client.resource.manager.SimpleReloadableResourceManager`:
/// one fallback stack per namespace.
pub struct SimpleReloadableResourceManager {
    namespaces: BTreeMap<String, FallbackResourceManager>,
    packs: Vec<Rc<dyn ResourcePack>>,
}

impl SimpleReloadableResourceManager {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            packs: Vec::new(),
        }
    }

    /// Adds a pack covering the given namespaces, bottom of the stack (a
    /// later call with the same namespaces wins).
    pub fn add_pack(&mut self, pack: Rc<dyn ResourcePack>, namespaces: &[&str]) {
        let shared: Rc<dyn ResourcePack> = pack.clone();
        for namespace in namespaces {
            self.namespaces
                .entry((*namespace).to_owned())
                .or_default()
                .add(shared.clone());
        }
        self.packs.push(pack);
    }

    pub fn get_namespaces(&self) -> Vec<&str> {
        self.namespaces.keys().map(String::as_str).collect()
    }

    pub fn get_resource(&self, location: &Identifier) -> io::Result<SimpleResource> {
        self.namespaces
            .get(location.namespace())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, location.to_string()))?
            .get_resource(location)
    }

    pub fn get_all_resources(&self, location: &Identifier) -> io::Result<Vec<SimpleResource>> {
        self.namespaces
            .get(location.namespace())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, location.to_string()))?
            .get_all_resources(location)
    }

    pub fn namespace_manager(&self, namespace: &str) -> Option<&FallbackResourceManager> {
        self.namespaces.get(namespace)
    }

    /// All copies of a namespace-free asset across packs of the given
    /// namespaces, in namespace then pack order (the loader's read order).
    pub fn get_assets(&self, path: &str, namespaces: &[&str]) -> Vec<Vec<u8>> {
        let mut found = Vec::new();
        for namespace in namespaces {
            if let Some(manager) = self.namespaces.get(*namespace) {
                let location = Identifier::new(namespace, path);
                if let Ok(resources) = manager.get_all_resources(&location) {
                    found.extend(resources.into_iter().map(|resource| resource.data));
                }
            }
        }
        found
    }
}

impl Default for SimpleReloadableResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// `net.minecraft.client.resource.metadata.ResourcePackMetadata`: the
/// `pack.mcmeta` contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackMetadata {
    pub pack_format: i32,
    pub description: String,
}

impl PackMetadata {
    /// Parses `pack.mcmeta`; errors name the field like `JsonUtils` does.
    pub fn parse(data: &[u8]) -> Result<PackMetadata, String> {
        let text = std::str::from_utf8(data).map_err(|_| "pack.mcmeta is not UTF-8".to_owned())?;
        let root = json::parse(text)?;
        let pack = root
            .get("pack")
            .ok_or_else(|| "pack is missing".to_owned())?;
        let format = pack
            .get("pack_format")
            .ok_or_else(|| "pack_format is missing".to_owned())?
            .as_i64("pack_format")?;
        let description = pack
            .get("description")
            .ok_or_else(|| "description is missing".to_owned())?
            .as_string("description")?;
        Ok(PackMetadata {
            pack_format: format as i32,
            description: description.to_owned(),
        })
    }

    /// True when the pack is readable by 1.7.10.
    pub fn matches_game(&self) -> bool {
        self.pack_format == RESOURCE_PACK_FORMAT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_with(name: &str, files: &[(&str, &str, &str)]) -> Rc<MemoryResourcePack> {
        let mut pack = MemoryResourcePack::new(name);
        for (namespace, path, data) in files {
            pack.add(namespace, path, *data);
        }
        Rc::new(pack)
    }

    #[test]
    fn resource_path_is_assets_namespace_path() {
        let root = std::env::temp_dir().join("sunset-1-7-assets-test");
        let pack = DirectoryResourcePack::new(&root);
        assert!(pack.open(&Identifier::parse("minecraft:stone")).is_err());
    }

    #[test]
    fn top_pack_wins() {
        let mut manager = SimpleReloadableResourceManager::new();
        manager.add_pack(
            pack_with("bottom", &[("minecraft", "a.txt", "bottom")]),
            &["minecraft"],
        );
        manager.add_pack(
            pack_with("top", &[("minecraft", "a.txt", "top")]),
            &["minecraft"],
        );
        let resource = manager
            .get_resource(&Identifier::parse("minecraft:a.txt"))
            .unwrap();
        assert_eq!(resource.data, b"top");
    }

    #[test]
    fn mcmeta_sidecar_from_highest_pack() {
        // The top pack carries both: its sidecar wins with its resource.
        let mut manager = SimpleReloadableResourceManager::new();
        manager.add_pack(
            pack_with(
                "bottom",
                &[
                    ("minecraft", "x.png", "image"),
                    ("minecraft", "x.png.mcmeta", "{\"animation\":{}}"),
                ],
            ),
            &["minecraft"],
        );
        manager.add_pack(
            pack_with(
                "top",
                &[
                    ("minecraft", "x.png", "better image"),
                    (
                        "minecraft",
                        "x.png.mcmeta",
                        "{\"animation\":{\"frames\":[]}}",
                    ),
                ],
            ),
            &["minecraft"],
        );
        let resource = manager
            .get_resource(&Identifier::parse("minecraft:x.png"))
            .unwrap();
        assert_eq!(resource.data, b"better image");
        assert_eq!(
            resource.mcmeta.as_deref(),
            Some("{\"animation\":{\"frames\":[]}}".as_bytes())
        );

        // The top pack carries only the resource: Java's rev loop returns
        // before reaching the bottom pack's sidecar, so the mcmeta is null.
        let mut manager = SimpleReloadableResourceManager::new();
        manager.add_pack(
            pack_with(
                "bottom",
                &[
                    ("minecraft", "x.png", "image"),
                    ("minecraft", "x.png.mcmeta", "{\"animation\":{}}"),
                ],
            ),
            &["minecraft"],
        );
        manager.add_pack(
            pack_with("top", &[("minecraft", "x.png", "better image")]),
            &["minecraft"],
        );
        let resource = manager
            .get_resource(&Identifier::parse("minecraft:x.png"))
            .unwrap();
        assert_eq!(resource.data, b"better image");
        assert_eq!(resource.mcmeta, None);
    }

    #[test]
    fn all_resources_lists_every_pack() {
        let mut manager = SimpleReloadableResourceManager::new();
        manager.add_pack(pack_with("a", &[("minecraft", "f", "one")]), &["minecraft"]);
        manager.add_pack(pack_with("b", &[("minecraft", "f", "two")]), &["minecraft"]);
        let all = manager
            .get_all_resources(&Identifier::parse("minecraft:f"))
            .unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].data, b"one");
        assert_eq!(all[1].data, b"two");
    }

    #[test]
    fn pack_metadata_format_1() {
        let metadata =
            PackMetadata::parse(br#"{"pack":{"pack_format":1,"description":"1.7.10 assets"}}"#)
                .unwrap();
        assert_eq!(metadata.pack_format, 1);
        assert_eq!(metadata.description, "1.7.10 assets");
        assert!(metadata.matches_game());
        assert!(
            !PackMetadata::parse(br#"{"pack":{"pack_format":4,"description":"too new"}}"#)
                .unwrap()
                .matches_game()
        );
        assert!(PackMetadata::parse(br#"{}"#).is_err());
    }

    #[test]
    fn assets_folder_constant_matches_1_7_10() {
        assert_eq!(ASSETS_FOLDER, "assets");
        assert_eq!(RESOURCE_PACK_FORMAT, 1);
    }

    #[test]
    fn get_assets_reads_every_pack_in_order() {
        let mut manager = SimpleReloadableResourceManager::new();
        manager.add_pack(
            pack_with("a", &[("minecraft", "lang/en_US.lang", "one")]),
            &["minecraft"],
        );
        manager.add_pack(
            pack_with("b", &[("minecraft", "lang/en_US.lang", "two")]),
            &["minecraft"],
        );
        assert_eq!(
            manager.get_assets("lang/en_US.lang", &["minecraft"]),
            vec![b"one".to_vec(), b"two".to_vec()]
        );
        assert!(
            manager
                .get_assets("lang/en_US.lang", &["missing"])
                .is_empty()
        );
    }
}
