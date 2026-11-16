//! The combat log parser, on lines shaped like the real Forever logs. The
//! other players and mobs here are made up.

use desktop_lib::parser::{Parser, parse_log};
use serde_json::{Value, json, to_value};

const ME_GUID: &str = "Player-6929-00F21447";
const ME: &str = r#"Player-6929-00F21447,"Anslem-Nightglyph-Normal-US",0x511,0x80000000"#;
const ZOMBIE_GUID: &str = "Creature-0-7052-0-971-1501-00006D7EA9";
const ZOMBIE: &str =
    r#"Creature-0-7052-0-971-1501-00006D7EA9,"Mindless Zombie",0x10a28,0x80000000"#;
const BAT: &str = r#"Creature-0-7052-0-971-1513-0000ED7DB2,"Mangy Duskbat",0xa28,0x80000000"#;
const STRANGER_GUID: &str = "Player-1-0000AAAA";
const STRANGER: &str = r#"Player-1-0000AAAA,"Some-Body-Normal-US",0x518,0x80000000"#;

const ZONE: &str = r#"ZONE_CHANGE,0,"Tirisfal Glades",0"#;
const MAP: &str =
    r#"MAP_CHANGE,1420,"Tirisfal Glades",3837.499756,824.999939,3033.333252,-1485.416626"#;

/// 11/5/2026 23:17:44.947 at UTC-8, when the first fight in these tests starts.
const HIT: i64 = 1_793_949_464_947;

/// A line at `time` on 11/5/2026, in UTC-8 like the real logs.
fn at(time: &str, event: &str) -> String {
    format!("11/5/2026 {time}-8  {event}")
}

/// The 19 advanced params for `guid`, standing at world (x, y) on map 1420.
fn advanced(guid: &str, x: f64, y: f64) -> String {
    format!("{guid},0000000000000000,122,122,24,0,94,0,0,0,0,241,271,0,{x},{y},1420,6.0070,0")
}

/// SPELL_DAMAGE from a Fireball. Its advanced params describe the target.
fn fireball(source: &str, target: &str, target_guid: &str, amount: u64, overkill: i64) -> String {
    let info = advanced(target_guid, 1899.49, 1542.13);
    format!(
        r#"SPELL_DAMAGE,{source},{target},133,"Fireball",0x4,{info},{amount},{amount},{overkill},4,0,0,0,nil,nil,nil,ST"#
    )
}

/// SWING_DAMAGE. Its advanced params describe the attacker.
fn swing(source: &str, source_guid: &str, target: &str, amount: u64) -> String {
    let info = advanced(source_guid, 1834.29, 1372.70);
    format!("SWING_DAMAGE,{source},{target},{info},{amount},{amount},-1,1,0,0,0,nil,nil,nil")
}

/// SWING_DAMAGE_LANDED: the same swing again, described from the target's side.
fn swing_landed(source: &str, target: &str, target_guid: &str, amount: u64) -> String {
    let info = advanced(target_guid, 1831.41, 1370.49);
    format!("SWING_DAMAGE_LANDED,{source},{target},{info},{amount},{amount},-1,1,0,0,0,nil,nil,nil")
}

/// SPELL_HEAL from the player's racial on themselves.
fn heal(amount: u64, overheal: u64, critical: bool) -> String {
    let info = advanced(ME_GUID, 1878.74, 1541.26);
    let critical = if critical { "1" } else { "nil" };
    format!(
        r#"SPELL_HEAL,{ME},{ME},1260198,"Touch of the Grave",0x20,{info},{amount},{amount},{overheal},0,{critical}"#
    )
}

/// SPELL_CAST_SUCCESS by the player, standing at world (x, y).
fn cast(x: f64, y: f64) -> String {
    let info = advanced(ME_GUID, x, y);
    format!(r#"SPELL_CAST_SUCCESS,{ME},{ZOMBIE},133,"Fireball",0x4,{info}"#)
}

fn died(target: &str) -> String {
    format!("UNIT_DIED,0000000000000000,nil,0x80000000,0x80000000,{target},0")
}

fn me() -> Value {
    json!({
        "guid": ME_GUID,
        "name": "Anslem Nightglyph",
        "kind": "player",
        "affiliation": "self",
        "playerName": { "firstName": "Anslem", "surname": "Nightglyph", "mode": "Normal", "region": "US" },
    })
}

fn zombie() -> Value {
    json!({ "guid": ZOMBIE_GUID, "name": "Mindless Zombie", "kind": "npc", "affiliation": "outsider" })
}

/// Parses `lines` in order with a new parser and returns the events as JSON.
fn parse(lines: &[String]) -> Vec<Value> {
    let mut parser = Parser::new();
    let mut events = Vec::new();
    for line in lines {
        parser.parse_line(line, &mut events);
    }
    events
        .iter()
        .map(|event| to_value(event).unwrap())
        .collect()
}

/// Each event's type, or its kind for combat events: "fightStart", "damage" and so on.
fn kinds(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .map(|event| match event["type"].as_str().unwrap() {
            "combat" => event["kind"].as_str().unwrap(),
            other => other,
        })
        .collect()
}

#[test]
fn damage_from_the_player_starts_a_fight() {
    let events = parse(&[
        at("23:17:29.943", ZONE),
        at("23:17:29.944", MAP),
        at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 20, 8)),
    ]);
    assert_eq!(
        events,
        [
            json!({
                "type": "fightStart",
                "ts": HIT,
                "fightId": "f-0001",
                "kind": "trash",
                "name": "Mindless Zombie",
                "zone": "Tirisfal Glades",
                "mapId": 1420,
            }),
            json!({
                "type": "combat",
                "kind": "damage",
                "ts": HIT,
                "fightId": "f-0001",
                "source": me(),
                "target": zombie(),
                "spell": { "id": 133, "name": "Fireball" },
                "amount": 20,
                "overkill": 8,
            }),
        ]
    );
}

#[test]
fn damage_to_the_player_starts_a_fight_too() {
    let events = parse(&[at("23:17:44.947", &swing(ZOMBIE, ZOMBIE_GUID, ME, 2))]);
    assert_eq!(kinds(&events), ["fightStart", "damage"]);
    assert_eq!(events[0]["name"], "Mindless Zombie");
    assert_eq!(events[1]["source"], zombie());
    assert_eq!(events[1]["target"], me());
}

#[test]
fn thorns_counts_as_damage() {
    // The game logs damage shields as DAMAGE_SHIELD, from the shield's holder
    // to whoever struck them. Its advanced params describe the target.
    let info = advanced(ZOMBIE_GUID, 1860.39, 1406.69);
    let thorns = format!(
        r#"DAMAGE_SHIELD,{ME},{ZOMBIE},782,"Thorns",0x8,{info},6,6,-1,8,0,0,0,nil,nil,nil,ST"#
    );
    let events = parse(&[
        at("23:17:44.947", &swing(ZOMBIE, ZOMBIE_GUID, ME, 2)),
        at("23:17:44.947", &thorns),
    ]);
    assert_eq!(kinds(&events), ["fightStart", "damage", "damage"]);
    assert_eq!(events[2]["fightId"], events[0]["fightId"]);
    assert_eq!(events[2]["source"], me());
    assert_eq!(events[2]["target"], zombie());
    assert_eq!(events[2]["spell"], json!({ "id": 782, "name": "Thorns" }));
    assert_eq!(events[2]["amount"], 6);
}

#[test]
fn melee_counts_each_swing_once() {
    // The game writes each swing twice: SWING_DAMAGE, then SWING_DAMAGE_LANDED.
    let events = parse(&[
        at("23:17:44.947", &swing(ME, ME_GUID, ZOMBIE, 8)),
        at("23:17:44.947", &swing_landed(ME, ZOMBIE, ZOMBIE_GUID, 8)),
    ]);
    assert_eq!(kinds(&events), ["fightStart", "damage"]);
    assert_eq!(events[1]["amount"], 8);
    assert_eq!(events[1].get("spell"), None, "melee has no spell");
    assert_eq!(events[1].get("overkill"), None, "-1 means no overkill");
}

#[test]
fn heals_join_the_current_fight_but_never_start_one() {
    let events = parse(&[
        at("23:17:40.000", &heal(6, 6, false)),
        at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 23, -1)),
        at("23:17:45.048", &heal(9, 0, true)),
    ]);
    assert_eq!(kinds(&events), ["heal", "fightStart", "damage", "heal"]);

    assert_eq!(events[0].get("fightId"), None);
    assert_eq!(
        events[0]["spell"],
        json!({ "id": 1260198, "name": "Touch of the Grave" })
    );
    assert_eq!(events[0]["amount"], 6);
    assert_eq!(events[0]["overheal"], 6);
    assert_eq!(events[0].get("critical"), None);

    assert_eq!(events[3]["fightId"], "f-0001");
    assert_eq!(events[3].get("overheal"), None);
    assert_eq!(events[3]["critical"], true);
}

#[test]
fn a_fight_ends_after_six_quiet_seconds() {
    let events = parse(&[
        at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
        // 5 s later: the same fight.
        at("23:17:50.000", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
        // Any line more than 6 s after the last hit ends it. Someone else's
        // kill makes no event of its own.
        at("23:17:56.001", &died(BAT)),
        at("23:17:57.000", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
    ]);
    assert_eq!(
        kinds(&events),
        [
            "fightStart",
            "damage",
            "damage",
            "fightEnd",
            "fightStart",
            "damage"
        ]
    );
    assert_eq!(
        events[3],
        json!({ "type": "fightEnd", "ts": 1_793_949_470_001_i64, "fightId": "f-0001" }),
        "the fight ends just after its last hit"
    );
    assert_eq!(events[4]["fightId"], "f-0002");
}

#[test]
fn the_clock_ends_a_fight_when_the_log_goes_quiet() {
    let mut parser = Parser::new();
    let mut events = Vec::new();
    parser.parse_line(
        &at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
        &mut events,
    );
    events.clear();

    // 6 s, plus 1 s for lines the game is still writing.
    parser.end_idle_fight(HIT + 7_000, &mut events);
    assert!(events.is_empty());
    parser.end_idle_fight(HIT + 7_001, &mut events);
    assert_eq!(
        to_value(&events).unwrap(),
        json!([{ "type": "fightEnd", "ts": HIT + 1, "fightId": "f-0001" }])
    );

    events.clear();
    parser.end_idle_fight(HIT + 60_000, &mut events);
    assert!(events.is_empty(), "a fight only ends once");
}

#[test]
fn deaths_count_for_the_player_and_their_enemies() {
    let events = parse(&[
        // The log has a UNIT_DIED for every mob anyone kills.
        at("23:17:30.000", &died(BAT)),
        at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
        at("23:17:45.000", &died(BAT)),
        at("23:17:47.106", &died(ZOMBIE)),
        at("23:18:30.000", &died(ME)),
    ]);
    assert_eq!(
        kinds(&events),
        ["fightStart", "damage", "death", "fightEnd", "death"]
    );
    assert_eq!(events[2]["fightId"], "f-0001");
    assert_eq!(events[2]["target"], zombie());
    assert_eq!(
        events[3]["ts"], 1_793_949_467_107_i64,
        "the kill counts as the fight's last moment"
    );
    assert_eq!(events[4]["target"], me());
    assert_eq!(events[4].get("fightId"), None);
}

#[test]
fn positions_need_the_map_and_come_at_most_once_a_second() {
    let events = parse(&[
        // No MAP_CHANGE yet, so there's nothing to place the position on.
        at("23:17:44.000", &cast(1878.74, 1541.26)),
        at("23:17:44.100", MAP),
        at("23:17:44.200", &cast(1878.74, 1541.26)),
        at("23:17:44.700", &cast(1878.74, 1541.26)),
        at("23:17:45.200", &cast(1834.29, 1372.70)),
    ]);
    // 33.0, 65.0 in game: Deathknell.
    assert_eq!(
        events,
        [
            json!({
                "type": "position",
                "ts": 1_793_949_464_200_i64,
                "unit": me(),
                "mapId": 1420,
                "x": 0.3302,
                "y": 0.6502,
                "source": "log",
            }),
            json!({
                "type": "position",
                "ts": 1_793_949_465_200_i64,
                "unit": me(),
                "mapId": 1420,
                "x": 0.3675,
                "y": 0.665,
                "source": "log",
            }),
        ]
    );
}

#[test]
fn only_the_players_position_is_kept() {
    let events = parse(&[
        at("23:17:44.100", MAP),
        // The player's swing as the zombie saw it: its params describe the zombie.
        at("23:17:44.200", &swing_landed(ME, ZOMBIE, ZOMBIE_GUID, 8)),
        at(
            "23:17:44.300",
            &format!(
                r#"SPELL_CAST_SUCCESS,{STRANGER},{ZOMBIE},133,"Fireball",0x4,{}"#,
                advanced(STRANGER_GUID, 1878.74, 1541.26)
            ),
        ),
    ]);
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn other_players_fights_are_ignored() {
    let events = parse(&[
        at(
            "23:17:44.947",
            &fireball(STRANGER, ZOMBIE, ZOMBIE_GUID, 23, 3),
        ),
        at("23:17:45.000", &swing(ZOMBIE, ZOMBIE_GUID, STRANGER, 5)),
        at("23:17:46.000", &died(ZOMBIE)),
    ]);
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn the_zone_and_map_can_be_read_from_earlier_lines() {
    // The feed starts at the end of a log that's already open, so it reads the
    // zone and map from what came before. That makes no events of its own.
    let mut parser = Parser::new();
    for line in [
        at("23:17:29.943", ZONE),
        at("23:17:29.944", MAP),
        at("23:17:30.000", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
    ] {
        parser.read_zone_and_map(&line);
    }

    let mut events = Vec::new();
    parser.parse_line(&at("23:17:44.200", &cast(1878.74, 1541.26)), &mut events);
    parser.parse_line(
        &at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1)),
        &mut events,
    );
    let events: Vec<Value> = events.iter().map(|e| to_value(e).unwrap()).collect();
    assert_eq!(kinds(&events), ["position", "fightStart", "damage"]);
    assert_eq!(events[1]["zone"], "Tirisfal Glades");
    assert_eq!(events[1]["mapId"], 1420);
    assert_eq!(events[1]["fightId"], "f-0001");
}

#[test]
fn quoted_names_can_hold_commas_and_accents() {
    let guid = "Creature-0-7052-0-971-9999-000000000A";
    let boss = format!(r#"{guid},"Gor, the Ünkind",0x10a48,0x80000000"#);
    let events = parse(&[at("23:17:44.947", &fireball(ME, &boss, guid, 30, -1))]);
    assert_eq!(events[0]["name"], "Gor, the Ünkind");
    assert_eq!(events[1]["target"]["name"], "Gor, the Ünkind");
    assert_eq!(events[1]["amount"], 30);
}

#[test]
fn timestamps_are_utc_ms() {
    for (time, expected) in [
        ("11/5/2026 23:17:44.947-8", HIT),
        // The same moment at UTC+5.5, with one digit of fraction.
        ("11/6/2026 12:47:44.9+5.5", 1_793_949_464_900),
        // Already next year in UTC.
        ("12/31/2026 23:59:59.999-8", 1_798_790_399_999),
        // A leap day.
        ("2/29/2028 12:00:00.000+0", 1_835_438_400_000),
    ] {
        let line = format!("{time}  {}", fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1));
        assert_eq!(parse(&[line])[0]["ts"], expected, "{time}");
    }
}

#[test]
fn lines_it_cant_use_are_skipped() {
    let hit = fireball(ME, ZOMBIE, ZOMBIE_GUID, 10, -1);
    let events = parse(&[
        String::new(),
        "not a log line".into(),
        at(
            "23:17:29.943",
            "COMBAT_LOG_VERSION,22,ADVANCED_LOG_ENABLED,1,BUILD_VERSION,1.60.1,PROJECT_ID,18",
        ),
        at("23:17:44.947", "SPELL_DAMAGE"),
        // Cut off partway, as a half-written line would be.
        at("23:17:44.947", &hit[..hit.len() / 2]),
        format!("someday  {hit}"),
    ]);
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn a_whole_log_ends_its_last_fight() {
    // A replay reads the whole log at once, so no later line or timer ends the last fight.
    let lines = [
        at("23:17:29.943", ZONE),
        at("23:17:29.944", MAP),
        at("23:17:44.947", &fireball(ME, ZOMBIE, ZOMBIE_GUID, 20, 8)),
    ];
    let events: Vec<Value> = parse_log(&lines.join("\n"))
        .iter()
        .map(|event| to_value(event).unwrap())
        .collect();
    assert_eq!(kinds(&events), ["fightStart", "damage", "fightEnd"]);
    assert_eq!(events[2]["ts"], HIT + 1);
    assert!(parse_log("").is_empty());
}
