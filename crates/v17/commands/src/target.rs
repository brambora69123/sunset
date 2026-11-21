//! `net.minecraft.server.command.TargetSelector`: the `@[parf]` player
//! selectors with their argument map, and the multi-match check the
//! dispatcher uses to fan a command out over players.

use std::collections::HashMap;

/// `TARGET_SELECTOR_PATTERN`: `^@([parf])(?:\[([\w=,!-]*)\])?$` — the whole
/// string, one of `p a r f`, an optional `[...]` of `[A-Za-z0-9_=,!-]`.
/// Returns the selector kind and the bracket body (`None` when absent).
pub fn matches_pattern(text: &str) -> Option<(char, Option<&str>)> {
    let rest = text.strip_prefix('@')?;
    let mut chars = rest.chars();
    let kind = chars.next()?;
    if !matches!(kind, 'p' | 'a' | 'r' | 'f') {
        return None;
    }
    let after_kind = chars.as_str();
    let body = match after_kind.is_empty() {
        true => None,
        false => {
            let body = after_kind.strip_prefix('[')?;
            let body = body.strip_suffix(']')?;
            if body.chars().any(|c| !is_selector_char(c)) {
                return None;
            }
            Some(body)
        }
    };
    Some((kind, body))
}

/// `[\w=,!-]` over ASCII: `[A-Za-z0-9_], =, comma, !, -`.
fn is_selector_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '=' || c == ',' || c == '!' || c == '-'
}

/// `\w` over ASCII: `[A-Za-z0-9_]`.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `[-!]?[\w-]*`: an optional leading `-` or `!`, then word chars and `-`.
fn scan_value(body: &str, start: usize) -> usize {
    let bytes = body.as_bytes();
    let mut i = start;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'!') {
        i += 1;
    }
    while i < bytes.len() && (is_word_char(bytes[i] as char) || bytes[i] == b'-') {
        i += 1;
    }
    i
}

/// One `VALUE_PATTERN` match anchored at `pos`: `([-!]?[\w-]*)(?:$|,)` —
/// the value runs to the next comma (consumed) or the end. Returns
/// `(match start, match end, group 1)`.
fn value_match(body: &str, pos: usize) -> Option<(usize, usize, &str)> {
    let end = scan_value(body, pos);
    let bytes = body.as_bytes();
    if end >= bytes.len() {
        return Some((pos, end, &body[pos..end]));
    }
    if bytes[end] == b',' {
        return Some((pos, end + 1, &body[pos..end]));
    }
    None
}

/// One `ARG_PATTERN` match anchored at `pos`: `(\w+)=([-!]?[\w-]*)(?:$|,)`.
/// Returns `(key, value, match end)`.
fn arg_match(body: &str, pos: usize) -> Option<(&str, &str, usize)> {
    let bytes = body.as_bytes();
    let mut i = pos;
    let key_start = i;
    while i < bytes.len() && is_word_char(bytes[i] as char) {
        i += 1;
    }
    if i == key_start || i >= bytes.len() || bytes[i] != b'=' {
        return None;
    }
    let key = &body[key_start..i];
    i += 1;
    let value_start = i;
    i = scan_value(body, i);
    let value = &body[value_start..i];
    if i >= bytes.len() {
        return Some((key, value, i));
    }
    if bytes[i] == b',' {
        return Some((key, value, i + 1));
    }
    None
}

/// `parseArgs`: Java's regex walk. The bare phase (`VALUE_PATTERN`) fills
/// `x y z r` from the first comma-separated values — each match consumes a
/// positional slot even when the value is empty, and empty values put
/// nothing. The named phase (`ARG_PATTERN`) reads `key=value` pairs over
/// what the bare walk left; later pairs overwrite earlier keys.
pub fn parse_args(body: Option<&str>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(body) = body else {
        return map;
    };
    let names = ["x", "y", "z", "r"];
    let mut slot = 0usize;
    // The last VALUE match's end, -1 when the walk never matched (Java's
    // `var4`, read after the loop).
    let mut last_end: i64 = -1;
    let mut pos = 0usize;
    while let Some((start, end, value)) = value_match(body, pos) {
        if slot < names.len() && !value.is_empty() {
            map.insert(names[slot].to_string(), value.to_string());
        }
        slot += 1;
        last_end = end as i64;
        if end == start {
            break; // a zero-length match at the end; Java skips its repeat
        }
        pos = end;
    }

    let tail = if last_end < 0 {
        body
    } else if (last_end as usize) < body.len() {
        &body[last_end as usize..]
    } else {
        return map;
    };
    let mut p = 0usize;
    while let Some((key, value, end)) = arg_match(tail, p) {
        map.insert(key.to_string(), value.to_string());
        p = end;
    }
    map
}

/// `MathHelper.parseInt(s, defaultValue)`: the parse or the default.
pub fn math_helper_parse_int(text: &str, default: i32) -> i32 {
    crate::args::parse_int(text).unwrap_or(default)
}

/// `isValid`: any `@[parf][...]` shape.
pub fn is_valid(text: &str) -> bool {
    matches_pattern(text).is_some()
}

/// `getTargetCount`: 0 for `a`, 1 otherwise.
fn target_count(kind: char) -> i32 {
    if kind == 'a' { 0 } else { 1 }
}

/// `matchesMultiple`: whether the selector can match more than one player —
/// the dispatcher fans the command out over each match exactly when this
/// holds.
pub fn matches_multiple(text: &str) -> bool {
    let Some((kind, body)) = matches_pattern(text) else {
        return false;
    };
    let args = parse_args(body);
    let count = match args.get("c") {
        Some(c) => math_helper_parse_int(c, target_count(kind)),
        None => target_count(kind),
    };
    count != 1
}

/// `parseScoreboardScores`: the `score_<name>` keys, each parsed with
/// `MathHelper.parseInt(value, 1)`; the `_min` suffix stays in the key (the
/// player match reads it).
pub fn parse_scoreboard_scores(args: &HashMap<String, String>) -> HashMap<String, i32> {
    let mut scores = HashMap::new();
    for (key, value) in args {
        if let Some(name) = key.strip_prefix("score_")
            && !name.is_empty()
        {
            scores.insert(name.to_string(), math_helper_parse_int(value, 1));
        }
    }
    scores
}

/// A parsed selector: the kind, the position, radii, levels, game mode
/// filter, count, name and team filters, and the scoreboard scores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub kind: char,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    /// A position, radius or the `rm` key was given: the world filter applies
    /// (Java's `var15`).
    pub position_given: bool,
    pub min_radius: i32,
    pub max_radius: i32,
    pub min_level: i32,
    pub max_level: i32,
    pub count: i32,
    /// `WorldSettings$GameMode.NOT_SET.getId()` by default.
    pub game_mode: i32,
    pub name: Option<String>,
    pub team: Option<String>,
    pub scores: HashMap<String, i32>,
}

/// `TargetSelector.select`'s parsing half: everything but the player lookup,
/// which needs the server's player list (the caller supplies candidates).
pub fn parse_selection(text: &str) -> Option<Selection> {
    let (kind, body) = matches_pattern(text)?;
    let args = parse_args(body);

    let mut selection = Selection {
        kind,
        x: 0,
        y: 0,
        z: 0,
        position_given: false,
        min_radius: 0,
        max_radius: 0,
        min_level: 0,
        max_level: i32::MAX,
        count: target_count(kind),
        game_mode: -1,
        name: None,
        team: None,
        scores: parse_scoreboard_scores(&args),
    };

    // Java's order: rm, r, lm, l, x, y, z, m, c, team, name. The radius and
    // position keys set the world filter.
    for (key, target) in [
        ("rm", &mut selection.min_radius),
        ("r", &mut selection.max_radius),
        ("lm", &mut selection.min_level),
        ("l", &mut selection.max_level),
        ("m", &mut selection.game_mode),
        ("c", &mut selection.count),
    ] {
        if let Some(value) = args.get(key) {
            *target = math_helper_parse_int(value, *target);
        }
    }
    for (key, target) in [
        ("x", &mut selection.x),
        ("y", &mut selection.y),
        ("z", &mut selection.z),
    ] {
        if let Some(value) = args.get(key) {
            *target = math_helper_parse_int(value, *target);
            selection.position_given = true;
        }
    }
    if args.contains_key("rm") || args.contains_key("r") {
        selection.position_given = true;
    }
    selection.name = args.get("name").cloned();
    selection.team = args.get("team").cloned();
    Some(selection)
}

/// One candidate the caller filtered: its name, world index, block position
/// (the `getCommandSourceBlockPos` numbers), xp level, game mode id, team
/// name and scoreboard values. The distance reads in `f32`, as `Vec3i` does.
pub struct Candidate {
    pub name: String,
    pub world: usize,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub level: i32,
    pub game_mode: i32,
    pub team: Option<String>,
    pub scores: HashMap<String, i32>,
}

impl Candidate {
    /// `Vec3i.squaredDistanceTo`: the differences in `f32`, summed in `f32`.
    fn squared_distance(&self, x: i32, y: i32, z: i32) -> f32 {
        let dx = (self.x - x) as f32;
        let dy = (self.y - y) as f32;
        let dz = (self.z - z) as f32;
        dx * dx + dy * dy + dz * dz
    }
}

/// `PlayerManager.getMatching`: filter, sort by distance (a stable sort keeps
/// the player list's order for ties, as Java's sort does), reverse for a
/// negative count, then truncate to `count` (0 = unlimited).
pub fn select(
    candidates: &[Candidate],
    selection: &Selection,
    source: (i32, i32, i32),
    source_world_matches: impl Fn(&Candidate) -> bool,
) -> Vec<usize> {
    let mut matched: Vec<usize> = Vec::new();
    let negative = selection.count < 0;
    let name_negated = selection
        .name
        .as_deref()
        .is_some_and(|n| n.starts_with('!'));
    let team_negated = selection
        .team
        .as_deref()
        .is_some_and(|t| t.starts_with('!'));
    let player_name = selection
        .name
        .as_deref()
        .map(|n| n.strip_prefix('!').unwrap_or(n).to_string());
    let team_name = selection
        .team
        .as_deref()
        .map(|t| t.strip_prefix('!').unwrap_or(t).to_string());
    let min_r_sq = selection.min_radius as f32 * selection.min_radius as f32;
    let max_r_sq = selection.max_radius as f32 * selection.max_radius as f32;
    let count = selection.count.abs();

    for (i, candidate) in candidates.iter().enumerate() {
        if !source_world_matches(candidate) {
            continue;
        }
        if let Some(name) = &player_name {
            let equal = name.eq_ignore_ascii_case(&candidate.name);
            if name_negated == equal {
                continue;
            }
        }
        if let Some(team) = &team_name {
            let candidate_team = candidate.team.as_deref().unwrap_or("");
            if team_negated == team.eq_ignore_ascii_case(candidate_team) {
                continue;
            }
        }
        if selection.position_given && (selection.min_radius > 0 || selection.max_radius > 0) {
            let distance = candidate.squared_distance(source.0, source.1, source.2);
            if (selection.min_radius > 0 && distance < min_r_sq)
                || (selection.max_radius > 0 && distance > max_r_sq)
            {
                continue;
            }
        }
        if !does_player_match_scores(candidate, &selection.scores) {
            continue;
        }
        if selection.game_mode != -1 && selection.game_mode != candidate.game_mode {
            continue;
        }
        if selection.min_level > 0 && candidate.level < selection.min_level {
            continue;
        }
        if candidate.level > selection.max_level {
            continue;
        }
        matched.push(i);
    }

    // Collections.sort with DistancePlayerSorter, then the negative-count reverse.
    if selection.position_given {
        matched.sort_by(|&a, &b| {
            let da = candidates[a].squared_distance(source.0, source.1, source.2);
            let db = candidates[b].squared_distance(source.0, source.1, source.2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    if negative {
        matched.reverse();
    }
    if count > 0 {
        matched.truncate(count as usize);
    }
    matched
}

/// `doesPlayerMatchScores`: every `score_x` entry (with its `_min` form)
/// must hold; a missing objective refuses the candidate.
fn does_player_match_scores(candidate: &Candidate, wanted: &HashMap<String, i32>) -> bool {
    if wanted.is_empty() {
        return true;
    }
    for (key, value) in wanted {
        let (name, minimum) = match key.strip_suffix("_min") {
            Some(base) if key.len() > 4 => (base, true),
            _ => (key.as_str(), false),
        };
        // The objective must exist on the player's scoreboard: this port
        // models that as the score being present.
        let Some(score) = candidate.scores.get(name) else {
            return false;
        };
        if minimum {
            if *score < *value {
                return false;
            }
        } else if *score > *value {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_shapes() {
        assert!(matches_pattern("@p").is_some());
        assert!(matches_pattern("@a").is_some());
        assert!(matches_pattern("@r[x=1,y=2,z=3,r=10]").is_some());
        assert!(matches_pattern("@p[name=!Bob,team=red]").is_some());
        assert!(matches_pattern("@f").is_some());
        assert!(matches_pattern("@p[]").is_some());
        assert!(matches_pattern("@q").is_none(), "only parf");
        assert!(matches_pattern("@").is_none());
        assert!(matches_pattern("p").is_none());
        assert!(matches_pattern("@p[").is_none());
        assert!(matches_pattern("@p]x[").is_none());
        assert!(
            matches_pattern("@p[a b]").is_none(),
            "space is not in the class"
        );
        assert!(matches_pattern("@px").is_none());
        assert!(matches_pattern("@p[ü]").is_none(), "the class is ASCII");
    }

    #[test]
    fn bare_positional_args() {
        let args = parse_args(Some("1,64,0,32"));
        assert_eq!(args.get("x").map(String::as_str), Some("1"));
        assert_eq!(args.get("y").map(String::as_str), Some("64"));
        assert_eq!(args.get("z").map(String::as_str), Some("0"));
        assert_eq!(args.get("r").map(String::as_str), Some("32"));
        // A fifth bare value is dropped (only x y z r have names).
        let args = parse_args(Some("1,2,3,4,5"));
        assert_eq!(args.len(), 4);
        // Named pairs after the bare values.
        let args = parse_args(Some("1,2,3,4,c=-1,name=Bob"));
        assert_eq!(args.get("c").map(String::as_str), Some("-1"));
        assert_eq!(args.get("name").map(String::as_str), Some("Bob"));
    }

    #[test]
    fn empty_values_consume_positional_slots() {
        // The bare walk assigns x, then the empty value consumes y.
        let args = parse_args(Some(",5"));
        assert_eq!(args.get("x"), None);
        assert_eq!(args.get("y").map(String::as_str), Some("5"));
        // A trailing empty value consumes a slot but puts nothing.
        let args = parse_args(Some("5,"));
        assert_eq!(args.get("x").map(String::as_str), Some("5"));
        assert_eq!(args.get("y"), None);
    }

    #[test]
    fn named_args_only() {
        let args = parse_args(Some("name=!Bob,team=red"));
        assert_eq!(args.get("name").map(String::as_str), Some("!Bob"));
        assert_eq!(args.get("team").map(String::as_str), Some("red"));
        assert_eq!(args.get("x"), None);
        // The bare phase refuses a value that carries '='; the ARG phase
        // reads what the bare walk left.
        let args = parse_args(Some("a=b"));
        assert_eq!(
            args.get("a").map(String::as_str),
            Some("b"),
            "the bare walk stops; ARG reads the pair"
        );
        let args = parse_args(Some("a=b=c"));
        assert!(args.is_empty(), "a second '=' refuses the pair");
    }

    #[test]
    fn later_pairs_overwrite_earlier_keys() {
        // The bare value fills x, then the named pair overwrites it.
        let args = parse_args(Some("5,x=1"));
        assert_eq!(args.get("x").map(String::as_str), Some("1"));
    }

    #[test]
    fn multi_match_rules() {
        assert!(matches_multiple("@a"), "a matches everyone");
        assert!(!matches_multiple("@p"), "p is nearest one");
        assert!(!matches_multiple("@r"), "r is random one");
        assert!(matches_multiple("@p[c=5]"));
        assert!(!matches_multiple("@a[c=1]"));
        assert!(matches_multiple("@p[c=-1]"), "negative counts fan out");
        assert!(!matches_multiple("Bob"));
    }

    #[test]
    fn score_args_parse() {
        let args = parse_args(Some("score_kills=3,score_xp_min=10"));
        let scores = parse_scoreboard_scores(&args);
        assert_eq!(scores.get("kills"), Some(&3));
        assert_eq!(
            scores.get("xp_min"),
            Some(&10),
            "the _min suffix stays in the key"
        );
        // score_ alone is too short to count.
        let args = parse_args(Some("score_=1"));
        assert!(parse_scoreboard_scores(&args).is_empty());
    }

    #[test]
    fn selection_defaults() {
        let selection = parse_selection("@a").unwrap();
        assert_eq!(selection.count, 0);
        assert_eq!(selection.max_level, i32::MAX);
        assert_eq!(selection.game_mode, -1);
        assert!(!selection.position_given);
        let selection = parse_selection("@p[x=10,y=64,z=20,r=5,c=-3]").unwrap();
        assert_eq!(selection.count, -3);
        assert_eq!(selection.max_radius, 5);
        assert!(selection.position_given);
        // rm alone also sets the world filter.
        let selection = parse_selection("@p[rm=2]").unwrap();
        assert!(selection.position_given);
        assert_eq!(selection.min_radius, 2);
        // An invalid selector parses to nothing.
        assert!(parse_selection("Bob").is_none());
    }

    fn candidate(name: &str, x: i32, level: i32) -> Candidate {
        Candidate {
            name: name.to_string(),
            world: 0,
            x,
            y: 64,
            z: 0,
            level,
            game_mode: 0,
            team: None,
            scores: HashMap::new(),
        }
    }

    #[test]
    fn nearest_first_and_count() {
        let candidates = [
            candidate("far", 30, 0),
            candidate("near", 1, 0),
            candidate("mid", 10, 0),
        ];
        let selection = parse_selection("@p").unwrap();
        // Without a position, no sort happens: the list order stands.
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [0], "count 1 truncates to the first, unsorted");
        let selection = parse_selection("@p[x=0,y=64,z=0]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(
            candidates[picked[0]].name, "near",
            "position sorts by distance"
        );
        let selection = parse_selection("@a[x=0,y=64,z=0]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked.len(), 3);
        assert_eq!(candidates[picked[0]].name, "near");
        assert_eq!(candidates[picked[2]].name, "far");
    }

    #[test]
    fn negative_count_reverses_farthest_first() {
        let candidates = [
            candidate("near", 1, 0),
            candidate("mid", 10, 0),
            candidate("far", 30, 0),
        ];
        let selection = parse_selection("@p[x=0,y=64,z=0,c=-1]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(
            candidates[picked[0]].name, "far",
            "negative c takes the far end"
        );
    }

    #[test]
    fn level_game_mode_and_name_filters() {
        let mut pro = candidate("pro", 0, 30);
        pro.game_mode = 1;
        let candidates = [candidate("newbie", 5, 0), pro];
        let selection = parse_selection("@a[l=10]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [0], "l is the maximum; the min defaults to 0");
        let selection = parse_selection("@a[lm=10]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(
            picked,
            [1],
            "lm is the minimum; the max defaults to i32::MAX"
        );
        let selection = parse_selection("@a[m=1]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [1]);
        let selection = parse_selection("@a[name=pro]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [1]);
        let selection = parse_selection("@a[name=!pro]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [0]);
    }

    #[test]
    fn team_filter_with_negation() {
        let mut red = candidate("a", 0, 0);
        red.team = Some("red".to_string());
        let mut blue = candidate("b", 1, 0);
        blue.team = Some("blue".to_string());
        let none = candidate("c", 2, 0);
        let candidates = [red, blue, none];
        let selection = parse_selection("@a[team=red]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [0]);
        let selection = parse_selection("@a[team=!red]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(
            picked,
            [1, 2],
            "a missing team counts as \"\" for the negated check"
        );
    }

    #[test]
    fn score_filters() {
        let mut rich = candidate("rich", 0, 0);
        rich.scores.insert("coins".to_string(), 10);
        let mut poor = candidate("poor", 1, 0);
        poor.scores.insert("coins".to_string(), 2);
        let candidates = [rich, poor];
        let selection = parse_selection("@a[score_coins=5]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [1], "score_x is x <= value");
        let selection = parse_selection("@a[score_coins_min=5]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert_eq!(picked, [0], "score_x_min is x >= value");
        // A missing objective refuses.
        let selection = parse_selection("@a[score_gold=1]").unwrap();
        let picked = select(&candidates, &selection, (0, 64, 0), |_| true);
        assert!(picked.is_empty());
    }
}
