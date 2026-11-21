//! Minecraft 1.7.10 biome generation, ported from the vanilla layer chain.
//! Seed mixing and RNG are verbatim so output matches the Java game bit for bit.

// The layer arithmetic below is transcribed from vanilla's `GenLayer`
// subclasses, whose index sums (``i + 0 + (j + 0) * width`` and friends) are
// kept verbatim so the port can be diffed against the Java line by line.
#![allow(clippy::identity_op)]

const A: i64 = 6_364_136_223_846_793_005;
const B: i64 = 1_442_695_040_888_963_407;

const MESA_PLATEAU_F: i32 = 38;
const MESA_PLATEAU: i32 = 39;
const OCEAN: i32 = 0;
const DEEP_OCEAN: i32 = 24;
const FROZEN_OCEAN: i32 = 10;
const RIVER: i32 = 7;
const FROZEN_RIVER: i32 = 11;
const ICE_PLAINS: i32 = 12;
const MUSHROOM_ISLAND: i32 = 14;
const MUSHROOM_ISLAND_SHORE: i32 = 15;
const PLAINS: i32 = 1;
const DESERT: i32 = 2;
const EXTREME_HILLS: i32 = 3;
const EXTREME_HILLS_EDGE: i32 = 20;
const EXTREME_HILLS_PLUS: i32 = 34;
const FOREST: i32 = 4;
const FOREST_HILLS: i32 = 18;
const BIRCH_FOREST: i32 = 27;
const BIRCH_FOREST_HILLS: i32 = 28;
const ROOFED_FOREST: i32 = 29;
const TAIGA: i32 = 5;
const TAIGA_HILLS: i32 = 19;
const MEGA_TAIGA: i32 = 32;
const MEGA_TAIGA_HILLS: i32 = 33;
const COLD_TAIGA: i32 = 30;
const COLD_TAIGA_HILLS: i32 = 31;
const SWAMPLAND: i32 = 6;
const JUNGLE: i32 = 21;
const JUNGLE_HILLS: i32 = 22;
const JUNGLE_EDGE: i32 = 23;
const SAVANNA: i32 = 35;
const SAVANNA_PLATEAU: i32 = 36;
const MESA: i32 = 37;
const BEACH: i32 = 16;
const STONE_BEACH: i32 = 25;
const COLD_BEACH: i32 = 26;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BiomeType {
    Ocean,
    Plains,
    Desert,
    ExtremeHills,
    Forest,
    Taiga,
    Swamp,
    River,
    Hell,
    End,
    Ice,
    Mushroom,
    Beach,
    Jungle,
    StoneBeach,
    Savanna,
    Mesa,
    None,
}

fn biome_type(id: i32) -> BiomeType {
    match id {
        0 | 10 | 24 => BiomeType::Ocean,
        1 => BiomeType::Plains,
        2 | 17 => BiomeType::Desert,
        3 | 20 | 34 => BiomeType::ExtremeHills,
        4 | 18 | 27 | 28 | 29 => BiomeType::Forest,
        5 | 19 | 30 | 31 | 32 | 33 => BiomeType::Taiga,
        6 => BiomeType::Swamp,
        7 | 11 => BiomeType::River,
        8 => BiomeType::Hell,
        9 => BiomeType::End,
        12 | 13 => BiomeType::Ice,
        14 | 15 => BiomeType::Mushroom,
        16 | 26 => BiomeType::Beach,
        21..=23 => BiomeType::Jungle,
        25 => BiomeType::StoneBeach,
        35 | 36 => BiomeType::Savanna,
        37..=39 => BiomeType::Mesa,
        _ => BiomeType::None,
    }
}

fn valid(id: i32) -> bool {
    (0..40).contains(&id)
}

/// `Biome.byId(id + 128) != null`: the base biomes whose static block calls
/// `mutate()`, so `RegionHillsLayer` may produce the mutated id.
fn has_mutation(id: i32) -> bool {
    matches!(
        id,
        1 | 2
            | 3
            | 4
            | 5
            | 6
            | 12
            | 21
            | 23
            | 27
            | 28
            | 29
            | 30
            | 32
            | 33
            | 34
            | 35
            | 36
            | 37
            | 38
            | 39
    )
}

fn snowy(id: i32) -> bool {
    matches!(id, 10 | 11 | 12 | 13 | 26 | 30 | 31)
}

fn temperature(id: i32) -> f32 {
    match id {
        2 | 17 => 2.0,
        3 | 20 | 34 | 25 => 0.2,
        5 | 19 => 0.25,
        6 => 0.8,
        8 => 2.0,
        10..=13 => 0.0,
        14 | 15 => 0.9,
        16 => 0.8,
        21..=23 => 0.95,
        26 => 0.05,
        30 | 31 => -0.5,
        32 | 33 => 0.3,
        35 => 1.2,
        36 => 1.0,
        _ => 0.5,
    }
}

fn temperature_category(id: i32) -> u8 {
    let t = temperature(id);
    if t < 0.2 {
        0
    } else if t < 1.0 {
        1
    } else {
        2
    }
}

fn registered(id: i32) -> bool {
    valid(id) || (128..168).contains(&id)
}

/// `MutatedBiome.getType()`: a mutated id compares as the biome it mutates.
fn base_of(id: i32) -> i32 {
    if id >= 128 { id - 128 } else { id }
}

fn is_same(a: i32, b: i32) -> bool {
    if a == b {
        return true;
    }
    if a != MESA_PLATEAU_F && a != MESA_PLATEAU {
        return registered(a)
            && registered(b)
            && biome_type(base_of(a)) == biome_type(base_of(b))
            && biome_type(base_of(a)) != BiomeType::None;
    }
    b == MESA_PLATEAU_F || b == MESA_PLATEAU
}

fn is_ocean(id: i32) -> bool {
    id == OCEAN || id == DEEP_OCEAN || id == FROZEN_OCEAN
}

fn valid_temp_edge(biome: i32, edge: i32) -> bool {
    if is_same(biome, edge) {
        return true;
    }
    if valid(biome) && valid(edge) {
        let a = temperature_category(biome);
        let b = temperature_category(edge);
        return a == b || a == 1 || b == 1;
    }
    false
}

#[derive(Clone, Copy)]
enum EdgeMode {
    CoolWarm,
    HeatIce,
    Special,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GenType {
    Default,
    LargeBiomes,
    Default11,
}

#[derive(Clone, Copy)]
enum Kind {
    #[cfg(test)]
    Fixed,
    Island,
    FuzzyZoom,
    NormalZoom,
    AddIsland,
    RemoveTooMuchOcean,
    AddSnow,
    AddEdge(EdgeMode),
    AddMushroomIsland,
    AddDeepOcean,
    BiomeInit(GenType),
    BiomeEdge,
    RegionHills,
    RiverInit,
    River,
    Smooth,
    Shore,
    AddSunflowerPlains,
    OceanRiverMixer,
    VoronoiZoom,
}

#[derive(Clone)]
pub struct Layer {
    #[cfg(test)]
    fixed: Option<Vec<i32>>,
    seed: i64,
    local: i64,
    chunk: i64,
    kind: Kind,
    parent: Option<Box<Layer>>,
    second: Option<Box<Layer>>,
}

impl Layer {
    fn new(seed: i64, kind: Kind) -> Self {
        let mut s = seed;
        for _ in 0..3 {
            s = s
                .wrapping_mul(s.wrapping_mul(A).wrapping_add(B))
                .wrapping_add(seed);
        }
        Self {
            #[cfg(test)]
            fixed: None,
            seed: s,
            local: 0,
            chunk: 0,
            kind,
            parent: None,
            second: None,
        }
    }

    fn parent(mut self, p: Layer) -> Self {
        self.parent = Some(Box::new(p));
        self
    }

    fn second(mut self, p: Layer) -> Self {
        self.second = Some(Box::new(p));
        self
    }

    fn set_local_world_seed(&mut self, world: i64) {
        if let Some(p) = self.parent.as_mut() {
            p.set_local_world_seed(world);
        }
        // Only OceanRiverMixerLayer overrides setLocalWorldSeed to reach its riverLayer;
        // RegionHillsLayer's riverLayer keeps localWorldSeed 0 in vanilla.
        let reach_river_layer = matches!(self.kind, Kind::OceanRiverMixer);
        let second = self.second.as_mut();
        if let (true, Some(s)) = (reach_river_layer, second) {
            s.set_local_world_seed(world);
        }
        let mut l = world;
        for _ in 0..3 {
            l = l
                .wrapping_mul(l.wrapping_mul(A).wrapping_add(B))
                .wrapping_add(self.seed);
        }
        self.local = l;
    }

    fn set_local_noprop(&mut self, world: i64) {
        let mut l = world;
        for _ in 0..3 {
            l = l
                .wrapping_mul(l.wrapping_mul(A).wrapping_add(B))
                .wrapping_add(self.seed);
        }
        self.local = l;
    }

    fn set_chunk_seed(&mut self, x: i32, z: i32) {
        let mut c = self.local;
        c = c
            .wrapping_mul(c.wrapping_mul(A).wrapping_add(B))
            .wrapping_add(x as i64);
        c = c
            .wrapping_mul(c.wrapping_mul(A).wrapping_add(B))
            .wrapping_add(z as i64);
        c = c
            .wrapping_mul(c.wrapping_mul(A).wrapping_add(B))
            .wrapping_add(x as i64);
        c = c
            .wrapping_mul(c.wrapping_mul(A).wrapping_add(B))
            .wrapping_add(z as i64);
        self.chunk = c;
    }

    fn next_int(&mut self, bound: i32) -> i32 {
        let mut v = ((self.chunk >> 24).wrapping_rem(bound as i64)) as i32;
        if v < 0 {
            v += bound;
        }
        self.chunk = self
            .chunk
            .wrapping_mul(self.chunk.wrapping_mul(A).wrapping_add(B))
            .wrapping_add(self.local);
        v
    }

    fn pick(&mut self, values: &[i32]) -> i32 {
        values[self.next_int(values.len() as i32) as usize]
    }

    fn mode_or_random(&mut self, i1: i32, i2: i32, i3: i32, i4: i32) -> i32 {
        if i2 == i3 && i3 == i4 {
            return i2;
        }
        if i1 == i2 && i1 == i3 {
            return i1;
        }
        if i1 == i2 && i1 == i4 {
            return i1;
        }
        if i1 == i3 && i1 == i4 {
            return i1;
        }
        if i1 == i2 && i3 != i4 {
            return i1;
        }
        if i1 == i3 && i2 != i4 {
            return i1;
        }
        if i1 == i4 && i2 != i3 {
            return i1;
        }
        if i2 == i3 && i1 != i4 {
            return i2;
        }
        if i2 == i4 && i1 != i3 {
            return i2;
        }
        if i3 == i4 && i1 != i2 {
            return i3;
        }
        self.pick(&[i1, i2, i3, i4])
    }

    pub fn area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let kind = self.kind;
        match kind {
            #[cfg(test)]
            Kind::Fixed => self.fixed.clone().unwrap_or_default(),
            Kind::Island => self.island(x, z, w, h),
            Kind::FuzzyZoom => self.zoom(x, z, w, h, true),
            Kind::NormalZoom => self.zoom(x, z, w, h, false),
            Kind::AddIsland => self.add_island(x, z, w, h),
            Kind::RemoveTooMuchOcean => self.remove_ocean(x, z, w, h),
            Kind::AddSnow => self.add_snow(x, z, w, h),
            Kind::AddEdge(mode) => self.add_edge(x, z, w, h, mode),
            Kind::AddMushroomIsland => self.add_mushroom(x, z, w, h),
            Kind::AddDeepOcean => self.add_deep_ocean(x, z, w, h),
            Kind::BiomeInit(gtype) => self.biome_init(x, z, w, h, gtype),
            Kind::BiomeEdge => self.biome_edge(x, z, w, h),
            Kind::RegionHills => self.region_hills(x, z, w, h),
            Kind::RiverInit => self.river_init(x, z, w, h),
            Kind::River => self.river(x, z, w, h),
            Kind::Smooth => self.smooth(x, z, w, h),
            Kind::Shore => self.shore(x, z, w, h),
            Kind::AddSunflowerPlains => self.sunflower(x, z, w, h),
            Kind::OceanRiverMixer => self.mixer(x, z, w, h),
            Kind::VoronoiZoom => self.voronoi(x, z, w, h),
        }
    }

    fn parent_area(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        self.parent
            .as_mut()
            .expect("layer has no parent")
            .area(x, z, w, h)
    }

    fn island(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(x + i, z + j);
                out[(i + j * w) as usize] = if self.next_int(10) == 0 { 1 } else { 0 };
            }
        }
        if x > -w && x <= 0 && z > -h && z <= 0 {
            out[(-x + -z * w) as usize] = 1;
        }
        out
    }

    fn zoom(&mut self, x: i32, z: i32, w: i32, h: i32, fuzzy: bool) -> Vec<i32> {
        let px = x >> 1;
        let pz = z >> 1;
        let pw = (w >> 1) + 2;
        let ph = (h >> 1) + 2;
        let parent = self.parent_area(px, pz, pw, ph);
        let bw = (pw - 1) << 1;
        let bh = (ph - 1) << 1;
        let mut big = vec![0; (bw * bh) as usize];
        for j in 0..ph - 1 {
            let row = (j << 1) * bw;
            let mut a = parent[(0 + (j + 0) * pw) as usize];
            let mut b = parent[(0 + (j + 1) * pw) as usize];
            let mut i = 0;
            while i < pw - 1 {
                self.set_chunk_seed((i + px) << 1, (j + pz) << 1);
                let c = parent[(i + 1 + (j + 0) * pw) as usize];
                let d = parent[(i + 1 + (j + 1) * pw) as usize];
                big[(row + i * 2) as usize] = a;
                big[(row + i * 2 + bw) as usize] = self.pick(&[a, b]);
                big[(row + i * 2 + 1) as usize] = self.pick(&[a, c]);
                big[(row + i * 2 + 1 + bw) as usize] = if fuzzy {
                    self.pick(&[a, c, b, d])
                } else {
                    self.mode_or_random(a, c, b, d)
                };
                a = c;
                b = d;
                i += 1;
            }
        }
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            let src = ((j + (z & 1)) * bw + (x & 1)) as usize;
            out[(j * w) as usize..((j + 1) * w) as usize]
                .copy_from_slice(&big[src..src + w as usize]);
        }
        out
    }

    fn add_island(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let n = parent[(i + 0 + (j + 0) * pw) as usize];
                let e = parent[(i + 2 + (j + 0) * pw) as usize];
                let s = parent[(i + 0 + (j + 2) * pw) as usize];
                let wn = parent[(i + 2 + (j + 2) * pw) as usize];
                let c = parent[(i + 1 + (j + 1) * pw) as usize];
                self.set_chunk_seed(i + x, j + z);
                let idx = (i + j * w) as usize;
                if c != 0 || (n == 0 && e == 0 && s == 0 && wn == 0) {
                    if c > 0 && (n == 0 || e == 0 || s == 0 || wn == 0) {
                        if self.next_int(5) == 0 {
                            out[idx] = if c == 4 { 4 } else { 0 };
                        } else {
                            out[idx] = c;
                        }
                    } else {
                        out[idx] = c;
                    }
                } else {
                    let mut chance = 1;
                    let mut chosen = 1;
                    // Java: nextInt(var18++) increments the bound on every nonzero neighbour,
                    // whether or not the draw was zero.
                    if n != 0 {
                        let r = self.next_int(chance);
                        chance += 1;
                        if r == 0 {
                            chosen = n;
                        }
                    }
                    if e != 0 {
                        let r = self.next_int(chance);
                        chance += 1;
                        if r == 0 {
                            chosen = e;
                        }
                    }
                    if s != 0 {
                        let r = self.next_int(chance);
                        chance += 1;
                        if r == 0 {
                            chosen = s;
                        }
                    }
                    if wn != 0 {
                        let r = self.next_int(chance);
                        if r == 0 {
                            chosen = wn;
                        }
                    }
                    if self.next_int(3) == 0 {
                        out[idx] = chosen;
                    } else if chosen == 4 {
                        out[idx] = 4;
                    } else {
                        out[idx] = 0;
                    }
                }
            }
        }
        out
    }

    fn remove_ocean(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let n = parent[(i + 1 + (j + 0) * pw) as usize];
                let e = parent[(i + 2 + (j + 1) * pw) as usize];
                let s = parent[(i + 0 + (j + 1) * pw) as usize];
                let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                let c = parent[(i + 1 + (j + 1) * pw) as usize];
                let idx = (i + j * w) as usize;
                out[idx] = c;
                self.set_chunk_seed(i + x, j + z);
                if c == 0 && n == 0 && e == 0 && s == 0 && wn == 0 && self.next_int(2) == 0 {
                    out[idx] = 1;
                }
            }
        }
        out
    }

    fn add_snow(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let c = parent[(i + 1 + (j + 1) * pw) as usize];
                self.set_chunk_seed(i + x, j + z);
                if c == 0 {
                    out[(i + j * w) as usize] = 0;
                } else {
                    let r = self.next_int(6);
                    out[(i + j * w) as usize] = if r == 0 {
                        4
                    } else if r <= 1 {
                        3
                    } else {
                        1
                    };
                }
            }
        }
        out
    }

    fn add_edge(&mut self, x: i32, z: i32, w: i32, h: i32, mode: EdgeMode) -> Vec<i32> {
        let mut out = vec![0; (w * h) as usize];
        match mode {
            EdgeMode::Special => {
                let parent = self.parent_area(x, z, w, h);
                for j in 0..h {
                    for i in 0..w {
                        self.set_chunk_seed(i + x, j + z);
                        let mut v = parent[(i + j * w) as usize];
                        if v != 0 && self.next_int(13) == 0 {
                            v |= (1 + self.next_int(15)) << 8 & 3840;
                        }
                        out[(i + j * w) as usize] = v;
                    }
                }
            }
            EdgeMode::CoolWarm | EdgeMode::HeatIce => {
                let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
                let pw = w + 2;
                for j in 0..h {
                    for i in 0..w {
                        self.set_chunk_seed(i + x, j + z);
                        let mut v = parent[(i + 1 + (j + 1) * pw) as usize];
                        let n = parent[(i + 1 + (j + 0) * pw) as usize];
                        let e = parent[(i + 2 + (j + 1) * pw) as usize];
                        let s = parent[(i + 0 + (j + 1) * pw) as usize];
                        let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                        match mode {
                            EdgeMode::CoolWarm => {
                                if v == 1
                                    && ((n == 3 || e == 3 || s == 3 || wn == 3)
                                        || (n == 4 || e == 4 || s == 4 || wn == 4))
                                {
                                    v = 2;
                                }
                            }
                            EdgeMode::HeatIce => {
                                if v == 4
                                    && ((n == 2 || e == 2 || s == 2 || wn == 2)
                                        || (n == 1 || e == 1 || s == 1 || wn == 1))
                                {
                                    v = 3;
                                }
                            }
                            EdgeMode::Special => unreachable!(),
                        }
                        out[(i + j * w) as usize] = v;
                    }
                }
            }
        }
        out
    }

    fn add_mushroom(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let n = parent[(i + 0 + (j + 0) * pw) as usize];
                let e = parent[(i + 2 + (j + 0) * pw) as usize];
                let s = parent[(i + 0 + (j + 2) * pw) as usize];
                let wn = parent[(i + 2 + (j + 2) * pw) as usize];
                let c = parent[(i + 1 + (j + 1) * pw) as usize];
                self.set_chunk_seed(i + x, j + z);
                out[(i + j * w) as usize] =
                    if c == 0 && n == 0 && e == 0 && s == 0 && wn == 0 && self.next_int(100) == 0 {
                        MUSHROOM_ISLAND
                    } else {
                        c
                    };
            }
        }
        out
    }

    fn add_deep_ocean(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let n = parent[(i + 1 + (j + 0) * pw) as usize];
                let e = parent[(i + 2 + (j + 1) * pw) as usize];
                let s = parent[(i + 0 + (j + 1) * pw) as usize];
                let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                let c = parent[(i + 1 + (j + 1) * pw) as usize];
                let count = [n, e, s, wn].iter().filter(|&&v| v == 0).count() as i32;
                out[(i + j * w) as usize] = if c == 0 && count > 3 { DEEP_OCEAN } else { c };
            }
        }
        out
    }

    fn biome_init(&mut self, x: i32, z: i32, w: i32, h: i32, gtype: GenType) -> Vec<i32> {
        let parent = self.parent_area(x, z, w, h);
        let warm: &[i32] = if gtype == GenType::Default11 {
            &[2, 4, 3, 6, 1, 5]
        } else {
            &[2, 2, 2, 35, 35, 1]
        };
        let temperate: &[i32] = &[4, 29, 3, 1, 27, 6];
        let cool: &[i32] = &[4, 3, 5, 1];
        let snowy: &[i32] = &[12, 12, 12, 30];
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(i + x, j + z);
                let mut v = parent[(i + j * w) as usize];
                let variant = (v & 3840) >> 8;
                v &= -3841;
                out[(i + j * w) as usize] = if is_ocean(v) || v == MUSHROOM_ISLAND {
                    v
                } else if v == 1 {
                    if variant > 0 {
                        if self.next_int(3) == 0 {
                            MESA_PLATEAU
                        } else {
                            MESA_PLATEAU_F
                        }
                    } else {
                        warm[self.next_int(warm.len() as i32) as usize]
                    }
                } else if v == 2 {
                    if variant > 0 {
                        JUNGLE
                    } else {
                        temperate[self.next_int(temperate.len() as i32) as usize]
                    }
                } else if v == 3 {
                    if variant > 0 {
                        MEGA_TAIGA
                    } else {
                        cool[self.next_int(cool.len() as i32) as usize]
                    }
                } else if v == 4 {
                    snowy[self.next_int(snowy.len() as i32) as usize]
                } else {
                    MUSHROOM_ISLAND
                };
            }
        }
        out
    }

    fn biome_edge(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(i + x, j + z);
                let v = parent[(i + 1 + (j + 1) * pw) as usize];
                let n = parent[(i + 1 + (j + 0) * pw) as usize];
                let e = parent[(i + 2 + (j + 1) * pw) as usize];
                let s = parent[(i + 0 + (j + 1) * pw) as usize];
                let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                let idx = (i + j * w) as usize;
                let set = if is_same(v, EXTREME_HILLS) {
                    out[idx] = if valid_temp_edge(n, EXTREME_HILLS)
                        && valid_temp_edge(e, EXTREME_HILLS)
                        && valid_temp_edge(s, EXTREME_HILLS)
                        && valid_temp_edge(wn, EXTREME_HILLS)
                    {
                        v
                    } else {
                        EXTREME_HILLS_EDGE
                    };
                    true
                } else if v == MESA_PLATEAU_F || v == MESA_PLATEAU {
                    out[idx] = if is_same(n, v) && is_same(e, v) && is_same(s, v) && is_same(wn, v)
                    {
                        v
                    } else {
                        MESA
                    };
                    true
                } else if v == MEGA_TAIGA {
                    out[idx] = if is_same(n, v) && is_same(e, v) && is_same(s, v) && is_same(wn, v)
                    {
                        v
                    } else {
                        TAIGA
                    };
                    true
                } else {
                    false
                };
                if set {
                    continue;
                }
                if v == DESERT {
                    out[idx] = if n == ICE_PLAINS
                        || e == ICE_PLAINS
                        || s == ICE_PLAINS
                        || wn == ICE_PLAINS
                    {
                        EXTREME_HILLS_PLUS
                    } else {
                        v
                    };
                } else if v == SWAMPLAND {
                    if n == DESERT
                        || e == DESERT
                        || s == DESERT
                        || wn == DESERT
                        || n == COLD_TAIGA
                        || e == COLD_TAIGA
                        || s == COLD_TAIGA
                        || wn == COLD_TAIGA
                        || n == ICE_PLAINS
                        || e == ICE_PLAINS
                        || s == ICE_PLAINS
                        || wn == ICE_PLAINS
                    {
                        out[idx] = PLAINS;
                    } else if e != JUNGLE
                        && e != JUNGLE_HILLS
                        && e != JUNGLE_EDGE
                        && s != JUNGLE
                        && wn != JUNGLE
                        && n != JUNGLE
                    {
                        out[idx] = v;
                    } else {
                        out[idx] = JUNGLE_EDGE;
                    }
                } else {
                    out[idx] = v;
                }
            }
        }
        out
    }

    fn region_hills(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent =
            self.parent
                .as_mut()
                .expect("region hills parent")
                .area(x - 1, z - 1, w + 2, h + 2);
        let river =
            self.second
                .as_mut()
                .expect("region hills river")
                .area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(i + x, j + z);
                let v = parent[(i + 1 + (j + 1) * pw) as usize];
                let rv = river[(i + 1 + (j + 1) * pw) as usize];
                let is_river_edge = rv >= 2 && (rv - 2) % 29 == 1;
                let is_special = rv >= 2 && (rv - 2) % 29 == 0;
                let idx = (i + j * w) as usize;
                if v != 0 && rv >= 2 && is_river_edge && v < 128 {
                    out[idx] = if has_mutation(v) { v + 128 } else { v };
                } else if self.next_int(3) != 0 && !is_special {
                    out[idx] = v;
                } else {
                    let mut hill = if v == DESERT {
                        17
                    } else if v == FOREST {
                        FOREST_HILLS
                    } else if v == BIRCH_FOREST {
                        BIRCH_FOREST_HILLS
                    } else if v == ROOFED_FOREST {
                        PLAINS
                    } else if v == TAIGA {
                        TAIGA_HILLS
                    } else if v == MEGA_TAIGA {
                        MEGA_TAIGA_HILLS
                    } else if v == COLD_TAIGA {
                        COLD_TAIGA_HILLS
                    } else if v == PLAINS {
                        if self.next_int(3) == 0 {
                            FOREST_HILLS
                        } else {
                            FOREST
                        }
                    } else if v == ICE_PLAINS {
                        13
                    } else if v == JUNGLE {
                        22
                    } else if v == OCEAN {
                        DEEP_OCEAN
                    } else if v == EXTREME_HILLS {
                        EXTREME_HILLS_PLUS
                    } else if v == SAVANNA {
                        SAVANNA_PLATEAU
                    } else if is_same(v, MESA_PLATEAU_F) {
                        MESA
                    } else if v == DEEP_OCEAN && self.next_int(3) == 0 {
                        if self.next_int(2) == 0 {
                            PLAINS
                        } else {
                            FOREST
                        }
                    } else {
                        v
                    };
                    if is_special && hill != v {
                        hill = if has_mutation(hill) { hill + 128 } else { v };
                    }
                    if hill == v {
                        out[idx] = v;
                    } else {
                        let n = parent[(i + 1 + (j + 0) * pw) as usize];
                        let e = parent[(i + 2 + (j + 1) * pw) as usize];
                        let s = parent[(i + 0 + (j + 1) * pw) as usize];
                        let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                        let count = [n, e, s, wn].iter().filter(|&&c| is_same(c, v)).count();
                        out[idx] = if count >= 3 { hill } else { v };
                    }
                }
            }
        }
        out
    }

    fn river_init(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x, z, w, h);
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(i + x, j + z);
                let v = parent[(i + j * w) as usize];
                out[(i + j * w) as usize] = if v > 0 { self.next_int(299_999) + 2 } else { 0 };
            }
        }
        out
    }

    fn river(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        fn curve(v: i32) -> i32 {
            if v >= 2 { 2 + (v & 1) } else { v }
        }
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let n = curve(parent[(i + 0 + (j + 1) * pw) as usize]);
                let e = curve(parent[(i + 2 + (j + 1) * pw) as usize]);
                let s = curve(parent[(i + 1 + (j + 0) * pw) as usize]);
                let wn = curve(parent[(i + 1 + (j + 2) * pw) as usize]);
                let c = curve(parent[(i + 1 + (j + 1) * pw) as usize]);
                out[(i + j * w) as usize] = if c == n && c == s && c == e && c == wn {
                    -1
                } else {
                    RIVER
                };
            }
        }
        out
    }

    fn smooth(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let n = parent[(i + 0 + (j + 1) * pw) as usize];
                let e = parent[(i + 2 + (j + 1) * pw) as usize];
                let s = parent[(i + 1 + (j + 0) * pw) as usize];
                let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                let mut c = parent[(i + 1 + (j + 1) * pw) as usize];
                if n == e && s == wn {
                    self.set_chunk_seed(i + x, j + z);
                    c = if self.next_int(2) == 0 { n } else { s };
                } else {
                    if n == e {
                        c = n;
                    }
                    if s == wn {
                        c = s;
                    }
                }
                out[(i + j * w) as usize] = c;
            }
        }
        out
    }

    fn shore(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        fn jungle_edge(id: i32) -> bool {
            if valid(id) && biome_type(id) == BiomeType::Jungle {
                return true;
            }
            id == JUNGLE_EDGE
                || id == JUNGLE
                || id == JUNGLE_HILLS
                || id == FOREST
                || id == TAIGA
                || is_ocean(id)
        }
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(i + x, j + z);
                let v = parent[(i + 1 + (j + 1) * pw) as usize];
                let n = parent[(i + 1 + (j + 0) * pw) as usize];
                let e = parent[(i + 2 + (j + 1) * pw) as usize];
                let s = parent[(i + 0 + (j + 1) * pw) as usize];
                let wn = parent[(i + 1 + (j + 2) * pw) as usize];
                let idx = (i + j * w) as usize;
                let near = [n, e, s, wn];
                out[idx] = if v == MUSHROOM_ISLAND {
                    if near.contains(&OCEAN) {
                        MUSHROOM_ISLAND_SHORE
                    } else {
                        v
                    }
                } else if valid(v) && biome_type(v) == BiomeType::Jungle {
                    if !near.iter().all(|&b| jungle_edge(b)) {
                        JUNGLE_EDGE
                    } else if near.iter().all(|&b| !is_ocean(b)) {
                        v
                    } else {
                        BEACH
                    }
                } else if v == EXTREME_HILLS || v == EXTREME_HILLS_PLUS || v == EXTREME_HILLS_EDGE {
                    if is_ocean(v) {
                        v
                    } else if near.iter().any(|&b| is_ocean(b)) {
                        STONE_BEACH
                    } else {
                        v
                    }
                } else if valid(v) && snowy(v) {
                    if is_ocean(v) {
                        v
                    } else if near.iter().any(|&b| is_ocean(b)) {
                        COLD_BEACH
                    } else {
                        v
                    }
                } else if v == MESA || v == MESA_PLATEAU_F {
                    if near.iter().any(|&b| is_ocean(b))
                        || near
                            .iter()
                            .all(|&b| valid(b) && biome_type(b) == BiomeType::Mesa)
                    {
                        v
                    } else {
                        DESERT
                    }
                } else if v != OCEAN && v != DEEP_OCEAN && v != RIVER && v != SWAMPLAND {
                    if near.iter().any(|&b| is_ocean(b)) {
                        BEACH
                    } else {
                        v
                    }
                } else {
                    v
                };
            }
        }
        out
    }

    fn sunflower(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let parent = self.parent_area(x - 1, z - 1, w + 2, h + 2);
        let pw = w + 2;
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                self.set_chunk_seed(i + x, j + z);
                let v = parent[(i + 1 + (j + 1) * pw) as usize];
                out[(i + j * w) as usize] = if self.next_int(57) == 0 && v == PLAINS {
                    PLAINS + 128
                } else {
                    v
                };
            }
        }
        out
    }

    fn mixer(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let biome = self
            .parent
            .as_mut()
            .expect("mixer biome parent")
            .area(x, z, w, h);
        let river = self
            .second
            .as_mut()
            .expect("mixer river parent")
            .area(x, z, w, h);
        let mut out = vec![0; (w * h) as usize];
        for i in 0..(w * h) as usize {
            let b = biome[i];
            if b == OCEAN || b == DEEP_OCEAN {
                out[i] = b;
            } else if river[i] == RIVER {
                if b == ICE_PLAINS {
                    out[i] = FROZEN_RIVER;
                } else if b != MUSHROOM_ISLAND && b != MUSHROOM_ISLAND_SHORE {
                    out[i] = river[i] & 0xFF;
                } else {
                    out[i] = MUSHROOM_ISLAND_SHORE;
                }
            } else {
                out[i] = b;
            }
        }
        out
    }

    fn voronoi(&mut self, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let ox = x - 2;
        let oz = z - 2;
        let px = ox >> 2;
        let pz = oz >> 2;
        let pw = (w >> 2) + 2;
        let ph = (h >> 2) + 2;
        let parent = self.parent_area(px, pz, pw, ph);
        let bw = (pw - 1) << 2;
        let bh = (ph - 1) << 2;
        let mut big = vec![0; (bw * bh) as usize];
        for j in 0..ph - 1 {
            let mut i = 0;
            let mut a = parent[(0 + (j + 0) * pw) as usize];
            let mut c = parent[(0 + (j + 1) * pw) as usize];
            while i < pw - 1 {
                self.set_chunk_seed((i + px) << 2, (j + pz) << 2);
                let x0 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                let z0 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                self.set_chunk_seed((i + px + 1) << 2, (j + pz) << 2);
                let x1 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                let z1 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                self.set_chunk_seed((i + px) << 2, (j + pz + 1) << 2);
                let x2 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6;
                let z2 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                self.set_chunk_seed((i + px + 1) << 2, (j + pz + 1) << 2);
                let x3 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                let z3 = (self.next_int(1024) as f64 / 1024.0 - 0.5) * 3.6 + 4.0;
                let b = parent[(i + 1 + (j + 0) * pw) as usize] & 0xFF;
                let d = parent[(i + 1 + (j + 1) * pw) as usize] & 0xFF;
                for dz in 0..4 {
                    for dx in 0..4 {
                        let d0 = (dz as f64 - z0) * (dz as f64 - z0)
                            + (dx as f64 - x0) * (dx as f64 - x0);
                        let d1 = (dz as f64 - z1) * (dz as f64 - z1)
                            + (dx as f64 - x1) * (dx as f64 - x1);
                        let d2 = (dz as f64 - z2) * (dz as f64 - z2)
                            + (dx as f64 - x2) * (dx as f64 - x2);
                        let d3 = (dz as f64 - z3) * (dz as f64 - z3)
                            + (dx as f64 - x3) * (dx as f64 - x3);
                        let val = if d0 < d1 && d0 < d2 && d0 < d3 {
                            a
                        } else if d1 < d0 && d1 < d2 && d1 < d3 {
                            b
                        } else if d2 < d0 && d2 < d1 && d2 < d3 {
                            c
                        } else {
                            d
                        };
                        big[(((j << 2) + dz) * bw + (i << 2) + dx) as usize] = val;
                    }
                }
                a = b;
                c = d;
                i += 1;
            }
        }
        let mut out = vec![0; (w * h) as usize];
        for j in 0..h {
            let src = ((j + (oz & 3)) * bw + (ox & 3)) as usize;
            out[(j * w) as usize..((j + 1) * w) as usize]
                .copy_from_slice(&big[src..src + w as usize]);
        }
        out
    }
}

fn zoom(seed: i64, layer: Layer, n: i32) -> Layer {
    let mut l = layer;
    for i in 0..n {
        l = Layer::new(seed + i as i64, Kind::NormalZoom).parent(l);
    }
    l
}

/// Returns `[noise_biomes, biomes, mixer]` like vanilla's `Layer.init`.
pub fn init(seed: i64, large_biomes: bool) -> [Layer; 3] {
    let gens = if large_biomes {
        GenType::LargeBiomes
    } else {
        GenType::Default
    };
    let mut continents = Layer::new(1, Kind::Island);
    continents = Layer::new(2000, Kind::FuzzyZoom).parent(continents);
    continents = Layer::new(1, Kind::AddIsland).parent(continents);
    continents = zoom(2001, continents, 1);
    continents = Layer::new(2, Kind::AddIsland).parent(continents);
    continents = Layer::new(50, Kind::AddIsland).parent(continents);
    continents = Layer::new(70, Kind::AddIsland).parent(continents);
    continents = Layer::new(2, Kind::RemoveTooMuchOcean).parent(continents);
    continents = Layer::new(2, Kind::AddSnow).parent(continents);
    continents = Layer::new(3, Kind::AddIsland).parent(continents);
    continents = Layer::new(2, Kind::AddEdge(EdgeMode::CoolWarm)).parent(continents);
    continents = Layer::new(2, Kind::AddEdge(EdgeMode::HeatIce)).parent(continents);
    continents = Layer::new(3, Kind::AddEdge(EdgeMode::Special)).parent(continents);
    continents = zoom(2002, continents, 1);
    continents = zoom(2003, continents, 1);
    continents = Layer::new(4, Kind::AddIsland).parent(continents);
    continents = Layer::new(5, Kind::AddMushroomIsland).parent(continents);
    continents = Layer::new(4, Kind::AddDeepOcean).parent(continents);
    continents = zoom(1000, continents, 0);

    let mut noise = continents.clone();
    noise = zoom(1000, noise, 0);
    noise = Layer::new(100, Kind::RiverInit).parent(noise);

    let mut biome = continents;
    biome = Layer::new(200, Kind::BiomeInit(gens)).parent(biome);
    biome = zoom(1000, biome, 2);
    biome = Layer::new(1000, Kind::BiomeEdge).parent(biome);

    let mut hills_base = noise.clone();
    // Java shares the RiverInitLayer subtree between the river branch and RegionHills'
    // riverLayer, so it is seeded through the river branch (twice). The two zoom layers
    // RegionHills adds on top are never seeded, so seed before wrapping.
    hills_base.set_local_world_seed(seed);
    hills_base.set_local_world_seed(seed);
    let river_hills = zoom(1000, hills_base, 2);
    biome = Layer::new(1000, Kind::RegionHills)
        .parent(biome)
        .second(river_hills);

    let mut river = noise;
    river = zoom(1000, river, 2);
    river = zoom(1000, river, if large_biomes { 6 } else { 4 });
    river = Layer::new(1, Kind::River).parent(river);
    river = Layer::new(1000, Kind::Smooth).parent(river);

    biome = Layer::new(1001, Kind::AddSunflowerPlains).parent(biome);
    let scale = if large_biomes { 6 } else { 4 };
    for i in 0..scale {
        biome = Layer::new(1000 + i, Kind::NormalZoom).parent(biome);
        if i == 0 {
            biome = Layer::new(3, Kind::AddIsland).parent(biome);
        }
        if i == 1 {
            biome = Layer::new(1000, Kind::Shore).parent(biome);
        }
    }
    biome = Layer::new(1000, Kind::Smooth).parent(biome);
    let mut mixer = Layer::new(100, Kind::OceanRiverMixer)
        .parent(biome)
        .second(river);
    // Vanilla shares one mixer object as the Voronoi parent; Java's setLocalWorldSeed on the
    // Voronoi node reaches it a second time. ponytail: clone instead of Rc<RefCell> until a
    // caller needs both layers' mutable state in the same run.
    mixer.set_local_world_seed(seed);
    mixer.set_local_world_seed(seed);
    let mut vn = Layer::new(10, Kind::VoronoiZoom).parent(mixer.clone());
    vn.set_local_noprop(seed);
    [mixer, vn, Layer::new(0, Kind::Island)]
}

#[cfg(test)]
mod tests {
    fn fixed(values: Vec<i32>) -> Layer {
        let mut l = Layer::new(0, Kind::Fixed);
        l.fixed = Some(values);
        l
    }

    use super::*;

    #[test]
    fn rng_draws() {
        let mut l = Layer::new(3, Kind::AddIsland);
        l.set_local_world_seed(999);
        l.set_chunk_seed(3, -2);
        println!(
            "RUST {} {} {} {}",
            l.next_int(1),
            l.next_int(2),
            l.next_int(2),
            l.next_int(3)
        );
    }

    #[test]
    fn add_island_cell() {
        let vals = vec![
            0, 24, 24, 24, 4, 24, 24, 0, 0, 24, 24, 24, 0, 0, 0, 24, 24, 24, 24, 29, 0, 24, 0, 0,
            29, 29, 29, 29, 0, 0, 0, 29, 29, 29, 29, 0, 0, 0, 29, 29, 29, 29, 24, 0, 0, 0, 29, 29,
            29,
        ];
        let mut l = Layer::new(3, Kind::AddIsland).parent(fixed(vals));
        l.set_local_world_seed(999);
        let out = l.area(-1, -2, 5, 5);
        println!(
            "ADDISLAND {}",
            out.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    #[test]
    fn mini_zoom_island() {
        let mut l = Layer::new(1003, Kind::NormalZoom).parent(Layer::new(1, Kind::Island));
        l.set_local_world_seed(999);
        let out = l.area(15, -17, 6, 6);
        println!(
            "{}",
            out.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(out.len(), 36);
    }
}
