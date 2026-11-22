//! Minecraft 1.7.10 block rendering for the engine's mesher.
//!
//! A 1.7.10 world's blocks are the era's `(id, metadata)` pairs, which the
//! engine's content export carries as one `meta` property on every block state.
//! This module is the second meshing path: instead of 26.3's baked block models
//! it runs the era's `RenderBlocks` ([`render`]) on the state's id and metadata,
//! with the sprite each face registers ([`sunset17_assets`]) and the era's own
//! light and tint model.
//!
//! The era's render assets are picked by the resource root's profile (the one
//! the font and HUD loaders read), and the mesher's path by the same profile:
//! [`Profile::V1_7_10`](sunrise_core::profile::Profile::V1_7_10) selects the
//! content export's `meta` rows, [`Profile::Modern`] never does. No 26.3 state
//! takes the era path, so the 26.3 path never changes.
//!
//! - [`blocks`]: the era block facts (shape, culling, tint) read by name.
//! - [`render`]: the ported `RenderBlocks` and its vertex stream.
//! - [`fixture`]: the probe's sample world, the oracle's atlas cells.
//! - [`sprite`]: atlas sprites and their `getU`/`getV` mapping.
//! - [`world`]: the render-side world view and `getBrightness` smoothing.

pub mod blocks;
pub mod fixture;
pub mod material;
pub mod render;
pub mod sprite;
mod stairs;
pub mod tesselator;
pub mod world;

use crate::assets::{Assets, RenderAssets};
use crate::biome::Biomes;
use crate::mesher::{OPAQUE_LAYER, Padded, SectionMesh, TerrainVertex, Tints, pi, round_to_u16};
use crate::sprites::decode_rgba;
use blocks::{BlockDef, Tint};
use render::{RenderBlocks, RenderOptions};
use sprite::Sprite;
use sunrise_render_core::UV_STEPS;
use sunrise_world::registry::BlockStateId;
use world::WorldView;

/// One 1.7.10 block state: the era block, its metadata, its emitted light and
/// its six face sprites.
#[derive(Clone, Debug)]
pub struct State17 {
    pub block: BlockDef,
    pub meta: u8,
    /// `Block.getLight()`, the light the block emits.
    pub light: u8,
    /// Bottom, top, north, south, west, east (`Block.getSprite(face, metadata)`).
    pub sprites: [Sprite; 6],
    /// `grass_side_overlay`, the fancy-graphics side overlay pass.
    pub overlay: Option<Sprite>,
}

impl State17 {
    /// The air state every position outside the world reads.
    pub fn air() -> State17 {
        State17 {
            block: BlockDef::new("air"),
            meta: 0,
            light: 0,
            sprites: [Sprite::unit(0); 6],
            overlay: None,
        }
    }
}

/// The era's per-state table, indexed by the engine's block state id.
///
/// The fixture (the probe and the unit tests) and a 1.7.10 content root both
/// build one; state 0 is always air.
#[derive(Clone, Debug)]
pub struct StateTable17 {
    pub states: Vec<State17>,
    /// The fixture's `(id, metadata)` pairs by state index; empty for a content
    /// root, whose state ids are the content export's.
    pub pairs: Vec<(u16, u8)>,
}

impl StateTable17 {
    /// The state with this engine id: air for an id this table does not know.
    pub fn get(&self, index: u16) -> &State17 {
        self.states
            .get(usize::from(index))
            .unwrap_or_else(|| &self.states[0])
    }

    /// The fixture state of an era `(id, metadata)` pair; air when the fixture
    /// has none.
    pub fn state_of(&self, id: u16, meta: u8) -> u16 {
        self.pairs
            .iter()
            .position(|&pair| pair == (id, meta))
            .unwrap_or(0) as u16
    }

    /// The era's per-state table of a 1.7.10 content root.
    ///
    /// A state whose block name the export does not know (a block a mod added
    /// to the era registry) stays air: the era has no id for it, so it has no
    /// sprite either.
    pub fn era(
        assets: &Assets,
        render: &RenderAssets,
        emission: &[u8],
    ) -> crate::Result<StateTable17> {
        let era = render
            .era
            .as_ref()
            .ok_or("the 1.7.10 mesher needs the era's prepared assets")?;
        if assets.registry.len() > usize::from(u16::MAX) {
            return Err(format!(
                "the 1.7.10 mesher indexes states by u16, and the registry has {}",
                assets.registry.len()
            )
            .into());
        }
        let mut states = Vec::with_capacity(assets.registry.len());
        for id in 0..assets.registry.len() {
            let Some(state) = assets.registry.state(BlockStateId(id as u32)) else {
                states.push(State17::air());
                continue;
            };
            // The manifest names blocks with their namespace (`minecraft:stone`);
            // the render-side facts are keyed by the era's bare name (`stone`).
            let name = state.name.as_str();
            let Some(row) = era.blocks.iter().find(|row| row.name == name) else {
                states.push(State17::air());
                continue;
            };
            let plain = name.strip_prefix("minecraft:").unwrap_or(name);
            let meta: u8 = state
                .properties
                .get("meta")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let mut sprites = [Sprite::unit(0); 6];
            for (face, sprite) in sprites.iter_mut().enumerate() {
                let Some(texture) = crate::sunset17_assets::block_texture(row.id, meta, face as u8)
                else {
                    continue;
                };
                let Some(&layer) = render
                    .sprite_layers
                    .get(&crate::sunset17_assets::sprite_name(&texture))
                else {
                    continue;
                };
                *sprite = Sprite::unit(layer as u16).marked(texture.sprite() == "grass_side");
            }
            let overlay = era
                .overlay_layer
                .map(|layer| Sprite::unit(layer).marked(true));
            let light = emission.get(id).copied().unwrap_or(0);
            states.push(State17 {
                block: BlockDef::new(plain).with_key(row.id),
                meta,
                light,
                sprites,
                overlay,
            });
        }
        Ok(StateTable17 {
            states,
            pairs: Vec::new(),
        })
    }
}

/// The era's biome colormaps (`textures/colormap/grass.png`, `foliage.png`):
/// `GrassColors`/`FoliageColors`' 256x256 tables of `0xAARRGGBB`.
#[derive(Clone, Debug, Default)]
pub struct Colormaps {
    pub grass: Vec<u32>,
    pub foliage: Vec<u32>,
}

impl Colormaps {
    /// Read the two colormaps of a 1.7.10 resource source; an absent file is an
    /// empty (all black) table, like vanilla's `new int[65536]`.
    pub fn load(source: &crate::resource_source::ResourceSource) -> Colormaps {
        let read = |name: &str| {
            let path = format!("assets/minecraft/textures/colormap/{name}.png");
            let (width, height, rgba) = decode_rgba(&source.read(&path).ok()?).ok()?;
            if (width, height) != (256, 256) {
                log::warn!("{path} is {width}x{height}, not 256x256");
            }
            Some(
                rgba.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|pixel| {
                        u32::from(pixel[3]) << 24
                            | u32::from(pixel[0]) << 16
                            | u32::from(pixel[1]) << 8
                            | u32::from(pixel[2])
                    })
                    .collect(),
            )
        };
        Colormaps {
            grass: read("grass").unwrap_or_else(|| vec![0; 65536]),
            foliage: read("foliage").unwrap_or_else(|| vec![0; 65536]),
        }
    }

    /// `GrassColors.getColor(temperature, humidity)`.
    pub fn grass(&self, temperature: f64, humidity: f64) -> i32 {
        Self::sample(&self.grass, temperature, humidity)
    }

    /// `FoliageColors.get(temperature, humidity)`.
    pub fn foliage(&self, temperature: f64, humidity: f64) -> i32 {
        Self::sample(&self.foliage, temperature, humidity)
    }

    fn sample(colors: &[u32], temperature: f64, humidity: f64) -> i32 {
        let humidity = humidity * temperature;
        let u = ((1.0 - temperature) * 255.0) as usize;
        let v = ((1.0 - humidity) * 255.0) as usize;
        // The table's entries carry alpha; the renderer multiplies RGB.
        (colors.get(v << 8 | u).copied().unwrap_or(0) & 0xFF_FFFF) as i32
    }
}

/// The 1.7.10 data the renderer keeps beside its sprites: the content export's
/// block rows and the era's colormaps.
#[derive(Clone, Debug)]
pub struct EraRenderAssets {
    /// Every block row of the era extraction's manifest: the numeric id and name.
    pub blocks: Vec<crate::sunset17_assets::BlockRow>,
    /// The layer `grass_side_overlay` sits on, when the atlas has it.
    pub overlay_layer: Option<u16>,
    pub colormaps: Colormaps,
}

impl EraRenderAssets {
    pub fn load(
        era: &crate::sunset17_assets::Assets,
        layers: &std::collections::BTreeMap<String, u32>,
        source: &crate::resource_source::ResourceSource,
    ) -> crate::Result<EraRenderAssets> {
        let overlay_layer = crate::sunset17_assets::Texture::from_sprite(
            crate::sunset17_assets::Atlas::Blocks,
            "grass_side_overlay",
        )
        .and_then(|texture| {
            layers
                .get(&crate::sunset17_assets::sprite_name(&texture))
                .map(|&layer| layer as u16)
        });
        Ok(EraRenderAssets {
            blocks: era.blocks().to_vec(),
            overlay_layer,
            colormaps: Colormaps::load(source),
        })
    }
}

/// The world view of one section's padded blocks and light.
struct SectionWorld<'a> {
    pad: &'a Padded,
    table: &'a StateTable17,
    biomes: &'a Biomes,
    colormaps: &'a Colormaps,
    tints: &'a Tints<'a>,
}

impl SectionWorld<'_> {
    /// `Biome.getGrassColor`/`getFoliageColor` averaged over the nine columns
    /// around the block (`GrassBlock.getColor`'s 3x3 loop).
    fn era_tint(&self, tint: Tint, x: i32, y: i32, z: i32) -> i32 {
        let mut sum = [0_i32; 3];
        let mut count = 0_i32;
        for dz in -1..=1 {
            for dx in -1..=1 {
                let biome = Tints::biome_id(
                    &self.tints.zoom,
                    self.tints.region,
                    self.tints.plains,
                    self.tints.origin,
                    [x + dx, y, z + dz],
                );
                let info = self.biomes.get(biome);
                // `Biome.getGrassColor` clamps both to 0..1 first.
                let temperature = f64::from(info.temperature).clamp(0.0, 1.0);
                let downfall = f64::from(info.downfall).clamp(0.0, 1.0);
                let color = match tint {
                    Tint::Foliage => self.colormaps.foliage(temperature, downfall),
                    _ => self.colormaps.grass(temperature, downfall),
                };
                sum = [
                    sum[0] + (color >> 16 & 0xFF),
                    sum[1] + (color >> 8 & 0xFF),
                    sum[2] + (color & 0xFF),
                ];
                count += 1;
            }
        }
        ((sum[0] / count) & 0xFF) << 16 | ((sum[1] / count) & 0xFF) << 8 | (sum[2] / count) & 0xFF
    }
}

impl WorldView for SectionWorld<'_> {
    fn state(&self, x: i32, y: i32, z: i32) -> &State17 {
        self.table.get(self.pad.blocks[pi(x, y, z)] as u16)
    }

    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32 {
        i32::from(self.pad.light[pi(x, y, z)] >> 4)
    }

    fn block_light(&self, x: i32, y: i32, z: i32) -> i32 {
        i32::from(self.pad.light[pi(x, y, z)] & 15)
    }

    fn biome_tint(&self, tint: Tint, x: i32, y: i32, z: i32) -> i32 {
        self.era_tint(tint, x, y, z)
    }
}

/// Everything one section's era mesh reads.
pub(super) struct MeshInput<'a> {
    pub pad: &'a Padded,
    pub table: &'a StateTable17,
    pub render: &'a RenderAssets,
    pub biomes: &'a Biomes,
    pub tints: &'a Tints<'a>,
    /// World x of section-local x = 0, the section's lowest world y, and world z
    /// of section-local z = 0.
    pub origin: [i32; 3],
    pub options: RenderOptions,
}

/// Mesh one section of a 1.7.10 world into `out` (the era path).
pub(super) fn mesh(input: MeshInput<'_>, out: &mut SectionMesh) {
    let MeshInput {
        pad,
        table,
        render,
        biomes,
        tints,
        origin,
        options,
    } = input;
    let Some(era) = &render.era else { return };
    let world = SectionWorld {
        pad,
        table,
        biomes,
        colormaps: &era.colormaps,
        tints,
    };
    let mut renderer = RenderBlocks::new(&world, options);
    for y in 0..16 {
        for z in 0..16 {
            let row = pi(0, y, z);
            for x in 0..16 {
                let cell = row + x as usize;
                let id = pad.blocks[cell];
                if id == 0 {
                    continue;
                }
                let state = table.get(id as u16);
                if state.block.kind == blocks::Kind::Air {
                    continue;
                }
                let at = [origin[0] + x, origin[1] + y, origin[2] + z];
                if state.light > 0 {
                    out.lights.push((at, id));
                }
                renderer.tesselator.clear();
                renderer.tesselate_in_world(&state.block, x, y, z);
                let translucent = state.block.translucent_pass;
                for quad in renderer.tesselator.vertices.as_chunks::<4>().0 {
                    out.note_sprite_layer(quad[0].layer);
                    let mut corners = [TerrainVertex::packed_uv([0.0; 3], [0, 0], 0, 0, 0); 4];
                    for (corner, vertex) in corners.iter_mut().zip(quad) {
                        *corner = TerrainVertex::packed_uv(
                            [vertex.x, vertex.y, vertex.z],
                            [
                                round_to_u16(vertex.u * UV_STEPS),
                                round_to_u16(vertex.v * UV_STEPS),
                            ],
                            vertex.layer & !OPAQUE_LAYER,
                            vertex.color as u32,
                            ((vertex.brightness >> 16 & 0xFF) as u16)
                                | ((vertex.brightness & 0xFF) as u16) << 8,
                        );
                    }
                    if translucent {
                        out.translucent.extend(corners);
                    } else {
                        out.solid.extend(corners);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Properties;
    use sunrise_core::profile::Profile;
    use sunrise_items::blocks::BlockFacts;
    use sunrise_world::registry::BlockRegistry;

    fn registry(json: &str) -> &'static BlockRegistry {
        Box::leak(Box::new(
            BlockRegistry::from_facts_json(json.as_bytes()).expect("a block registry"),
        ))
    }

    fn assets(registry: &'static BlockRegistry, profile: Profile) -> Assets {
        Assets {
            registry,
            profile,
            properties: Properties::for_tests(&[(false, false, true, false)]),
            material_states: [BlockStateId(0); 6],
            blocks_fluid_flow: Default::default(),
        }
    }

    /// The era mesher runs only under the era profile; the registry's shape
    /// does not select it on its own.
    #[test]
    fn the_era_mesher_runs_for_the_era_profile_only() {
        let era = registry(
            r#"{"defaults":{"minecraft:stone":0},"states":[
                ["stone",false,false,"",{"id":"minecraft:stone","properties":{"meta":"0"}}]]}"#,
        );
        assert_eq!(assets(era, Profile::V1_7_10).profile, Profile::V1_7_10);
        assert_eq!(assets(era, Profile::Modern).profile, Profile::Modern);
    }

    /// The prepared 1.7.10 folder, when this machine has one (CI does not).
    fn era_root() -> Option<std::path::PathBuf> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(crate::sunset17_assets::DEFAULT_DIR);
        (Profile::of_resource_root(&root).ok() == Some(Profile::V1_7_10)).then_some(root)
    }

    /// The real content export's every state finds the era block, id and six
    /// sprites: the production path over the prepared assets.
    #[test]
    fn the_eras_prepared_root_resolves_every_state() {
        let Some(root) = era_root() else { return };
        // The era content export is the registry; the prepared root has no
        // 26.3 `sunrise-resources.json`.
        let registry = Box::leak(Box::new(
            BlockRegistry::from_facts_json(
                std::fs::read(root.join("facts.json")).unwrap().as_slice(),
            )
            .unwrap(),
        ));
        let facts = BlockFacts::load(&root, registry).unwrap();
        let mut properties = vec![(false, false, true, false); registry.len()];
        properties[0] = (true, false, false, false);
        let assets = Assets {
            registry,
            profile: Profile::V1_7_10,
            properties: Properties::for_tests(&properties),
            material_states: [BlockStateId(0); 6],
            blocks_fluid_flow: Default::default(),
        };
        let assets = &assets;
        let source = crate::resource_source::ResourceRoot::directory(root);
        let render = RenderAssets::load_1_7_10(&source).unwrap();
        let emission: Vec<u8> = facts.states.iter().map(|state| state.emission).collect();
        let table = StateTable17::era(assets, &render, &emission).unwrap();
        assert_eq!(table.states.len(), assets.registry.len());
        let layers = render.texture_layers as u16;
        for (id, state) in table.states.iter().enumerate() {
            let name = &assets.registry.state(BlockStateId(id as u32)).unwrap().name;
            if name == "minecraft:air" {
                continue;
            }
            // Fire and the end portal draw nothing through 1.7.10's block
            // renderer; every other block must have found its era facts.
            if matches!(name.as_str(), "minecraft:fire" | "minecraft:end_portal") {
                assert_eq!(state.block.kind, blocks::Kind::Air, "state {id} ({name})");
                continue;
            }
            assert_ne!(
                state.block.kind,
                blocks::Kind::Air,
                "state {id} ({name}) has no era block"
            );
            for (face, sprite) in state.sprites.iter().enumerate() {
                assert!(
                    sprite.layer < layers,
                    "state {id} ({name}) face {face} has no sprite layer"
                );
            }
        }
        // The era's own lookups: stone's sprite, grass's overlay, the fern's
        // metadata variant and water's still/flow pair.
        let layer = |name: &str| render.sprite_layers[name] as u16;
        let stone = table
            .get(super::fixture::id("stone"))
            .sprites
            .iter()
            .map(|sprite| sprite.layer)
            .collect::<Vec<_>>();
        assert!(stone.iter().all(|&l| l == layer("minecraft:blocks/stone")));
        let grass = assets.registry.default_state("minecraft:grass").unwrap();
        assert!(table.get(grass.0 as u16).overlay.is_some());
        let fern = assets
            .registry
            .id(&sunrise_world::chunk::BlockState {
                name: "minecraft:tallgrass".to_owned(),
                properties: [("meta".to_owned(), "2".to_owned())].into_iter().collect(),
            })
            .unwrap();
        assert_eq!(
            table.get(fern.0 as u16).sprites[2].layer,
            layer("minecraft:blocks/fern")
        );
        let water = assets.registry.default_state("minecraft:water").unwrap();
        assert_eq!(
            table.get(water.0 as u16).sprites[1].layer,
            layer("minecraft:blocks/water_still")
        );
        // The era colormaps at plains' temperature and downfall: the grass
        // green 1.7.10's own plains use (`RenderOracle`'s flat tint).
        let colormaps = &render.era.as_ref().unwrap().colormaps;
        assert_eq!(colormaps.grass(0.8, 0.4), 0x91_BD59);
        assert_eq!(colormaps.foliage(0.8, 0.4), 0x77_AB2F);
    }

    /// The era table asks the content export's rows for the numeric id and the
    /// era's sprite table for each face, and reads the metadata property.
    #[test]
    fn the_era_table_maps_names_to_ids_and_sprites() {
        let registry = registry(
            r#"{"defaults":{"minecraft:air":0,"minecraft:stone_slab":1},"states":[
                ["air",true,false,"","minecraft:air"],
                ["stone_slab",false,false,"",{"id":"minecraft:stone_slab","properties":{"meta":"8"}}]]}"#,
        );
        let render = RenderAssets {
            sprite_size: 16,
            sprite_minimum_size: 16,
            pixels: crate::assets::SpritePixels::new(vec![255; 4]),
            texture_layers: 2,
            model_catalog: crate::models::StateModels::default(),
            model_library: Err("unused in the era".into()),
            model_placements: vec![],
            model_gallery: vec![],
            animations: vec![],
            sprite_layers: std::collections::BTreeMap::from([
                ("minecraft:blocks/stone_slab_top".to_owned(), 5),
                ("minecraft:blocks/stone_slab_side".to_owned(), 6),
                ("minecraft:blocks/grass_side_overlay".to_owned(), 9),
            ]),
            file_layers: Default::default(),
            emissive: vec![],
            translucent_layers: vec![false; 7],
            sprite_mips: vec![],
            pbr_normals: vec![],
            pbr_specular: vec![],
            pbr_size: 16,
            connected: Default::default(),
            fusion: Default::default(),
            mod_animations: Default::default(),
            mod_pipelines: Default::default(),
            generation: 1,
            era: Some(EraRenderAssets {
                blocks: vec![crate::sunset17_assets::BlockRow {
                    id: 44,
                    name: "minecraft:stone_slab".to_owned(),
                    sprites: vec!["stone_slab_top".to_owned(), "stone_slab_side".to_owned()],
                    textureless: false,
                }],
                overlay_layer: Some(9),
                colormaps: Colormaps::default(),
            }),
        };
        let table =
            StateTable17::era(&assets(registry, Profile::V1_7_10), &render, &[0, 0]).unwrap();
        let slab = table.get(1);
        assert_eq!(slab.block.kind, blocks::Kind::Slab { double: false });
        assert_eq!(slab.meta, 8);
        // StoneSlabBlock.getSprite: `stone_slab_top` on both horizontal faces,
        // `stone_slab_side` elsewhere.
        assert_eq!(slab.sprites[1].layer, 5);
        assert_eq!(slab.sprites[2].layer, 6);
        assert_eq!(slab.sprites[0].layer, 5);
    }

    #[test]
    fn colormap_sampling_follows_the_eras_index() {
        let mut colors = vec![0u32; 65536];
        // `GrassColors.getColor(0.5, 1.0)`: humidity * temperature = 0.5, so
        // u = (1 - 0.5) * 255 = 127 and v = 127.
        colors[127 << 8 | 127] = 0x00_91_BD_59;
        let colormaps = Colormaps {
            grass: colors.clone(),
            foliage: colors,
        };
        assert_eq!(colormaps.grass(0.5, 1.0), 0x91_BD_59);
        // The humidity is multiplied by the temperature first:
        // (1 - 0.5 * 0.5) * 255 = 191.
        assert_eq!(colormaps.grass(0.5, 0.5), 0);
    }

    #[test]
    fn the_fixture_table_is_air_first_and_finds_its_pairs() {
        let table = fixture::states();
        assert_eq!(table.get(0).block.kind, blocks::Kind::Air);
        let glass = table.state_of(20, 0);
        assert_eq!(table.get(glass).block.kind, blocks::Kind::Glass);
        assert_eq!(table.get(table.state_of(17, 4)).meta, 4);
    }

    #[test]
    fn the_fixture_uses_the_oracles_atlas_cells() {
        let table = fixture::states();
        let stone = table.get(table.state_of(1, 0));
        assert_eq!(
            stone.sprites[2].u_min(),
            sprite::Sprite::atlas(0, 0, 0, 256, 256).u_min()
        );
        let grass = table.get(table.state_of(2, 0));
        assert!(grass.sprites[2].grass_side);
        assert!(grass.overlay.is_some());
        // Grass's top is 0x91BD59 once tinted in the fixture's world.
        assert!(table.get(table.state_of(2, 0)).block.top_is_grass_top);
    }
}
