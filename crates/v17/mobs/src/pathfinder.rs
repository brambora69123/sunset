//! `PathFinder`, from `entity/ai/pathing/PathFinder.java`.

use crate::blocks::{Block, Material, block_by_id};
use crate::math;
use crate::mob::MobShape;
use crate::path::{BinaryHeap, NodeStore, Path, PathNode};
use crate::world::MobWorld;

/// `PathFinder`: A* over walkable blocks with vanilla's blocking rules.
pub struct PathFinder {
    heap: BinaryHeap,
    nodes: NodeStore,
    neighbors: [usize; 32],
    can_walk_through_doors: bool,
    can_open_doors: bool,
    can_float: bool,
    can_swim: bool,
}

impl PathFinder {
    pub fn new(
        can_walk_through_doors: bool,
        can_open_doors: bool,
        can_float: bool,
        can_swim: bool,
    ) -> Self {
        PathFinder {
            heap: BinaryHeap::new(),
            nodes: NodeStore::new(),
            neighbors: [usize::MAX; 32],
            can_walk_through_doors,
            can_open_doors,
            can_float,
            can_swim,
        }
    }

    /// `findPath(entity, target, range)` — an entity target's feet position.
    pub fn find_path_to_entity(
        &mut self,
        world: &dyn MobWorld,
        entity: &MobShape,
        target_x: f64,
        target_y: f64,
        target_z: f64,
        range: f32,
    ) -> Option<Path> {
        self.find_path(world, entity, target_x, target_y, target_z, range)
    }

    /// `findPath(entity, x, y, z, range)` — a block target.
    pub fn find_path_to_block(
        &mut self,
        world: &dyn MobWorld,
        entity: &MobShape,
        x: i32,
        y: i32,
        z: i32,
        range: f32,
    ) -> Option<Path> {
        self.find_path(
            world,
            entity,
            x as f64 + 0.5,
            y as f64 + 0.5,
            z as f64 + 0.5,
            range,
        )
    }

    /// `findPath(entity, double x, double y, double z, range)`.
    fn find_path(
        &mut self,
        world: &dyn MobWorld,
        entity: &MobShape,
        x: f64,
        y: f64,
        z: f64,
        range: f32,
    ) -> Option<Path> {
        self.heap.clear();
        self.nodes.clear();
        let saved_can_float = self.can_float;
        let mut top;
        if self.can_swim && entity.in_water {
            top = entity.min_y() as i32;
            let mut block =
                block_by_id(world.get_block(math::floor(entity.x), top, math::floor(entity.z)));
            while block.is_water {
                top += 1;
                block =
                    block_by_id(world.get_block(math::floor(entity.x), top, math::floor(entity.z)));
            }

            self.can_float = false;
        } else {
            top = math::floor(entity.min_y() + 0.5);
        }

        let start = self.nodes.get_or_add(
            math::floor(entity.min_x()),
            top,
            math::floor(entity.min_z()),
        );
        let target = self.nodes.get_or_add(
            math::floor(x - entity.width as f64 / 2.0),
            math::floor(y),
            math::floor(z - entity.width as f64 / 2.0),
        );
        let bounds = PathNode::new(
            math::floor_f32(entity.width + 1.0),
            math::floor_f32(entity.height + 1.0),
            math::floor_f32(entity.width + 1.0),
        );
        let path = self.build_path(world, entity, start, target, &bounds, range);
        self.can_float = saved_can_float;
        path
    }

    /// `buildPath(entity, start, target, bounds, range)`.
    fn build_path(
        &mut self,
        world: &dyn MobWorld,
        entity: &MobShape,
        start: usize,
        target: usize,
        bounds: &PathNode,
        range: f32,
    ) -> Option<Path> {
        self.nodes.nodes[start].distance_from_start = 0.0;
        self.nodes.nodes[start].distance_to_target =
            self.nodes.nodes[start].squared_distance_to(&self.nodes.nodes[target]);
        self.nodes.nodes[start].weight = self.nodes.nodes[start].distance_to_target;
        self.heap.clear();
        self.heap.insert(&mut self.nodes, start);
        let mut best = start;

        while !self.heap.is_empty() {
            let node = self.heap.pop(&mut self.nodes);
            if self.nodes.nodes[node].equals_node(&self.nodes.nodes[target]) {
                return Some(self.make_path(target));
            }

            if self.nodes.nodes[node].squared_distance_to(&self.nodes.nodes[target])
                < self.nodes.nodes[best].squared_distance_to(&self.nodes.nodes[target])
            {
                best = node;
            }

            self.nodes.nodes[node].visited = true;
            let count = self.get_neighbors(world, entity, node, bounds, target, range);
            for i in 0..count {
                let neighbor = self.neighbors[i];
                let dist = self.nodes.nodes[node].distance_from_start
                    + self.nodes.nodes[node].squared_distance_to(&self.nodes.nodes[neighbor]);
                if !self.nodes.nodes[neighbor].is_in_heap()
                    || dist < self.nodes.nodes[neighbor].distance_from_start
                {
                    self.nodes.nodes[neighbor].prev = Some(node);
                    self.nodes.nodes[neighbor].distance_from_start = dist;
                    self.nodes.nodes[neighbor].distance_to_target =
                        self.nodes.nodes[neighbor].squared_distance_to(&self.nodes.nodes[target]);
                    if self.nodes.nodes[neighbor].is_in_heap() {
                        let weight = self.nodes.nodes[neighbor].distance_from_start
                            + self.nodes.nodes[neighbor].distance_to_target;
                        self.heap.set_weight(&mut self.nodes, neighbor, weight);
                    } else {
                        self.nodes.nodes[neighbor].weight = self.nodes.nodes[neighbor]
                            .distance_from_start
                            + self.nodes.nodes[neighbor].distance_to_target;
                        self.heap.insert(&mut self.nodes, neighbor);
                    }
                }
            }
        }

        if best == start {
            None
        } else {
            Some(self.make_path(best))
        }
    }

    /// `getNeighbors(entity, node, bounds, target, range)`.
    fn get_neighbors(
        &mut self,
        world: &dyn MobWorld,
        entity: &MobShape,
        node: usize,
        bounds: &PathNode,
        target: usize,
        range: f32,
    ) -> usize {
        let mut count = 0;
        let mut blocker = 0;
        let (nx, ny, nz) = (
            self.nodes.nodes[node].x,
            self.nodes.nodes[node].y,
            self.nodes.nodes[node].z,
        );
        if self.get_blocking_type(world, entity, nx, ny + 1, nz, bounds) == 1 {
            blocker = 1;
        }

        // vanilla order: z+1, x-1, x+1, z-1
        let north = self.get_valid_node(world, entity, nx, ny, nz + 1, bounds, blocker);
        let west = self.get_valid_node(world, entity, nx - 1, ny, nz, bounds, blocker);
        let east = self.get_valid_node(world, entity, nx + 1, ny, nz, bounds, blocker);
        let south = self.get_valid_node(world, entity, nx, ny, nz - 1, bounds, blocker);
        for candidate in [north, west, east, south].into_iter().flatten() {
            if !self.nodes.nodes[candidate].visited
                && self.nodes.nodes[candidate].distance_to(&self.nodes.nodes[target]) < range
            {
                self.neighbors[count] = candidate;
                count += 1;
            }
        }

        count
    }

    /// `getValidNode(entity, x, y, z, bounds, neighborBlockingType)`.
    fn get_valid_node(
        &mut self,
        world: &dyn MobWorld,
        entity: &MobShape,
        x: i32,
        y: i32,
        z: i32,
        bounds: &PathNode,
        neighbor_blocking_type: i32,
    ) -> Option<usize> {
        let mut result: Option<usize> = None;
        let mut y = y;
        let blocking = self.get_blocking_type(world, entity, x, y, z, bounds);
        if blocking == 2 {
            return Some(self.nodes.get_or_add(x, y, z));
        }

        if blocking == 1 {
            result = Some(self.nodes.get_or_add(x, y, z));
        }

        if result.is_none()
            && neighbor_blocking_type > 0
            && blocking != -3
            && blocking != -4
            && self.get_blocking_type(world, entity, x, y + neighbor_blocking_type, z, bounds) == 1
        {
            result = Some(self.nodes.get_or_add(x, y + neighbor_blocking_type, z));
            y += neighbor_blocking_type;
        }

        if result.is_some() {
            let mut climbs = 0;
            let mut ground = 0;
            while y > 0 {
                ground = self.get_blocking_type(world, entity, x, y - 1, z, bounds);
                if self.can_float && ground == -1 {
                    return None;
                }

                if ground != 1 {
                    break;
                }

                if climbs >= entity.safe_fall_distance {
                    return None;
                }
                climbs += 1;

                y -= 1;
                if y > 0 {
                    result = Some(self.nodes.get_or_add(x, y, z));
                }
            }

            if ground == -2 {
                return None;
            }
        }

        result
    }

    /// `getBlockingType(entity, x, y, z, bounds)`.
    fn get_blocking_type(
        &self,
        world: &dyn MobWorld,
        entity: &MobShape,
        x: i32,
        y: i32,
        z: i32,
        bounds: &PathNode,
    ) -> i32 {
        get_blocking_type(
            world,
            entity,
            x,
            y,
            z,
            bounds,
            self.can_float,
            self.can_open_doors,
            self.can_walk_through_doors,
        )
    }

    /// `buildPath(start, target)` — walks the `prev` chain into a `Path`.
    fn make_path(&self, target: usize) -> Path {
        let mut chain = Vec::new();
        let mut current = Some(target);
        while let Some(index) = current {
            let node = &self.nodes.nodes[index];
            chain.push((node.x, node.y, node.z));
            current = node.prev;
        }
        chain.reverse();
        Path::new(chain)
    }
}

/// `PathFinder.getBlockingType` — the static form, the exact vanilla blocking rules.
#[allow(clippy::too_many_arguments)]
pub fn get_blocking_type(
    world: &dyn MobWorld,
    entity: &MobShape,
    x: i32,
    y: i32,
    z: i32,
    bounds: &PathNode,
    can_float: bool,
    can_open_doors: bool,
    can_walk_through_doors: bool,
) -> i32 {
    let mut walkable_liquid = false;
    for bx in x..x + bounds.x {
        for by in y..y + bounds.y {
            for bz in z..z + bounds.z {
                let block: Block = block_by_id(world.get_block(bx, by, bz));
                if block.get_material() != Material::Air {
                    if block.is_trapdoor {
                        walkable_liquid = true;
                    } else if !block.is_water {
                        if !can_walk_through_doors && block.is_wooden_door {
                            return 0;
                        }
                    } else {
                        if can_float {
                            return -1;
                        }

                        walkable_liquid = true;
                    }

                    let render_type = block.get_render_type();
                    if render_type == 9 {
                        let ex = math::floor(entity.x);
                        let ey = math::floor(entity.y);
                        let ez = math::floor(entity.z);
                        if block_by_id(world.get_block(ex, ey, ez)).get_render_type() != 9
                            && block_by_id(world.get_block(ex, ey - 1, ez)).get_render_type() != 9
                        {
                            return -3;
                        }
                    } else if !block.can_walk_through()
                        && (!can_open_doors || !block.is_wooden_door)
                    {
                        if render_type == 11 || block.is_fence_gate || render_type == 32 {
                            return -3;
                        }

                        if block.is_trapdoor {
                            return -4;
                        }

                        if block.get_material() != Material::Lava {
                            return 0;
                        }

                        if !entity.in_lava {
                            return -2;
                        }
                    }
                }
            }
        }
    }

    if walkable_liquid { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureWorld;
    use crate::mob::MobKind;

    #[test]
    fn straight_path_on_flat_ground() {
        let mut world = FixtureWorld::new();
        // flat grass floor at y=0, air above
        for x in -4..12 {
            for z in -4..12 {
                world.set_block(x, 0, z, 2);
            }
        }
        let mut finder = PathFinder::new(true, false, false, false);
        let entity = crate::fixture::mob_at(0.5, 1.0, 0.5).shape();
        let path = finder.find_path_to_block(&world, &entity, 8, 1, 8, 32.0);
        assert!(path.is_some(), "flat ground path must exist");
        let path = path.unwrap();
        let target = path.target().unwrap();
        assert_eq!(target, (8, 1, 8));
        // every node is walkable: ground below
        for i in 0..path.length() {
            let (x, y, z) = path.node(i);
            assert_eq!(
                world.get_block(x, y - 1, z),
                2,
                "ground below at {x},{y},{z}"
            );
        }
    }

    #[test]
    fn path_goes_around_a_wall() {
        let mut world = FixtureWorld::new();
        for x in -4..20 {
            for z in -4..20 {
                world.set_block(x, 0, z, 2);
            }
        }
        // one-block-high wall along x=6
        for z in -4..20 {
            world.set_block(6, 1, z, 1);
        }
        let mut finder = PathFinder::new(true, false, false, false);
        let entity = crate::fixture::mob_at(2.5, 1.0, 2.5).shape();
        let path = finder.find_path_to_block(&world, &entity, 9, 1, 9, 64.0);
        assert!(path.is_some(), "path around the wall must exist");
        let path = path.unwrap();
        assert_eq!(path.target().unwrap(), (9, 1, 9));
    }

    #[test]
    fn unreachable_target_returns_none() {
        let mut world = FixtureWorld::new();
        // island floor surrounded by a 3-deep lava ocean
        for x in -200..400 {
            for z in -200..400 {
                let island = (0..4).contains(&x) && (0..4).contains(&z);
                world.set_block(x, 0, z, if island { 2 } else { 11 });
                if !island {
                    world.set_block(x, 1, z, 11);
                    world.set_block(x, 2, z, 11);
                }
            }
        }
        let mut finder = PathFinder::new(true, false, false, false);
        let entity = crate::fixture::mob_at(1.5, 1.0, 1.5).shape();
        let path = finder.find_path_to_block(&world, &entity, 6, 1, 6, 64.0);
        // vanilla returns a partial path to the closest reachable node; the island
        // can never reach the target across the lava
        assert!(
            path.is_none_or(|p| p.target() != Some((6, 1, 6))),
            "lava-surrounded island must not reach the target"
        );
    }

    #[test]
    fn water_wading_without_can_float() {
        let mut world = FixtureWorld::new();
        for x in -20..30 {
            for z in -20..30 {
                world.set_block(x, 0, z, 2);
            }
        }
        // a water moat at x=4..5, one block deep
        for z in -4..12 {
            world.set_block(4, 1, z, 9);
            world.set_block(5, 1, z, 9);
            world.set_block(4, 0, z, 9);
            world.set_block(5, 0, z, 9);
        }
        // a land mob without canFloat wades through water: the moat column is
        // blocking type 2 (walkable liquid), so a path exists.
        let mut finder = PathFinder::new(true, false, false, false);
        let entity = crate::fixture::mob_at(1.5, 1.0, 1.5).shape();
        let path = finder.find_path_to_block(&world, &entity, 9, 1, 9, 64.0);
        assert!(
            path.is_some(),
            "water does not block a land mob without canFloat"
        );
    }

    #[test]
    fn water_blocks_with_can_float() {
        let mut world = FixtureWorld::new();
        for x in -200..400 {
            for z in -200..400 {
                world.set_block(x, 0, z, 2);
            }
        }
        for z in -200..400 {
            world.set_block(4, 1, z, 9);
            world.set_block(5, 1, z, 9);
            world.set_block(4, 2, z, 9);
            world.set_block(5, 2, z, 9);
            world.set_block(4, 0, z, 9);
            world.set_block(5, 0, z, 9);
        }
        // a canFloat mob refuses to path through water columns (blocking type -1);
        // vanilla returns the partial path to the closest reachable node, so assert
        // the target is never reached.
        let mut finder = PathFinder::new(true, false, true, false);
        let entity = crate::fixture::mob_at(1.5, 1.0, 1.5).shape();
        let path = finder.find_path_to_block(&world, &entity, 9, 1, 9, 64.0);
        // and every path node stays on the start side of the moat (x < 4),
        // so the target is never reached
        assert!(
            path.as_ref().is_none_or(|p| p.target() != Some((9, 1, 9))),
            "water blocks a canFloat mob from reaching the target"
        );
        if let Some(path) = path {
            for i in 0..path.length() {
                assert!(path.node(i).0 < 4, "node {} crossed the moat", i);
            }
        }
    }

    #[test]
    fn get_blocking_type_values() {
        let mut world = FixtureWorld::new();
        world.set_block(0, 0, 0, 1);
        let entity = crate::fixture::mob_at(0.5, 1.0, 0.5).shape();
        let bounds = PathNode::new(1, 2, 1);
        // solid block: 0
        assert_eq!(
            get_blocking_type(&world, &entity, 0, 0, 0, &bounds, false, false, true),
            0
        );
        // air: 1
        assert_eq!(
            get_blocking_type(&world, &entity, 1, 1, 1, &bounds, false, false, true),
            1
        );
        // water in the box while the entity stands on land: the fluid render branch
        // rejects the node for a land mob: -3
        world.set_block(2, 1, 2, 9);
        assert_eq!(
            get_blocking_type(&world, &entity, 2, 1, 2, &bounds, false, false, true),
            -3
        );
        // the same water column with the entity standing in it: walkable liquid: 2
        let swimmer = crate::fixture::mob_at(2.5, 1.0, 2.5).shape();
        assert_eq!(
            get_blocking_type(&world, &swimmer, 2, 1, 2, &bounds, false, false, true),
            2
        );
        let _ = MobKind::Zombie;
    }
}
