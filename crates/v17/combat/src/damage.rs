//! `net.minecraft.entity.damage.DamageSource`: the flags each damage type
//! carries through the pipeline. `DamageSource` is a mutable prototype in
//! vanilla; here a source is an immutable value and callers build one from the
//! statics in [`sources`] with [`DamageSource::with`], which keeps the shared
//! statics side-effect free.

/// 1.7.10's default exhaustion cost per source: 0.3, cleared by the
/// `setBypassesArmor`/`setUnblockable` setters.
pub const DEFAULT_EXHAUSTION: f32 = 0.3;

/// `net.minecraft.world.World$Settings` difficulty, read by hunger drain,
/// starvation and the `scaledWithDifficulty` damage sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    /// `PlayerEntity.takeDamage`: the sources flagged
    /// `scaledWithDifficulty` are scaled against the world difficulty.
    pub fn scale_damage(self, amount: f32) -> f32 {
        match self {
            Self::Peaceful => 0.0,
            Self::Easy => amount / 2.0 + 1.0,
            Self::Normal => amount,
            Self::Hard => amount * 3.0 / 2.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageSource {
    pub name: &'static str,
    pub bypasses_armor: bool,
    pub out_of_world: bool,
    pub unblockable: bool,
    pub fire: bool,
    pub is_projectile: bool,
    pub scaled_with_difficulty: bool,
    pub magic: bool,
    pub explosive: bool,
    pub exhaustion: f32,
    /// `EntityDamageSource.getAttacker()`: the attacker outside of projectiles.
    pub attacker: Option<u64>,
}

impl DamageSource {
    /// The shared prototype: 0.3 hunger cost, no flags.
    const fn new(name: &'static str) -> Self {
        Self {
            name,
            bypasses_armor: false,
            out_of_world: false,
            unblockable: false,
            fire: false,
            is_projectile: false,
            scaled_with_difficulty: false,
            magic: false,
            explosive: false,
            exhaustion: DEFAULT_EXHAUSTION,
            attacker: None,
        }
    }

    /// Vanilla's `setBypassesArmor`: no armor stage and no hunger cost.
    pub const fn set_bypasses_armor(mut self) -> Self {
        self.bypasses_armor = true;
        self.exhaustion = 0.0;
        self
    }

    /// Vanilla's `setUnblockable`: enchantment protection skips the source.
    pub const fn set_unblockable(mut self) -> Self {
        self.unblockable = true;
        self.exhaustion = 0.0;
        self
    }

    pub const fn set_fire(mut self) -> Self {
        self.fire = true;
        self
    }

    pub const fn set_projectile(mut self) -> Self {
        self.is_projectile = true;
        self
    }

    pub const fn set_out_of_world(mut self) -> Self {
        self.out_of_world = true;
        self
    }

    pub const fn set_magic(mut self) -> Self {
        self.magic = true;
        self
    }

    /// Re-flag a copy of a shared source, the way vanilla mutates its
    /// statics (`DamageSource.STARVE.setUnblockable()`).
    pub fn with(mut self, set: impl FnOnce(&mut Self)) -> Self {
        set(&mut self);
        self
    }

    pub fn set_attacker(mut self, attacker: u64) -> Self {
        self.attacker = Some(attacker);
        self
    }

    pub fn bypasses_armor(&self) -> bool {
        self.bypasses_armor
    }

    pub fn is_out_of_world(&self) -> bool {
        self.out_of_world
    }

    pub fn is_unblockable(&self) -> bool {
        self.unblockable
    }

    pub fn is_fire(&self) -> bool {
        self.fire
    }

    pub fn is_projectile(&self) -> bool {
        self.is_projectile
    }

    pub fn is_explosive(&self) -> bool {
        self.explosive
    }

    pub fn is_scaled_with_difficulty(&self) -> bool {
        self.scaled_with_difficulty
    }

    pub fn get_exhaustion(&self) -> f32 {
        self.exhaustion
    }

    /// `DamageSource.player`/`DamageSource.mob` (`EntityDamageSource`): a
    /// melee hit from an attacker.
    pub fn attacker_source(name: &'static str, attacker: u64) -> Self {
        Self::new(name).set_attacker(attacker)
    }

    /// `DamageSource.arrow`: projectile with a shooter for retaliation.
    pub fn arrow(shooter: u64) -> Self {
        Self::new("arrow").set_projectile().set_attacker(shooter)
    }
}

/// The static prototypes, with the same flags as the 1.7.10 statics.
pub mod sources {
    use super::DamageSource;

    pub const FIRE: DamageSource = DamageSource::new("inFire").set_fire();
    pub const ON_FIRE: DamageSource = DamageSource::new("onFire").set_bypasses_armor().set_fire();
    pub const LAVA: DamageSource = DamageSource::new("lava").set_fire();
    pub const IN_WALL: DamageSource = DamageSource::new("inWall").set_bypasses_armor();
    pub const DROWN: DamageSource = DamageSource::new("drown").set_bypasses_armor();
    pub const STARVE: DamageSource = DamageSource::new("starve")
        .set_bypasses_armor()
        .set_unblockable();
    pub const CACTUS: DamageSource = DamageSource::new("cactus");
    pub const FALL: DamageSource = DamageSource::new("fall").set_bypasses_armor();
    pub const OUT_OF_WORLD: DamageSource = DamageSource::new("outOfWorld")
        .set_bypasses_armor()
        .set_out_of_world();
    pub const GENERIC: DamageSource = DamageSource::new("generic").set_bypasses_armor();
    pub const MAGIC: DamageSource = DamageSource::new("magic").set_bypasses_armor().set_magic();
    pub const WITHER: DamageSource = DamageSource::new("wither").set_bypasses_armor();
    pub const ANVIL: DamageSource = DamageSource::new("anvil");
    pub const FALLING_BLOCK: DamageSource = DamageSource::new("fallingBlock");

    /// `DamageSource.thorns`.
    pub fn thorns(attacker: u64) -> DamageSource {
        DamageSource::new("thorns").with(|s| {
            s.magic = true;
            s.attacker = Some(attacker);
        })
    }

    /// `DamageSource.explosion(null)`.
    pub fn explosion() -> DamageSource {
        DamageSource::new("explosion").with(|s| {
            s.scaled_with_difficulty = true;
            s.explosive = true;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypasses_armor_clears_exhaustion() {
        assert_eq!(sources::DROWN.get_exhaustion(), 0.0);
        assert_eq!(sources::STARVE.get_exhaustion(), 0.0);
        assert!(sources::STARVE.is_unblockable());
        // Melee keeps the default 0.3 hunger cost.
        assert_eq!(sources::CACTUS.get_exhaustion(), super::DEFAULT_EXHAUSTION);
    }

    #[test]
    fn prototypes_are_immutable() {
        let hit = sources::CACTUS.set_attacker(7);
        assert_eq!(hit.attacker, Some(7));
        // The shared static keeps its default flags.
        assert_eq!(sources::CACTUS.attacker, None);
    }
}
