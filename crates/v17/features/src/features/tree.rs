//! Tree features: `TreeFeature` (oak, with jungle vines/cocoa), `BirchTree`,
//! `SpruceTree`, `PineTree`, `SwampTree` and `LargeOakTree`.

use crate::block::Block;
use crate::math;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::RandomSource;

/// `AbstractTreeFeature`: shared tree pre-checks and logs.
pub(crate) struct TreeBase {
    /// `setBlock` flag choice: notify-neighbors trees pass 3.
    pub(crate) notify_neighbors: bool,
}

impl TreeBase {
    pub(crate) fn new(notify_neighbors: bool) -> TreeBase {
        TreeBase { notify_neighbors }
    }

    pub(crate) fn flag(&self) -> u8 {
        if self.notify_neighbors {
            3
        } else {
            FLAG_NO_NEIGHBOR_NOTIFY
        }
    }

    /// The shared clearance loop (`y >= 1 && y + height + 1 <= 256`, ring
    /// radius by layer). Returns false on any unreplaceable block.
    pub(crate) fn can_place(
        &self,
        world: &dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        height: i32,
        radius_at: impl Fn(i32) -> i32,
    ) -> bool {
        if y < 1 || y + height + 1 > 256 {
            return false;
        }
        for layer in y..=(y + 1 + height) {
            let radius = radius_at(layer);
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if !(0..256).contains(&layer) {
                        return false;
                    }
                    if !world.get_block(bx, layer, bz).tree_can_replace() {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// `canReplace`-style check used by spruce/pine/swamp: air or leaves only.
    fn can_place_air_or_leaves(
        &self,
        world: &dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        height: i32,
        radius_at: impl Fn(i32) -> i32,
    ) -> bool {
        if y < 1 || y + height + 1 > 256 {
            return false;
        }
        for layer in y..=(y + 1 + height) {
            let radius = radius_at(layer);
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if !(0..256).contains(&layer) {
                        return false;
                    }
                    let block = world.get_block(bx, layer, bz);
                    if !(block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2))
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn ground_ok(&self, world: &dyn GenWorld, x: i32, y: i32, z: i32, height: i32) -> bool {
        let below = world.get_block(x, y - 1, z);
        matches!(below, Block::Grass | Block::Dirt | Block::Farmland) && y < 256 - height - 1
    }

    fn set_dirt(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        world.set_block_with_metadata(x, y - 1, z, Block::Dirt, 0, self.flag());
    }

    fn log(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32, variant: u8) {
        world.set_block_with_metadata(x, y, z, Block::Log, variant, self.flag());
    }
}

/// `TreeFeature` (WorldGenTrees): the oak. Jungle forms carry vines and cocoa.
pub(crate) struct OakTreeFeature {
    pub base_height: i32,
    pub log_variant: u8,
    pub leaves_variant: u8,
    pub place_vines: bool,
    pub notify_neighbors: bool,
}

impl OakTreeFeature {
    pub fn new(notify_neighbors: bool) -> OakTreeFeature {
        OakTreeFeature {
            base_height: 4,
            log_variant: 0,
            leaves_variant: 0,
            place_vines: false,
            notify_neighbors,
        }
    }

    fn flag(&self) -> u8 {
        if self.notify_neighbors {
            3
        } else {
            FLAG_NO_NEIGHBOR_NOTIFY
        }
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = next_int(random, 3) + self.base_height;
        let base = TreeBase {
            notify_neighbors: self.notify_neighbors,
        };
        if !base.can_place(world, x, y, z, height, |layer| {
            if layer == y {
                0
            } else if layer >= y + 1 + height - 2 {
                2
            } else {
                1
            }
        }) {
            return false;
        }

        if !base.ground_ok(world, x, y, z, height) {
            return false;
        }

        base.set_dirt(world, x, y, z);

        // Leaves: `abs(dx) != radius || abs(dz) != radius || (nextInt(2) != 0
        // && corner != 0)` — the draw happens only on ring corners.
        for layer in (y - 3 + height)..=(y + height) {
            let corner = layer - (y + height);
            let radius = 1 - corner / 2;
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if bx.abs_diff(x) != radius as u32
                        || bz.abs_diff(z) != radius as u32
                        || (next_int(random, 2) != 0 && corner != 0)
                    {
                        let block = world.get_block(bx, layer, bz);
                        if block.is_air_material()
                            || matches!(block, Block::Leaves | Block::Leaves2)
                        {
                            world.set_block_with_metadata(
                                bx,
                                layer,
                                bz,
                                Block::Leaves,
                                self.leaves_variant,
                                self.flag(),
                            );
                        }
                    }
                }
            }
        }

        // Trunk, with vine wraps on jungle forms.
        for step in 0..height {
            let block = world.get_block(x, y + step, z);
            if block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2) {
                base.log(world, x, y + step, z, self.log_variant);
                if self.place_vines && step > 0 {
                    if next_int(random, 3) > 0 && world.is_air(x - 1, y + step, z) {
                        world.set_block_with_metadata(
                            x - 1,
                            y + step,
                            z,
                            Block::Vine,
                            8,
                            self.flag(),
                        );
                    }
                    if next_int(random, 3) > 0 && world.is_air(x + 1, y + step, z) {
                        world.set_block_with_metadata(
                            x + 1,
                            y + step,
                            z,
                            Block::Vine,
                            2,
                            self.flag(),
                        );
                    }
                    if next_int(random, 3) > 0 && world.is_air(x, y + step, z - 1) {
                        world.set_block_with_metadata(
                            x,
                            y + step,
                            z - 1,
                            Block::Vine,
                            1,
                            self.flag(),
                        );
                    }
                    if next_int(random, 3) > 0 && world.is_air(x, y + step, z + 1) {
                        world.set_block_with_metadata(
                            x,
                            y + step,
                            z + 1,
                            Block::Vine,
                            4,
                            self.flag(),
                        );
                    }
                }
            }
        }

        if self.place_vines {
            self.place_hanging_vines(world, random, x, y, z, height);
            self.place_cocoa(world, random, x, y, height);
        }

        true
    }

    /// The vine-draping loop and the cocoa pod loop, in vanilla's order.
    fn place_hanging_vines(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
        height: i32,
    ) {
        for layer in (y - 3 + height)..=(y + height) {
            let corner = layer - (y + height);
            let radius = 2 - corner / 2;
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if world.get_block(bx, layer, bz).is_leaves() {
                        if next_int(random, 4) == 0 && world.is_air(bx - 1, layer, bz) {
                            self.vine_down(world, bx - 1, layer, bz, 8);
                        }
                        if next_int(random, 4) == 0 && world.is_air(bx + 1, layer, bz) {
                            self.vine_down(world, bx + 1, layer, bz, 2);
                        }
                        if next_int(random, 4) == 0 && world.is_air(bx, layer, bz - 1) {
                            self.vine_down(world, bx, layer, bz - 1, 1);
                        }
                        if next_int(random, 4) == 0 && world.is_air(bx, layer, bz + 1) {
                            self.vine_down(world, bx, layer, bz + 1, 4);
                        }
                    }
                }
            }
        }

        if next_int(random, 5) == 0 && height > 5 {
            for step in 0..2 {
                for facing in 0..4 {
                    if next_int(random, 4 - step) == 0 {
                        let age = next_int(random, 3);
                        // `Facings.X_OFFSET[Facings.OPPOSITE[facing]]`: the
                        // pod sits on the side the cocoa faces, so facing 0
                        // is north (-z), 1 east (+x), 2 south (+z), 3 west.
                        let (dx, dz) = match facing {
                            0 => (0, -1),
                            1 => (1, 0),
                            2 => (0, 1),
                            _ => (-1, 0),
                        };
                        world.set_block_with_metadata(
                            x + dx,
                            y + height - 5 + step,
                            z + dz,
                            Block::Cocoa,
                            ((age as u8) << 2) | facing as u8,
                            self.flag(),
                        );
                    }
                }
            }
        }
    }

    fn vine_down(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32, metadata: u8) {
        world.set_block_with_metadata(x, y, z, Block::Vine, metadata, self.flag());
        let mut y = y;
        for _ in (0..4).rev() {
            y -= 1;
            if world.get_block(x, y, z).is_air_material() {
                world.set_block_with_metadata(x, y, z, Block::Vine, metadata, self.flag());
            } else {
                break;
            }
        }
    }

    fn place_cocoa(
        &self,
        _world: &mut dyn GenWorld,
        _random: &mut sunset_1_7_core::LegacyRandom,
        _x: i32,
        _y: i32,
        _height: i32,
    ) {
        // Cocoa pods are placed inside `place_hanging_vines`, matching the
        // single combined loop of `TreeFeature`.
    }
}

/// `BirchTreeFeature`.
pub(crate) struct BirchTreeFeature {
    pub tall: bool,
    pub notify_neighbors: bool,
}

impl BirchTreeFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut height = next_int(random, 3) + 5;
        if self.tall {
            height += next_int(random, 7);
        }

        let base = TreeBase {
            notify_neighbors: self.notify_neighbors,
        };
        if !base.can_place(world, x, y, z, height, |layer| {
            if layer == y {
                0
            } else if layer >= y + 1 + height - 2 {
                2
            } else {
                1
            }
        }) {
            return false;
        }

        if !base.ground_ok(world, x, y, z, height) {
            return false;
        }

        base.set_dirt(world, x, y, z);

        for layer in (y - 3 + height)..=(y + height) {
            let corner = layer - (y + height);
            let radius = 1 - corner / 2;
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if bx.abs_diff(x) != radius as u32
                        || bz.abs_diff(z) != radius as u32
                        || (next_int(random, 2) != 0 && corner != 0)
                    {
                        let block = world.get_block(bx, layer, bz);
                        if block.is_air_material()
                            || matches!(block, Block::Leaves | Block::Leaves2)
                        {
                            world.set_block_with_metadata(
                                bx,
                                layer,
                                bz,
                                Block::Leaves,
                                2,
                                base.flag(),
                            );
                        }
                    }
                }
            }
        }

        for step in 0..height {
            let block = world.get_block(x, y + step, z);
            if block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2) {
                base.log(world, x, y + step, z, 2);
            }
        }

        true
    }
}

/// `SpruceTreeFeature`.
pub(crate) struct SpruceTreeFeature {
    pub notify_neighbors: bool,
}

impl SpruceTreeFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = next_int(random, 4) + 6;
        let start = 1 + next_int(random, 2);
        let max_radius = 2 + next_int(random, 2);

        let base = TreeBase {
            notify_neighbors: self.notify_neighbors,
        };
        if !base.can_place_air_or_leaves(world, x, y, z, height, |layer| {
            if layer - y < start { 0 } else { max_radius }
        }) {
            return false;
        }

        if !base.ground_ok(world, x, y, z, height) {
            return false;
        }

        base.set_dirt(world, x, y, z);

        let mut radius = next_int(random, 2);
        let mut max_radius_run = 1;
        let mut current = 0_u8;

        let crown_bottom = height - start;
        for layer in 0..=crown_bottom {
            let ly = y + height - layer;
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if !((bx.abs_diff(x) == radius as u32
                        && bz.abs_diff(z) == radius as u32
                        && radius > 0)
                        || world.get_block(bx, ly, bz).is_opaque())
                    {
                        world.set_block_with_metadata(bx, ly, bz, Block::Leaves, 1, base.flag());
                    }
                }
            }

            if radius >= max_radius_run {
                radius = current as i32;
                current = 1;
                max_radius_run += 1;
                if max_radius_run > max_radius {
                    max_radius_run = max_radius;
                }
            } else {
                radius += 1;
            }
        }

        // `random.nextInt(3)` of the trunk-top gap.
        let gap = next_int(random, 3);
        for step in 0..(height - gap) {
            let block = world.get_block(x, y + step, z);
            if block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2) {
                base.log(world, x, y + step, z, 1);
            }
        }

        true
    }
}

/// `PineTreeFeature`.
pub(crate) struct PineTreeFeature {
    pub notify_neighbors: bool,
}

impl PineTreeFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = next_int(random, 5) + 7;
        let crown_start = height - next_int(random, 2) - 3;
        let max_radius = 1 + next_int(random, height - crown_start + 1);

        let base = TreeBase {
            notify_neighbors: self.notify_neighbors,
        };
        if !base.can_place(world, x, y, z, height, |layer| {
            if layer - y < crown_start {
                0
            } else {
                max_radius
            }
        }) {
            return false;
        }

        if !base.ground_ok(world, x, y, z, height) {
            return false;
        }

        base.set_dirt(world, x, y, z);

        let mut radius = 0_i32;
        for ly in ((y + crown_start)..=(y + height)).rev() {
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if !((bx.abs_diff(x) == radius as u32
                        && bz.abs_diff(z) == radius as u32
                        && radius > 0)
                        || world.get_block(bx, ly, bz).is_opaque())
                    {
                        world.set_block_with_metadata(bx, ly, bz, Block::Leaves, 1, base.flag());
                    }
                }
            }

            if radius >= 1 && ly == y + crown_start + 1 {
                radius -= 1;
            } else if radius < max_radius {
                radius += 1;
            }
        }

        for step in 0..(height - 1) {
            let block = world.get_block(x, y + step, z);
            if block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2) {
                base.log(world, x, y + step, z, 1);
            }
        }

        true
    }
}

/// `SwampTreeFeature`.
pub(crate) struct SwampTreeFeature;

impl SwampTreeFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = next_int(random, 4) + 5;
        let mut y = y;
        while world.get_block(x, y - 1, z) == Block::Water {
            y -= 1;
        }

        // Clearance: water is allowed at the base layer, nothing else above.
        let mut clear = true;
        if y < 1 || y + height + 1 > 256 {
            return false;
        }
        'outer: for layer in y..=(y + 1 + height) {
            let radius = if layer == y {
                0
            } else if layer >= y + 1 + height - 2 {
                3
            } else {
                1
            };
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if !(0..256).contains(&layer) {
                        clear = false;
                        break 'outer;
                    }
                    let block = world.get_block(bx, layer, bz);
                    if !(block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2))
                    {
                        // Only still water at the base layer is allowed.
                        if (block != Block::Water && block != Block::FlowingWater) || layer > y {
                            clear = false;
                            break 'outer;
                        }
                    }
                }
            }
        }
        if !clear {
            return false;
        }

        let below = world.get_block(x, y - 1, z);
        if !matches!(below, Block::Grass | Block::Dirt) || y >= 256 - height - 1 {
            return false;
        }

        let base = TreeBase {
            notify_neighbors: false,
        };
        world.set_block_with_metadata(x, y - 1, z, Block::Dirt, 0, base.flag());

        // Leaves: the same ring-corner condition as oak, with no corner skip
        // when the ring is opaque-checked after.
        for layer in (y - 3 + height)..=(y + height) {
            let corner = layer - (y + height);
            let radius = 2 - corner / 2;
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if (bx.abs_diff(x) != radius as u32
                        || bz.abs_diff(z) != radius as u32
                        || (next_int(random, 2) != 0 && corner != 0))
                        && !world.get_block(bx, layer, bz).is_opaque()
                    {
                        world.set_block_with_metadata(bx, layer, bz, Block::Leaves, 0, base.flag());
                    }
                }
            }
        }

        // Trunk grows through water too.
        for step in 0..height {
            let block = world.get_block(x, y + step, z);
            if block.is_air_material()
                || matches!(
                    block,
                    Block::Leaves | Block::Leaves2 | Block::Water | Block::FlowingWater
                )
            {
                base.log(world, x, y + step, z, 0);
            }
        }

        // Hanging vines.
        for layer in (y - 3 + height)..=(y + height) {
            let corner = layer - (y + height);
            let radius = 2 - corner / 2;
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if world.get_block(bx, layer, bz).is_leaves() {
                        if next_int(random, 4) == 0 && world.is_air(bx - 1, layer, bz) {
                            self.vine_down(world, bx - 1, layer, bz, 8, base.flag());
                        }
                        if next_int(random, 4) == 0 && world.is_air(bx + 1, layer, bz) {
                            self.vine_down(world, bx + 1, layer, bz, 2, base.flag());
                        }
                        if next_int(random, 4) == 0 && world.is_air(bx, layer, bz - 1) {
                            self.vine_down(world, bx, layer, bz - 1, 1, base.flag());
                        }
                        if next_int(random, 4) == 0 && world.is_air(bx, layer, bz + 1) {
                            self.vine_down(world, bx, layer, bz + 1, 4, base.flag());
                        }
                    }
                }
            }
        }

        true
    }

    fn vine_down(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32, metadata: u8, flag: u8) {
        world.set_block_with_metadata(x, y, z, Block::Vine, metadata, flag);
        let mut y = y;
        for _ in (0..4).rev() {
            y -= 1;
            if world.get_block(x, y, z).is_air_material() {
                world.set_block_with_metadata(x, y, z, Block::Vine, metadata, flag);
            } else {
                break;
            }
        }
    }
}

/// `LargeOakTreeFeature` (the big balloon oak), with its internal random.
pub(crate) struct LargeOakTreeFeature {
    random: sunset_1_7_core::LegacyRandom,
    origin: [i32; 3],
    pub height: i32,
    trunk_height: i32,
    trunk_scale: f64,
    branch_slope: f64,
    branch_length_scale: f64,
    foliage_density: f64,
    max_trunk_height: i32,
    foliage_cluster_height: i32,
    branches: Vec<[i32; 4]>,
}

const MINOR_AXES: [usize; 6] = [2, 0, 0, 1, 2, 1];

impl LargeOakTreeFeature {
    pub fn new() -> LargeOakTreeFeature {
        LargeOakTreeFeature {
            random: sunset_1_7_core::LegacyRandom::new(0),
            origin: [0, 0, 0],
            height: 0,
            trunk_height: 0,
            trunk_scale: 0.618,
            branch_slope: 0.381,
            branch_length_scale: 1.0,
            foliage_density: 1.0,
            max_trunk_height: 12,
            foliage_cluster_height: 4,
            branches: Vec::new(),
        }
    }

    fn set_block(
        &self,
        world: &mut dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: u8,
    ) {
        world.set_block_with_metadata(x, y, z, block, metadata, FLAG_NO_NEIGHBOR_NOTIFY);
    }

    /// `prepare(1, 1, 1)`.
    pub fn prepare(&mut self) {
        self.max_trunk_height = (1.0 * 12.0) as i32;
        if 1.0 > 0.5 {
            self.foliage_cluster_height = 5;
        }
        self.branch_length_scale = 1.0;
        self.foliage_density = 1.0;
    }

    pub fn place(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        self.origin = [x, y, z];
        self.branches.clear();
        let seed = random.next_long();
        self.random.set_seed(seed);
        if self.height == 0 {
            self.height = 5 + next_int(&mut self.random, self.max_trunk_height);
        }

        if !self.can_place(world) {
            return false;
        }

        self.make_branches(world);
        self.place_foliage(world);
        self.place_trunk(world);
        self.place_branches(world);
        true
    }

    fn can_place(&mut self, world: &dyn GenWorld) -> bool {
        let below = world.get_block(self.origin[0], self.origin[1] - 1, self.origin[2]);
        if !matches!(below, Block::Dirt | Block::Grass | Block::Farmland) {
            return false;
        }

        let clear = self.try_branch(
            world,
            &[self.origin[0], self.origin[1], self.origin[2]],
            &[
                self.origin[0],
                self.origin[1] + self.height - 1,
                self.origin[2],
            ],
        );
        if clear == -1 {
            return true;
        }
        if clear < 6 {
            return false;
        }
        self.height = clear;
        true
    }

    /// Distance until the ray hits a non-replaceable block, or -1 if clear.
    fn try_branch(&self, world: &dyn GenWorld, from: &[i32; 3], to: &[i32; 3]) -> i32 {
        let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let mut major = 0;
        for axis in 0..3 {
            if delta[axis].abs() > delta[major].abs() {
                major = axis;
            }
        }
        if delta[major] == 0 {
            return -1;
        }
        let minor_a = MINOR_AXES[major];
        let minor_b = MINOR_AXES[major + 3];
        let step = if delta[major] > 0 { 1 } else { -1 };
        let slope_a = delta[minor_a] as f64 / delta[major] as f64;
        let slope_b = delta[minor_b] as f64 / delta[major] as f64;

        let mut pos = [0; 3];
        let target = delta[major] + step;
        let mut walked = 0;
        while walked != target {
            pos[major] = from[major] + walked;
            pos[minor_a] = math::floor(from[minor_a] as f64 + walked as f64 * slope_a);
            pos[minor_b] = math::floor(from[minor_b] as f64 + walked as f64 * slope_b);
            if !world.get_block(pos[0], pos[1], pos[2]).tree_can_replace() {
                break;
            }
            walked += step;
        }
        if walked == target { -1 } else { walked.abs() }
    }

    fn make_branches(&mut self, world: &dyn GenWorld) {
        self.trunk_height = (self.height as f64 * self.trunk_scale) as i32;
        if self.trunk_height >= self.height {
            self.trunk_height = self.height - 1;
        }

        let mut cluster_count =
            (1.382 + (self.foliage_density * self.height as f64 / 13.0).powi(2)) as i32;
        if cluster_count < 1 {
            cluster_count = 1;
        }

        let mut tips: Vec<[i32; 4]> = Vec::new();
        let mut foliage_y = self.origin[1] + self.height - self.foliage_cluster_height;
        let top_y = self.origin[1] + self.trunk_height;
        let mut offset = foliage_y - self.origin[1];
        tips.push([self.origin[0], foliage_y, self.origin[2], top_y]);
        foliage_y -= 1;

        while offset >= 0 {
            let shape = self.tree_shape(offset);
            if shape < 0.0 {
                foliage_y -= 1;
                offset -= 1;
            } else {
                let mut placed = 0;
                while placed < cluster_count {
                    let length = self.branch_length_scale
                        * (shape as f64 * (self.random.next_float() as f64 + 0.328));
                    // Java's literal 3.14159, kept for parity.
                    #[allow(clippy::approx_constant)]
                    let angle = self.random.next_float() as f64 * 2.0 * 3.141_59;
                    let tip_x = math::floor(length * angle.sin() + self.origin[0] as f64 + 0.5);
                    let tip_z = math::floor(length * angle.cos() + self.origin[2] as f64 + 0.5);
                    let tip = [tip_x, foliage_y, tip_z];
                    let tip_top = [tip_x, foliage_y + self.foliage_cluster_height, tip_z];
                    if self.try_branch(world, &tip, &tip_top) == -1 {
                        let mut base = [self.origin[0], self.origin[1], self.origin[2]];
                        let distance = ((self.origin[0] - tip_x).abs().pow(2) as f64
                            + (self.origin[2] - tip_z).abs().pow(2) as f64)
                            .sqrt();
                        let drop = distance * self.branch_slope;
                        if (tip[1] as f64 - drop) > top_y as f64 {
                            base[1] = top_y;
                        } else {
                            base[1] = (tip[1] as f64 - drop) as i32;
                        }

                        if self.try_branch(world, &base, &tip) == -1 {
                            tips.push([tip_x, foliage_y, tip_z, base[1]]);
                        }
                    }
                    placed += 1;
                }
                foliage_y -= 1;
                offset -= 1;
            }
        }

        self.branches = tips;
    }

    fn tree_shape(&self, height: i32) -> f32 {
        if (height as f64) < f64::from(self.height) * 0.3 {
            return -1.618;
        }
        let half = self.height as f32 / 2.0;
        let off = self.height as f32 / 2.0 - height as f32;
        let value = if off == 0.0 {
            half
        } else if off.abs() >= half {
            0.0
        } else {
            ((half.abs().powi(2) - off.abs().powi(2)) as f64).sqrt() as f32
        };
        value * 0.5
    }

    fn cluster_shape(&self, layer: i32) -> f32 {
        if !(0..self.foliage_cluster_height).contains(&layer) {
            -1.0
        } else if layer != 0 && layer != self.foliage_cluster_height - 1 {
            3.0
        } else {
            2.0
        }
    }

    fn place_foliage(&self, world: &mut dyn GenWorld) {
        let branches: Vec<[i32; 4]> = self.branches.clone();
        for branch in &branches {
            let base_y = branch[1];
            for y in base_y..(base_y + self.foliage_cluster_height) {
                let shape = self.cluster_shape(y - base_y);
                self.place_cluster(world, branch[0], y, branch[2], shape, 1, Block::Leaves, 0);
            }
        }
    }

    fn place_cluster(
        &self,
        world: &mut dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        shape: f32,
        major_axis: usize,
        block: Block,
        metadata: u8,
    ) {
        let reach = (shape as f64 + 0.618) as i32;
        let minor_a = MINOR_AXES[major_axis];
        let minor_b = MINOR_AXES[major_axis + 3];
        let mut minor_a_pos = -reach;
        let mut minor_b_pos;
        let pos = [x, y, z];
        let mut cursor = [0; 3];
        // `var11[majorAxis] = var10[majorAxis]`: the trunk axis is fixed.
        cursor[major_axis] = pos[major_axis];
        while minor_a_pos <= reach {
            cursor[minor_a] = pos[minor_a] + minor_a_pos;
            minor_b_pos = -reach;
            while minor_b_pos <= reach {
                let dist = ((minor_a_pos.abs() as f64 + 0.5).powi(2)
                    + (minor_b_pos.abs() as f64 + 0.5).powi(2)) as f32;
                if dist > shape * shape {
                    minor_b_pos += 1;
                } else {
                    cursor[minor_b] = pos[minor_b] + minor_b_pos;
                    let at = world.get_block(cursor[0], cursor[1], cursor[2]);
                    if !(at.is_air_material() || matches!(at, Block::Leaves | Block::Leaves2)) {
                        minor_b_pos += 1;
                    } else {
                        self.set_block(world, cursor[0], cursor[1], cursor[2], block, metadata);
                        minor_b_pos += 1;
                    }
                }
            }
            minor_a_pos += 1;
        }
    }

    fn place_branch(&self, world: &mut dyn GenWorld, from: &[i32; 3], to: &[i32; 3]) {
        let delta = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let mut major = 0;
        for axis in 0..3 {
            if delta[axis].abs() > delta[major].abs() {
                major = axis;
            }
        }
        if delta[major] == 0 {
            return;
        }
        let minor_a = MINOR_AXES[major];
        let minor_b = MINOR_AXES[major + 3];
        let step = if delta[major] > 0 { 1 } else { -1 };
        let slope_a = delta[minor_a] as f64 / delta[major] as f64;
        let slope_b = delta[minor_b] as f64 / delta[major] as f64;

        let mut pos = [0; 3];
        let target = delta[major] + step;
        let mut walked = 0;
        while walked != target {
            pos[major] = math::floor(from[major] as f64 + walked as f64 + 0.5);
            pos[minor_a] = math::floor(from[minor_a] as f64 + walked as f64 * slope_a + 0.5);
            pos[minor_b] = math::floor(from[minor_b] as f64 + walked as f64 * slope_b + 0.5);
            // The log faces by the strongest horizontal axis.
            let mut metadata = 0;
            let dx = (pos[0] - from[0]).abs();
            let dz = (pos[2] - from[2]).abs();
            let max = dx.max(dz);
            if max > 0 {
                if dx == max {
                    metadata = 4;
                } else if dz == max {
                    metadata = 8;
                }
            }
            self.set_block(world, pos[0], pos[1], pos[2], Block::Log, metadata);
            walked += step;
        }
    }

    fn place_trunk(&mut self, world: &mut dyn GenWorld) {
        let from = [self.origin[0], self.origin[1], self.origin[2]];
        let to = [
            self.origin[0],
            self.origin[1] + self.trunk_height,
            self.origin[2],
        ];
        self.place_branch(world, &from, &to);
    }

    fn place_branches(&mut self, world: &mut dyn GenWorld) {
        let branches: Vec<[i32; 4]> = self.branches.clone();
        let mut from = [self.origin[0], self.origin[1], self.origin[2]];
        for branch in &branches {
            from[1] = branch[3];
            let to = [branch[0], branch[1], branch[2]];
            let rise = from[1] - self.origin[1];
            // `shouldPlaceBranch`: !(rise < height * 0.2), f64 comparison.
            if (rise as f64) >= f64::from(self.height) * 0.2 {
                self.place_branch(world, &from, &to);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    fn ground() -> ChunkWorld {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        world
    }

    #[test]
    fn oak_places_log_and_leaves_above_ground() {
        let mut world = ground();
        let mut rng = sunset_1_7_core::LegacyRandom::new(21);
        let oak = OakTreeFeature::new(false);
        assert!(oak.place(&mut world, &mut rng, 8, 41, 8));
        assert!(world.block_count(Block::Log) > 0);
        assert!(world.block_count(Block::Leaves) > 0);
        assert_eq!(world.get_block(8, 40, 8), Block::Dirt);
    }

    #[test]
    fn oak_rejects_solid_clearance() {
        let mut world = ground();
        world.set_block_with_metadata(8, 46, 8, Block::Stone, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        let mut rng = sunset_1_7_core::LegacyRandom::new(21);
        let oak = OakTreeFeature::new(false);
        assert!(!oak.place(&mut world, &mut rng, 8, 41, 8));
    }

    #[test]
    fn birch_uses_birch_variants() {
        let mut world = ground();
        let mut rng = sunset_1_7_core::LegacyRandom::new(33);
        let birch = BirchTreeFeature {
            tall: false,
            notify_neighbors: false,
        };
        assert!(birch.place(&mut world, &mut rng, 8, 41, 8));
        assert!(world.block_count(Block::Log) > 0);
        // All logs carry variant 2 (birch), all leaves variant 2.
        let logs_birch = world
            .rows()
            .filter(|(_, _, _, b, _)| *b == Block::Log)
            .all(|(_, _, _, _, m)| m == 2);
        assert!(logs_birch);
    }

    #[test]
    fn spruce_and_pine_place_spruce_logs() {
        for seed in [4_i64, 9, 77] {
            let mut world = ground();
            let mut rng = sunset_1_7_core::LegacyRandom::new(seed);
            let spruce = SpruceTreeFeature {
                notify_neighbors: false,
            };
            assert!(spruce.place(&mut world, &mut rng, 8, 41, 8));
            assert!(world.block_count(Block::Log) > 0);

            let mut world2 = ground();
            let mut rng2 = sunset_1_7_core::LegacyRandom::new(seed + 1);
            let pine = PineTreeFeature {
                notify_neighbors: false,
            };
            assert!(pine.place(&mut world2, &mut rng2, 8, 41, 8));
            assert!(world2.block_count(Block::Log) > 0);
        }
    }

    #[test]
    fn swamp_tree_grows_through_water() {
        let mut world = ground();
        for x in 4..12 {
            for z in 4..12 {
                world.set_block_with_metadata(x, 41, z, Block::Water, 0, FLAG_NO_NEIGHBOR_NOTIFY);
            }
        }
        let mut rng = sunset_1_7_core::LegacyRandom::new(6);
        let swamp = SwampTreeFeature;
        assert!(swamp.place(&mut world, &mut rng, 8, 42, 8));
        assert!(world.block_count(Block::Log) > 0);
        assert!(world.block_count(Block::Vine) > 0);
    }

    #[test]
    fn large_oak_draws_only_from_the_outer_seed() {
        let mut world = ground();
        let mut rng = sunset_1_7_core::LegacyRandom::new(97);
        let mut oak = LargeOakTreeFeature::new();
        oak.prepare();
        assert!(oak.place(&mut world, &mut rng, 8, 41, 8));
        // One nextLong spent on the internal generator.
        let mut reference = sunset_1_7_core::LegacyRandom::new(97);
        let _ = reference.next_long();
        assert_eq!(rng.state(), reference.state());
        assert!(world.block_count(Block::Log) > 0);
        assert!(world.block_count(Block::Leaves) > 0);
    }
}
