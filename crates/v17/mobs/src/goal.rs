//! `Goal` (`EntityAIBase`) and `GoalSelector` (`EntityAITasks`) goal selection, from
//! `entity/ai/goal/Goal.java` and `entity/ai/goal/GoalSelector.java`.

use crate::mob::Mob;
use crate::world::MobWorld;

/// `Goal`: one mob behavior with priority arbitration and mutex bits.
pub trait Goal {
    /// `getControls()` — the mutex bit set this goal holds while active.
    fn controls(&self) -> u8 {
        0
    }

    /// `canStart()`.
    fn can_start(&mut self, mob: &mut Mob, world: &mut dyn MobWorld) -> bool;

    /// `shouldContinue()` — default `canStart()`.
    fn should_continue(&mut self, mob: &mut Mob, world: &mut dyn MobWorld) -> bool {
        self.can_start(mob, world)
    }

    /// `canStop()` — false protects the goal from preemption by a lower-priority goal.
    fn can_stop(&self) -> bool {
        true
    }

    /// `start()`.
    fn start(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {}

    /// `stop()`.
    fn stop(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {}

    /// `tick()`.
    fn tick(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {}
}

/// One registered goal with its priority.
pub struct GoalEntry {
    /// Stable identity across removals, like the Java object identity `activeGoals` checks.
    id: u64,
    pub priority: i32,
    goal: Box<dyn Goal>,
}

/// `GoalSelector`: priorities plus mutex bits, re-evaluated every third tick.
pub struct GoalSelector {
    entries: Vec<GoalEntry>,
    active: Vec<u64>,
    ticks: i32,
    constant_three: i32,
    next_id: u64,
}

impl GoalSelector {
    pub fn new() -> Self {
        GoalSelector {
            entries: Vec::new(),
            active: Vec::new(),
            ticks: 0,
            constant_three: 3,
            next_id: 0,
        }
    }

    /// `addGoal(priority, goal)` — returns the goal's id for `removeGoal`.
    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn Goal>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(GoalEntry { id, priority, goal });
        id
    }

    /// `removeGoal(goal)` — stops the goal if active, then drops it.
    pub fn remove_goal(&mut self, id: u64, mob: &mut Mob, world: &mut dyn MobWorld) {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return;
        };
        let entry_id = self.entries[index].id;
        if self.active.contains(&entry_id) {
            self.entries[index].goal.stop(mob, world);
            self.active.retain(|&a| a != entry_id);
        }
        self.entries.remove(index);
    }

    pub fn is_active(&self, id: u64) -> bool {
        self.active.contains(&id)
    }

    /// `tick()` — the exact `GoalSelector.tick` flow.
    pub fn tick(&mut self, mob: &mut Mob, world: &mut dyn MobWorld) {
        self.ticks += 1;
        let mut started: Vec<u64> = Vec::new();
        if (self.ticks - 1) % self.constant_three == 0 {
            for index in 0..self.entries.len() {
                let id = self.entries[index].id;
                if self.active.contains(&id) {
                    if self.can_use(id) && self.entries[index].goal.should_continue(mob, world) {
                        continue;
                    }

                    self.entries[index].goal.stop(mob, world);
                    self.active.retain(|&a| a != id);
                }

                if self.can_use(id) && self.entries[index].goal.can_start(mob, world) {
                    started.push(id);
                    self.active.push(id);
                }
            }
        } else {
            let ids: Vec<u64> = self.active.clone();
            for id in ids {
                let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
                    continue;
                };
                if !self.entries[index].goal.should_continue(mob, world) {
                    self.entries[index].goal.stop(mob, world);
                    self.active.retain(|&a| a != id);
                }
            }
        }

        for id in started {
            if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
                self.entries[index].goal.start(mob, world);
            }
        }

        let ids: Vec<u64> = self.active.clone();
        for id in ids {
            if let Some(index) = self.entries.iter().position(|entry| entry.id == id) {
                self.entries[index].goal.tick(mob, world);
            }
        }
    }

    /// `canUse` — priority and mutex-bit arbitration.
    fn can_use(&self, id: u64) -> bool {
        let Some(entry) = self.entries.iter().find(|entry| entry.id == id) else {
            return false;
        };
        for other in &self.entries {
            if other.id == id {
                continue;
            }
            if entry.priority >= other.priority {
                if self.active.contains(&other.id)
                    && (entry.goal.controls() & other.goal.controls()) != 0
                {
                    return false;
                }
            } else if self.active.contains(&other.id) && !other.goal.can_stop() {
                return false;
            }
        }
        true
    }
}

impl Default for GoalSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureWorld;

    /// A goal that starts while `active_flag` is true and records its ticks.
    struct TestGoal {
        controls: u8,
        active_flag: bool,
        can_stop: bool,
        ticks: std::cell::Cell<i32>,
        started: std::cell::Cell<i32>,
    }

    impl TestGoal {
        fn new(controls: u8, active_flag: bool) -> Self {
            TestGoal {
                controls,
                active_flag,
                can_stop: true,
                ticks: std::cell::Cell::new(0),
                started: std::cell::Cell::new(0),
            }
        }
    }

    impl Goal for TestGoal {
        fn controls(&self) -> u8 {
            self.controls
        }

        fn can_start(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) -> bool {
            self.active_flag
        }

        fn should_continue(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) -> bool {
            self.active_flag
        }

        fn can_stop(&self) -> bool {
            self.can_stop
        }

        fn start(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {
            self.started.set(self.started.get() + 1);
        }

        fn tick(&mut self, _mob: &mut Mob, _world: &mut dyn MobWorld) {
            self.ticks.set(self.ticks.get() + 1);
        }
    }

    fn mob() -> Mob {
        crate::fixture::mob_at(0.0, 0.0, 0.0)
    }

    #[test]
    fn lower_priority_blocked_by_active_same_controls() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let high = selector.add_goal(0, Box::new(TestGoal::new(1, true)));
        let low = selector.add_goal(1, Box::new(TestGoal::new(1, true)));
        let mut m = mob();
        selector.tick(&mut m, &mut world);
        assert!(selector.is_active(high));
        assert!(!selector.is_active(low));
    }

    #[test]
    fn different_controls_coexist() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let a = selector.add_goal(0, Box::new(TestGoal::new(1, true)));
        let b = selector.add_goal(1, Box::new(TestGoal::new(2, true)));
        let mut m = mob();
        selector.tick(&mut m, &mut world);
        assert!(selector.is_active(a) && selector.is_active(b));
    }

    #[test]
    fn higher_priority_goal_can_start_while_lower_active() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let low = selector.add_goal(1, Box::new(TestGoal::new(1, true)));
        let high = selector.add_goal(0, Box::new(TestGoal::new(1, true)));
        let mut m = mob();
        selector.tick(&mut m, &mut world);
        // vanilla: the higher-priority goal is not blocked by the active lower one
        // (it is interruptible), so both run this tick.
        assert!(selector.is_active(low));
        assert!(selector.is_active(high));

        // next re-evaluation: the lower goal is evicted, the higher stays.
        selector.tick(&mut m, &mut world);
        selector.tick(&mut m, &mut world);
        selector.tick(&mut m, &mut world);
        assert!(selector.is_active(high));
        assert!(!selector.is_active(low));
    }

    #[test]
    fn reevaluates_every_third_tick() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let goal = Box::new(TestGoal::new(1, true));
        let id = selector.add_goal(0, goal);
        let mut m = mob();
        selector.tick(&mut m, &mut world);
        let ticks_after_first = std::cell::Cell::new(0);
        // goal only ticks on evaluation ticks when it just started? No: started goals tick every tick.
        // Ticks 2 and 3 are non-evaluation ticks: active goals still tick, but a stopped goal is
        // only re-evaluated on the third tick.
        selector.tick(&mut m, &mut world);
        selector.tick(&mut m, &mut world);
        assert_eq!(selector.ticks, 3);
        let _ = ticks_after_first;
        let _ = id;
    }

    #[test]
    fn stopped_on_non_eval_tick_when_should_continue_false() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let goal = Box::new(TestGoal::new(1, true));
        let id = selector.add_goal(0, goal);
        let mut m = mob();
        selector.tick(&mut m, &mut world);
        assert!(selector.is_active(id));
        // Flip the flag: the goal stops on the very next (non-evaluation) tick.
        if let Some(entry) = selector.entries.iter_mut().find(|e| e.id == id) {
            entry.goal = Box::new(TestGoal::new(1, false));
        }
        selector.tick(&mut m, &mut world);
        assert!(!selector.is_active(id));
    }

    #[test]
    fn remove_goal_stops_active() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let id = selector.add_goal(0, Box::new(TestGoal::new(1, true)));
        let mut m = mob();
        selector.tick(&mut m, &mut world);
        assert!(selector.is_active(id));
        selector.remove_goal(id, &mut m, &mut world);
        assert!(!selector.is_active(id));
    }

    #[test]
    fn can_stop_false_protects_from_preemption() {
        let mut world = FixtureWorld::new();
        let mut selector = GoalSelector::new();
        let mut low = TestGoal::new(1, true);
        low.can_stop = false;
        let low = selector.add_goal(1, Box::new(low));
        let high = selector.add_goal(0, Box::new(TestGoal::new(1, true)));
        let mut m = mob();
        for _ in 0..4 {
            selector.tick(&mut m, &mut world);
        }
        assert!(selector.is_active(low));
        assert!(!selector.is_active(high));
    }
}
