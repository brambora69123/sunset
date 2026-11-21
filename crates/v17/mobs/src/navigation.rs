//! `PathNavigation`, from `entity/ai/pathing/PathNavigation.java`.

use crate::math;
use crate::mob::{Mob, MobShape};
use crate::path::Path;
use crate::pathfinder::PathFinder;
use crate::world::MobWorld;

/// `PathNavigation`: owns the current path, steers the mob along it each tick.
pub struct PathNavigation {
    path: Option<Path>,
    speed: f64,
    avoid_sunlight: bool,
    ticks: i32,
    last_stuck_check_time: i32,
    last_stuck_check_pos: (f64, f64, f64),
    can_walk_through_doors: bool,
    can_open_doors: bool,
    can_float: bool,
    can_swim: bool,
    follow_range: f64,
}

impl PathNavigation {
    pub fn new(follow_range: f64) -> Self {
        PathNavigation {
            path: None,
            speed: 1.0,
            avoid_sunlight: false,
            ticks: 0,
            last_stuck_check_time: 0,
            last_stuck_check_pos: (0.0, 0.0, 0.0),
            can_walk_through_doors: true,
            can_open_doors: false,
            can_float: false,
            can_swim: false,
            follow_range,
        }
    }

    /// `setCanFloat`.
    pub fn set_can_float(&mut self, can_float: bool) {
        self.can_float = can_float;
    }

    /// `canFloat()`.
    pub fn can_float(&self) -> bool {
        self.can_float
    }

    /// `setCanOpenDoors`.
    pub fn set_can_open_doors(&mut self, can_open_doors: bool) {
        self.can_open_doors = can_open_doors;
    }

    /// `setCanWalkThroughDoors`.
    pub fn set_can_walk_through_doors(&mut self, can_walk_through_doors: bool) {
        self.can_walk_through_doors = can_walk_through_doors;
    }

    /// `setAvoidSunlight`.
    pub fn set_avoid_sunlight(&mut self, avoid_sunlight: bool) {
        self.avoid_sunlight = avoid_sunlight;
    }

    /// `setCanSwim`.
    pub fn set_can_swim(&mut self, can_swim: bool) {
        self.can_swim = can_swim;
    }

    /// `setSpeed`.
    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed;
    }

    /// `setFollowRange` — the fixture seam sets the attribute base.
    pub fn set_follow_range(&mut self, follow_range: f64) {
        self.follow_range = follow_range;
    }

    /// `canUpdatePath()`.
    fn can_update_path(&self, shape: &MobShape) -> bool {
        shape.on_ground
            || self.can_swim && (shape.in_water || shape.in_lava)
            || shape.riding_zombie_on_chicken
    }

    /// `findPath(double x, double y, double z)`.
    pub fn find_path(
        &self,
        world: &dyn MobWorld,
        shape: &MobShape,
        x: f64,
        y: f64,
        z: f64,
    ) -> Option<Path> {
        if !self.can_update_path(shape) {
            return None;
        }
        let mut finder = PathFinder::new(
            self.can_walk_through_doors,
            self.can_open_doors,
            self.can_float,
            self.can_swim,
        );
        finder.find_path_to_block(
            world,
            shape,
            math::floor(x),
            y as i32,
            math::floor(z),
            self.follow_range as f32,
        )
    }

    /// `moveTo(double x, double y, double z, double speed)`.
    pub fn move_to(
        &mut self,
        world: &dyn MobWorld,
        shape: &MobShape,
        x: f64,
        y: f64,
        z: f64,
        speed: f64,
    ) -> bool {
        let path = self.find_path(world, shape, x, y, z);
        self.move_along(world, shape, path, speed)
    }

    /// `moveTo(Entity entity, double speed)` — an entity target's feet.
    pub fn move_to_entity(
        &mut self,
        world: &dyn MobWorld,
        shape: &MobShape,
        target_x: f64,
        target_y: f64,
        target_z: f64,
        speed: f64,
    ) -> bool {
        if !self.can_update_path(shape) {
            return false;
        }
        let mut finder = PathFinder::new(
            self.can_walk_through_doors,
            self.can_open_doors,
            self.can_float,
            self.can_swim,
        );
        let path = finder.find_path_to_entity(
            world,
            shape,
            target_x,
            target_y,
            target_z,
            self.follow_range as f32,
        );
        match path {
            Some(path) => self.move_along(world, shape, Some(path), speed),
            None => false,
        }
    }

    /// `moveAlong(Path path, double speed)`.
    pub fn move_along(
        &mut self,
        world: &dyn MobWorld,
        shape: &MobShape,
        path: Option<Path>,
        speed: f64,
    ) -> bool {
        let Some(path) = path else {
            self.path = None;
            return false;
        };

        if !path.equals_path(self.path.as_ref()) {
            self.path = Some(path);
        }

        if self.avoid_sunlight {
            self.trim_path(world, shape);
        }

        if self.path.as_ref().unwrap().length() == 0 {
            return false;
        }

        self.speed = speed;
        let (x, y, z) = (
            shape.x,
            surface_y(shape, world, self.can_swim) as f64,
            shape.z,
        );
        self.last_stuck_check_time = self.ticks;
        self.last_stuck_check_pos = (x, y, z);
        true
    }

    /// `getPath()`.
    pub fn get_path(&self) -> Option<&Path> {
        self.path.as_ref()
    }

    /// `trimPath()` — cut the path at the first sunlit node.
    fn trim_path(&mut self, world: &dyn MobWorld, shape: &MobShape) {
        if !world.has_sky_access(
            math::floor(shape.x),
            (shape.min_y() + 0.5) as i32,
            math::floor(shape.z),
        ) && let Some(path) = self.path.as_ref()
        {
            for var1 in 0..path.length() {
                let node = path.node(var1);
                if world.has_sky_access(node.0, node.1, node.2) {
                    self.path.as_mut().unwrap().truncate(var1 as i32 - 1);
                    return;
                }
            }
        }
    }

    /// `tick()` — advance the path and steer.
    pub fn tick(&mut self, mob: &mut Mob, world: &dyn MobWorld) {
        self.ticks += 1;
        let shape = mob.shape();
        if self.is_done() {
            return;
        }

        if self.can_update_path(&shape) {
            self.update_path(&shape, world);
        }

        if self.is_done() {
            return;
        }

        if let Some(path) = self.path.as_ref() {
            let (x, y, z) = path.get_current_pos(shape.width);
            mob.movement_control.update(x, y, z, self.speed);
        }
    }

    /// `updatePath()`.
    fn update_path(&mut self, shape: &MobShape, world: &dyn MobWorld) {
        let pos = (
            shape.x,
            surface_y(shape, world, self.can_swim) as f64,
            shape.z,
        );
        let path = self.path.as_mut().unwrap();
        let mut var2 = path.length();
        let current = path.current_index();
        for var3 in current..path.length() {
            if path.node(var3).1 != pos.1 as i32 {
                var2 = var3;
                break;
            }
        }

        let var8 = shape.width * shape.width;
        for var4 in current..var2 {
            let (px, py, pz) = path.get_pos(shape.width, var4);
            let dx = pos.0 - px;
            let dy = pos.1 - py;
            let dz = pos.2 - pz;
            if dx * dx + dy * dy + dz * dz < var8 as f64 {
                path.set_current_index(var4 + 1);
            }
        }

        let var9 = math::ceil(shape.width as f64);
        let var5 = shape.height as i32 + 1;
        let var6 = var9;
        for var7 in (current..var2).rev() {
            let (px, py, pz) = path.get_pos(shape.width, var7);
            if can_move_directly(world, shape, pos, (px, py, pz), var9, var5, var6) {
                path.set_current_index(var7);
                break;
            }
        }

        if self.ticks - self.last_stuck_check_time > 100 {
            let dx = pos.0 - self.last_stuck_check_pos.0;
            let dy = pos.1 - self.last_stuck_check_pos.1;
            let dz = pos.2 - self.last_stuck_check_pos.2;
            if dx * dx + dy * dy + dz * dz < 2.25 {
                self.stop();
            }

            self.last_stuck_check_time = self.ticks;
            self.last_stuck_check_pos = pos;
        }
    }

    /// `isDone()`.
    pub fn is_done(&self) -> bool {
        match self.path.as_ref() {
            None => true,
            Some(path) => path.is_done(),
        }
    }

    /// `stop()`.
    pub fn stop(&mut self) {
        self.path = None;
    }
}

/// `PathNavigation.getSurfaceY()`.
fn surface_y(shape: &MobShape, world: &dyn MobWorld, can_swim: bool) -> i32 {
    if shape.in_water && can_swim {
        let mut var1 = shape.min_y() as i32;
        let mut var2 = crate::blocks::block_by_id(world.get_block(
            math::floor(shape.x),
            var1,
            math::floor(shape.z),
        ));
        let mut var3 = 0;
        while var2.is_water {
            var1 += 1;
            var2 = crate::blocks::block_by_id(world.get_block(
                math::floor(shape.x),
                var1,
                math::floor(shape.z),
            ));
            var3 += 1;
            if var3 > 16 {
                return shape.min_y() as i32;
            }
        }

        return var1;
    }

    (shape.min_y() + 0.5) as i32
}

/// `PathNavigation.canMoveDirectly` — a straight-line walk check between two points.
#[allow(clippy::too_many_arguments)]
fn can_move_directly(
    world: &dyn MobWorld,
    shape: &MobShape,
    from: (f64, f64, f64),
    to: (f64, f64, f64),
    width: i32,
    height: i32,
    depth: i32,
) -> bool {
    let mut var6 = math::floor(from.0);
    let mut var7 = math::floor(from.2);
    let var8 = to.0 - from.0;
    let var10 = to.2 - from.2;
    let var12 = var8 * var8 + var10 * var10;
    if var12 < 1.0E-8 {
        return false;
    }

    let var14 = 1.0 / var12.sqrt();
    let var8 = var8 * var14;
    let var10 = var10 * var14;
    let mut width = width + 2;
    let mut depth = depth + 2;
    if !can_walk_on(
        world,
        shape,
        var6,
        from.1 as i32,
        var7,
        width,
        height,
        depth,
        from,
        var8,
        var10,
    ) {
        return false;
    }

    width -= 2;
    depth -= 2;
    let var16 = 1.0 / var8.abs();
    let var18 = 1.0 / var10.abs();
    let mut var20 = var6 as f64 * 1.0 - from.0;
    let mut var22 = var7 as f64 * 1.0 - from.2;
    if var8 >= 0.0 {
        var20 += 1.0;
    }

    if var10 >= 0.0 {
        var22 += 1.0;
    }

    var20 /= var8;
    var22 /= var10;
    let var24: i32 = if var8 < 0.0 { -1 } else { 1 };
    let var25: i32 = if var10 < 0.0 { -1 } else { 1 };
    let var26 = math::floor(to.0);
    let var27 = math::floor(to.2);
    let mut var28 = var26 - var6;
    let mut var29 = var27 - var7;

    while var28 * var24 > 0 || var29 * var25 > 0 {
        if var20 < var22 {
            var20 += var16;
            var6 += var24;
            var28 = var26 - var6;
        } else {
            var22 += var18;
            var7 += var25;
            var29 = var27 - var7;
        }
        if !can_walk_on(
            world,
            shape,
            var6,
            from.1 as i32,
            var7,
            width,
            height,
            depth,
            from,
            var8,
            var10,
        ) {
            return false;
        }
    }

    true
}

/// `PathNavigation.canWalkOn`.
#[allow(clippy::too_many_arguments)]
fn can_walk_on(
    world: &dyn MobWorld,
    shape: &MobShape,
    x: i32,
    y: i32,
    z: i32,
    width: i32,
    height: i32,
    depth: i32,
    pos: (f64, f64, f64),
    var8: f64,
    var10: f64,
) -> bool {
    let _ = shape;
    let var12 = x - width / 2;
    let var13 = z - depth / 2;
    if !can_walk_above(
        world, var12, y, var13, width, height, depth, pos, var8, var10,
    ) {
        return false;
    }

    for var14 in var12..var12 + width {
        for var15 in var13..var13 + depth {
            let var16 = var14 as f64 + 0.5 - pos.0;
            let var17 = var15 as f64 + 0.5 - pos.2;
            if !(var16 * var8 + var17 * var10 < 0.0) {
                let material =
                    crate::blocks::block_by_id(world.get_block(var14, y - 1, var15)).get_material();
                if material == crate::blocks::Material::Air {
                    return false;
                }

                if material == crate::blocks::Material::Water && !shape.in_water {
                    return false;
                }

                if material == crate::blocks::Material::Lava {
                    return false;
                }
            }
        }
    }

    true
}

/// `PathNavigation.canWalkAbove`.
#[allow(clippy::too_many_arguments)]
fn can_walk_above(
    world: &dyn MobWorld,
    x: i32,
    y: i32,
    z: i32,
    width: i32,
    height: i32,
    depth: i32,
    pos: (f64, f64, f64),
    var8: f64,
    var10: f64,
) -> bool {
    for var12 in x..x + width {
        for var13 in y..y + height {
            for var14 in z..z + depth {
                let var15 = var12 as f64 + 0.5 - pos.0;
                let var16 = var14 as f64 + 0.5 - pos.2;
                if !(var15 * var8 + var16 * var10 < 0.0)
                    && !crate::blocks::block_by_id(world.get_block(var12, var13, var14))
                        .can_walk_through()
                {
                    return false;
                }
            }
        }
    }

    true
}

impl Default for PathNavigation {
    fn default() -> Self {
        PathNavigation::new(16.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureWorld;
    use crate::mob::MobKind;

    #[test]
    fn move_to_sets_path_and_tick_steers() {
        let mut world = FixtureWorld::new();
        for x in -4..16 {
            for z in -4..16 {
                world.set_block(x, 0, z, 2);
            }
        }
        let mut mob = crate::fixture::mob_at(0.5, 1.0, 0.5);
        mob.kind = MobKind::Zombie;
        mob.on_ground = true;
        let mut nav = PathNavigation::new(16.0);
        let shape = mob.shape();
        assert!(nav.move_to(&world, &shape, 10.0, 1.0, 0.0, 1.0));
        assert!(nav.get_path().is_some());
        // steer east: movement control turns at most 30 degrees per tick
        nav.tick(&mut mob, &world);
        let mut movement_control = std::mem::take(&mut mob.movement_control);
        movement_control.tick(&mut mob);
        mob.movement_control = movement_control;
        assert!((mob.yaw - (-30.0)).abs() < 1e-4);
    }

    #[test]
    fn stop_clears_path() {
        let world = FixtureWorld::new();
        let mob = crate::fixture::mob_at(0.0, 1.0, 0.0);
        let mut nav = PathNavigation::new(16.0);
        let shape = mob.shape();
        let path = Path::new(vec![(0, 0, 0)]);
        assert!(!path.is_done());
        nav.move_along(&world, &shape, Some(path), 1.0);
        assert!(nav.get_path().is_some());
        nav.stop();
        assert!(nav.is_done());
    }

    #[test]
    fn move_along_empty_path_fails() {
        let world = FixtureWorld::new();
        let mob = crate::fixture::mob_at(0.0, 1.0, 0.0);
        let mut nav = PathNavigation::new(16.0);
        let shape = mob.shape();
        assert!(!nav.move_along(&world, &shape, Some(Path::new(Vec::new())), 1.0));
        assert!(!nav.move_along(&world, &shape, None, 1.0));
    }
}
