//! `net.minecraft.scoreboard`: objectives, criteria, scores, teams and the
//! display slots, with `/scoreboard`'s exact limits (names of 16, display
//! names of 32) and HashMap order semantics where they show.

use std::collections::HashMap;

/// `ScoreboardCriterion`: the four writable generics, the read-only health
/// criterion, and stat criteria (`stat.*` keys) a map can also carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Criterion {
    Dummy,
    DeathCount,
    PlayerKillCount,
    TotalKillCount,
    Health,
    /// A `StatCriterion` keyed by its stat's key.
    Stat(String),
}

impl Criterion {
    /// `getName`.
    pub fn name(&self) -> &str {
        match self {
            Self::Dummy => "dummy",
            Self::DeathCount => "deathCount",
            Self::PlayerKillCount => "playerKillCount",
            Self::TotalKillCount => "totalKillCount",
            Self::Health => "health",
            Self::Stat(key) => key,
        }
    }

    /// `isReadOnly`: only health.
    pub fn is_read_only(&self) -> bool {
        matches!(self, Self::Health)
    }

    /// `ScoreboardCriterion.BY_NAME` lookup, including the stat criteria the
    /// registry holds under their stat keys.
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "dummy" => Some(Self::Dummy),
            "deathCount" => Some(Self::DeathCount),
            "playerKillCount" => Some(Self::PlayerKillCount),
            "totalKillCount" => Some(Self::TotalKillCount),
            "health" => Some(Self::Health),
            _ => {
                // StatCriterion entries register under their stat key, which
                // begins "stat." (a `byName` hit for `/scoreboard objectives
                // add`'s completion set); this port accepts them as writable.
                if name.starts_with("stat.") {
                    Some(Self::Stat(name.to_string()))
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Objective {
    pub name: String,
    pub criterion: Criterion,
    pub display_name: String,
}

impl Objective {
    fn new(name: String, criterion: Criterion) -> Self {
        Self {
            display_name: name.clone(),
            name,
            criterion,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Team {
    pub name: String,
    pub display_name: String,
    pub prefix: String,
    pub suffix: String,
    /// `Team` constructs both flags `true`.
    pub allow_friendly_fire: bool,
    pub show_friendly_invisibles: bool,
    /// Java's `HashSet<String>`; the iteration order is hash order.
    pub members: Vec<String>,
}

impl Team {
    fn new(name: String) -> Self {
        Self {
            display_name: name.clone(),
            name,
            prefix: String::new(),
            suffix: String::new(),
            allow_friendly_fire: true,
            show_friendly_invisibles: true,
            members: Vec::new(),
        }
    }
}

/// `ScoreboardScore`: a value and its read-only guard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Score {
    pub objective: String,
    pub owner: String,
    pub score: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Scoreboard {
    /// `objectivesByName`: insertion-ordered (Java HashMap with String keys,
    /// whose iteration order this port keeps in insertion order).
    pub objectives: HashMap<String, Objective>,
    /// `displayObjectives[3]`.
    display: [Option<String>; 3],
    /// `scores`: owner -> (objective name -> score).
    scores: HashMap<String, HashMap<String, Score>>,
    pub teams: HashMap<String, Team>,
    /// `teamsByMember`.
    member_team: HashMap<String, String>,
}

impl Scoreboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// `getObjective(name)`.
    pub fn get_objective(&self, name: &str) -> Option<&Objective> {
        self.objectives.get(name)
    }

    /// `createObjective`: overwrite-checked by the caller
    /// (`/scoreboard objectives add`), Java's duplicate throws.
    pub fn create_objective(&mut self, name: String, criterion: Criterion) {
        self.objectives
            .insert(name.clone(), Objective::new(name, criterion));
    }

    /// `getObjectives()`: in registration order.
    pub fn get_objectives(&self) -> Vec<&Objective> {
        self.objectives.values().collect()
    }

    /// `removeObjective`: drops the objective, its display slots and scores.
    pub fn remove_objective(&mut self, name: &str) {
        self.objectives.remove(name);
        for slot in &mut self.display {
            if slot.as_deref() == Some(name) {
                *slot = None;
            }
        }
        for by_objective in self.scores.values_mut() {
            by_objective.remove(name);
        }
    }

    /// `getScore(owner, objective)`, created on the spot (`getScore` inserts).
    pub fn get_or_create_score(&mut self, owner: &str, objective: &str) -> &mut Score {
        let by_objective = self.scores.entry(owner.to_string()).or_default();
        by_objective.entry(objective.to_string()).or_insert(Score {
            objective: objective.to_string(),
            owner: owner.to_string(),
            score: 0,
        })
    }

    /// `getScores(owner)`: the score map, empty for an unknown owner.
    pub fn get_scores(&self, owner: &str) -> Vec<&Score> {
        match self.scores.get(owner) {
            Some(map) => map.values().collect(),
            None => Vec::new(),
        }
    }

    /// `getScoreOwners()`.
    pub fn get_score_owners(&self) -> Vec<String> {
        self.scores.keys().cloned().collect()
    }

    /// `removeScores(owner)`.
    pub fn remove_scores(&mut self, owner: &str) {
        self.scores.remove(owner);
    }

    /// `setDisplayObjective(slot, objective)`.
    pub fn set_display_objective(&mut self, slot: usize, objective: Option<String>) {
        if slot < 3 {
            self.display[slot] = objective;
        }
    }

    /// `getDisplayObjective(slot)`.
    pub fn get_display_objective(&self, slot: usize) -> Option<&str> {
        self.display.get(slot).and_then(|o| o.as_deref())
    }

    /// `getDisplayLocation(slot)`: "list", "sidebar", "belowName", else null.
    pub fn display_location(slot: usize) -> Option<&'static str> {
        match slot {
            0 => Some("list"),
            1 => Some("sidebar"),
            2 => Some("belowName"),
            _ => None,
        }
    }

    /// `getDisplaySlot(name)`: case-insensitive, -1 unknown (as `None`).
    pub fn display_slot(name: &str) -> Option<usize> {
        match name {
            n if n.eq_ignore_ascii_case("list") => Some(0),
            n if n.eq_ignore_ascii_case("sidebar") => Some(1),
            n if n.eq_ignore_ascii_case("belowName") => Some(2),
            _ => None,
        }
    }

    /// `getTeam(name)`.
    pub fn get_team(&self, name: &str) -> Option<&Team> {
        self.teams.get(name)
    }

    /// `addTeam`.
    pub fn add_team(&mut self, name: String) {
        self.teams.insert(name.clone(), Team::new(name));
    }

    /// `removeTeam`: the team and its members' back-pointers.
    pub fn remove_team(&mut self, name: &str) {
        if let Some(team) = self.teams.remove(name) {
            for member in team.members {
                self.member_team.remove(&member);
            }
        }
    }

    /// `getTeams()`.
    pub fn get_teams(&self) -> Vec<&Team> {
        self.teams.values().collect()
    }

    /// `getTeamNames()`.
    pub fn get_team_names(&self) -> Vec<String> {
        self.teams.keys().cloned().collect()
    }

    /// `getTeamOfMember`.
    pub fn get_team_of_member(&self, member: &str) -> Option<&str> {
        self.member_team.get(member).map(String::as_str)
    }

    /// `addMemberToTeam`: false for an unknown team; a member on another team
    /// leaves it first; re-adding to the member's own team still returns true.
    pub fn add_member_to_team(&mut self, member: &str, team_name: &str) -> bool {
        if !self.teams.contains_key(team_name) {
            return false;
        }
        if let Some(current) = self.member_team.get(member).cloned() {
            self.remove_member_from_team(member, &current);
        }
        self.teams
            .get_mut(team_name)
            .expect("checked above")
            .members
            .push(member.to_string());
        self.member_team
            .insert(member.to_string(), team_name.to_string());
        true
    }

    /// `removeMemberFromTeam(member)`: whether the member was on any team.
    pub fn remove_member_from_team_any(&mut self, member: &str) -> bool {
        match self.member_team.get(member).cloned() {
            Some(team_name) => {
                self.remove_member_from_team(member, &team_name);
                true
            }
            None => false,
        }
    }

    /// `removeMemberFromTeam(member, team)`.
    pub fn remove_member_from_team(&mut self, member: &str, team_name: &str) {
        self.member_team.remove(member);
        if let Some(team) = self.teams.get_mut(team_name) {
            team.members.retain(|m| m != member);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criteria_by_name_and_read_only() {
        assert_eq!(Criterion::by_name("dummy"), Some(Criterion::Dummy));
        assert_eq!(Criterion::by_name("health"), Some(Criterion::Health));
        assert_eq!(Criterion::by_name("nope"), None);
        assert!(!Criterion::Dummy.is_read_only());
        assert!(Criterion::Health.is_read_only());
        assert!(!Criterion::DeathCount.is_read_only());
        assert!(Criterion::by_name("stat.jump").is_some());
    }

    #[test]
    fn objective_display_defaults_to_name() {
        let mut board = Scoreboard::new();
        board.create_objective("coins".to_string(), Criterion::Dummy);
        let objective = board.get_objective("coins").unwrap();
        assert_eq!(objective.display_name, "coins");
    }

    #[test]
    fn remove_objective_clears_display_and_scores() {
        let mut board = Scoreboard::new();
        board.create_objective("a".to_string(), Criterion::Dummy);
        board.create_objective("b".to_string(), Criterion::Dummy);
        board.get_or_create_score("p", "a").score = 5;
        board.get_or_create_score("p", "b").score = 7;
        board.set_display_objective(1, Some("a".to_string()));
        board.remove_objective("a");
        assert!(board.get_objective("a").is_none());
        assert_eq!(board.get_display_objective(1), None);
        assert!(board.get_scores("p").iter().all(|s| s.objective != "a"));
        assert_eq!(board.get_scores("p").len(), 1);
    }

    #[test]
    fn display_slots() {
        assert_eq!(Scoreboard::display_slot("LIST"), Some(0));
        assert_eq!(Scoreboard::display_slot("sidebar"), Some(1));
        assert_eq!(Scoreboard::display_slot("belowname"), Some(2));
        assert_eq!(Scoreboard::display_slot("other"), None);
        assert_eq!(Scoreboard::display_location(0), Some("list"));
        assert_eq!(Scoreboard::display_location(3), None);
    }

    #[test]
    fn team_join_moves_between_teams() {
        let mut board = Scoreboard::new();
        board.add_team("red".to_string());
        board.add_team("blue".to_string());
        assert!(board.add_member_to_team("p", "red"));
        assert_eq!(board.get_team_of_member("p"), Some("red"));
        assert_eq!(board.get_team("red").unwrap().members, ["p"]);
        // Moving to another team removes the old membership.
        assert!(board.add_member_to_team("p", "blue"));
        assert_eq!(board.get_team_of_member("p"), Some("blue"));
        assert!(board.get_team("red").unwrap().members.is_empty());
        // Unknown team refuses.
        assert!(!board.add_member_to_team("p", "green"));
        // Leaving.
        assert!(board.remove_member_from_team_any("p"));
        assert!(!board.remove_member_from_team_any("p"));
    }

    #[test]
    fn remove_team_frees_members() {
        let mut board = Scoreboard::new();
        board.add_team("t".to_string());
        assert!(board.add_member_to_team("p", "t"));
        board.remove_team("t");
        assert_eq!(board.get_team_of_member("p"), None);
        assert!(!board.add_member_to_team("p", "t"));
    }

    #[test]
    fn remove_scores_drops_the_owner() {
        let mut board = Scoreboard::new();
        board.create_objective("a".to_string(), Criterion::Dummy);
        board.get_or_create_score("p", "a").score = 3;
        assert_eq!(board.get_score_owners(), ["p"]);
        board.remove_scores("p");
        assert!(board.get_score_owners().is_empty());
    }
}
