//! The entity id registry `/summon` creates from, with the mob ids it
//! initializes (`MobEntity.initialize`).

/// `Entities.getKeys()`: the 1.7.10 entity id strings, in registration order.
pub use crate::facts::ENTITY_IDS;

/// `Entities.getKeys().contains(id)`.
pub use crate::facts::id_is_known;

/// The ids that are no `MobEntity` (`instanceof MobEntity`): the items,
/// projectiles, vehicles and blocks' entities. Everything else that
/// `Entities` registers is a mob in this decompile's hierarchy, where
/// `PathFinderMobEntity` (monsters, golems, water mobs, passives),
/// `FlyingMobEntity` and `AmbientMobEntity` all extend `MobEntity`.
const NOT_MOBS: [&str; 26] = [
    "Item",
    "XPOrb",
    "LeashKnot",
    "Painting",
    "Arrow",
    "Snowball",
    "Fireball",
    "SmallFireball",
    "ThrownEnderpearl",
    "EyeOfEnderSignal",
    "ThrownPotion",
    "ThrownExpBottle",
    "ItemFrame",
    "WitherSkull",
    "PrimedTnt",
    "FallingSand",
    "FireworksRocketEntity",
    "Boat",
    "MinecartRideable",
    "MinecartChest",
    "MinecartFurnace",
    "MinecartTNT",
    "MinecartHopper",
    "MinecartSpawner",
    "MinecartCommandBlock",
    "EnderCrystal",
];

/// Whether the entity id is a `MobEntity` (`instanceof MobEntity`): the
/// target of `((MobEntity)entity).initialize(null)` when `/summon` was given
/// no NBT tag.
pub fn is_mob(id: &str) -> bool {
    id_is_known(id) && !NOT_MOBS.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mob_recognition() {
        assert!(is_mob("Zombie"));
        assert!(is_mob("Villager"));
        assert!(is_mob("VillagerGolem"), "golems extend MobEntity here");
        assert!(is_mob("SnowMan"));
        assert!(is_mob("Ghast"), "FlyingMobEntity extends MobEntity");
        assert!(is_mob("EnderDragon"));
        assert!(is_mob("Squid"));
        assert!(!is_mob("Item"));
        assert!(!is_mob("Boat"));
        assert!(!is_mob("MinecartRideable"));
        assert!(!is_mob("EnderCrystal"), "EnderCrystal extends Entity");
        assert!(!is_mob("Arrow"));
        assert!(!is_mob("nope"));
    }
}
