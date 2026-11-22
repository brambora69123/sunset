//! 1.7.10's `Block.afterMinedByPlayer` for an era world: the drop table in
//! `sunrise_world::level::era_drops`, the item entities it spawns and the
//! experience orbs.
use super::*;
use sunrise_core::profile::Profile;
use sunrise_world::level::era_drops::{self, Tool};

impl Game<'_> {
    /// `ServerPlayerGameMode.tryMineBlock`'s `afterMinedByPlayer`, for the
    /// block a survival player has just removed in an era world.
    ///
    /// A 26.3 world rolls its loot table instead ([`Self::block_loot_at`]); the
    /// era export has none, so the era's own table answers. The item entities
    /// are `Block.dropItem`'s: a stack at the block's corner plus 0.15..0.85 on
    /// each axis with `pickUpDelay` 10. The experience is `Block.dropXp`'s: the
    /// total split by `ExperienceOrbEntity.roundSize` at the block's centre.
    pub(in crate::game) fn era_block_destroyed(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        held: Option<&ItemStack>,
    ) {
        let (name, meta) = {
            let behaviours = self.res.behaviours();
            let meta = behaviours.table.int(state, behaviours.props.meta);
            // The era export keeps metadata in 0..=15; a pack's state that
            // carries no `meta` reads as 0, as `Block.getMetadata` would.
            (
                self.res.facts().get(state).name.clone(),
                meta.clamp(0, 15) as u8,
            )
        };
        let tool = Tool {
            shears: held.is_some_and(|stack| &*stack.item == "minecraft:shears"),
            silk_touch: held
                .is_some_and(|stack| stack.enchantment_level("minecraft:silk_touch") > 0),
            fortune: held.map_or(0, |stack| stack.enchantment_level("minecraft:fortune")),
        };
        let do_drops = self
            .ui
            .rules
            .bool_at(sunrise_client::gamerules::vanilla::BLOCK_DROPS);
        let drops = era_drops::after_mined_by_player(
            &name,
            meta,
            tool,
            pos,
            do_drops,
            &mut self.world.random,
        );
        if let Some(kept) = self.break_collect.as_mut() {
            kept.extend(drops.items.into_iter().map(|drop| ItemStack {
                item: Arc::from(drop.item),
                count: drop.count,
                damage: u32::from(drop.damage),
                extra: None,
            }));
        } else {
            for drop in drops.items {
                let stack = ItemStack {
                    item: Arc::from(drop.item),
                    count: drop.count,
                    damage: u32::from(drop.damage),
                    extra: None,
                };
                // `Block.dropItem` sets `pickUpDelay` to 10 after the
                // constructor; `spawn_item_own` draws the item's own random
                // (its motion and yaw), not the level's.
                self.spawn_item_own(stack, drop.position, 10);
            }
        }
        // `Block.dropXp`, at `x + 0.5, y + 0.5, z + 0.5`, each orb of the
        // largest size `roundSize` allows.
        let centre = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.5,
            f64::from(pos[2]) + 0.5,
        ];
        let mut value = drops.experience;
        while value > 0 {
            let orb = ExperienceOrb::split(value);
            value -= orb;
            self.add_orb(centre, orb);
        }
    }

    /// Whether this game rolls 1.7.10's block drops: an era content root (and
    /// therefore the era world opened from it, `Profile::resolve` pairs them).
    pub(in crate::game) fn era_drops(&self) -> bool {
        self.pack.profile == Profile::V1_7_10
    }
}
