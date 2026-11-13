//! Pins the JSON the Rust events produce to the contract in desktop/src/data/event.ts.

use desktop_lib::events::*;
use serde_json::{json, to_value};

fn me() -> Unit {
    Unit {
        guid: "Player-6929-00F21447".into(),
        name: "Anslem Nightglyph".into(),
        kind: UnitKind::Player,
        affiliation: Affiliation::Me,
        player_name: Some(PlayerName {
            first_name: "Anslem".into(),
            surname: "Nightglyph".into(),
            mode: "Normal".into(),
            region: "US".into(),
        }),
        owner_guid: None,
    }
}

fn zombie() -> Unit {
    Unit {
        guid: "Creature-0-1-2-3-1501-0".into(),
        name: "Mindless Zombie".into(),
        kind: UnitKind::Npc,
        affiliation: Affiliation::Outsider,
        player_name: None,
        owner_guid: None,
    }
}

#[test]
fn damage_matches_the_contract() {
    let event = CorvidEvent::Combat(CombatEvent::Damage(DamageEvent {
        ts: 1_762_413_449_943,
        fight_id: Some("f-0001".into()),
        source: me(),
        target: zombie(),
        spell: Some(Spell {
            id: 133,
            name: "Fireball".into(),
        }),
        amount: 20,
        overkill: Some(8),
        critical: true,
    }));
    assert_eq!(
        to_value(&event).unwrap(),
        json!({
            "type": "combat",
            "kind": "damage",
            "ts": 1_762_413_449_943_i64,
            "fightId": "f-0001",
            "source": {
                "guid": "Player-6929-00F21447",
                "name": "Anslem Nightglyph",
                "kind": "player",
                "affiliation": "self",
                "playerName": { "firstName": "Anslem", "surname": "Nightglyph", "mode": "Normal", "region": "US" }
            },
            "target": {
                "guid": "Creature-0-1-2-3-1501-0",
                "name": "Mindless Zombie",
                "kind": "npc",
                "affiliation": "outsider"
            },
            "spell": { "id": 133, "name": "Fireball" },
            "amount": 20,
            "overkill": 8,
            "critical": true
        })
    );
}

#[test]
fn optional_fields_are_left_out_not_null() {
    let event = CorvidEvent::Combat(CombatEvent::Damage(DamageEvent {
        ts: 1,
        fight_id: None,
        source: zombie(),
        target: me(),
        spell: None,
        amount: 2,
        overkill: None,
        critical: false,
    }));
    let value = to_value(&event).unwrap();
    for key in ["fightId", "spell", "overkill", "critical"] {
        assert!(value.get(key).is_none(), "{key} should be absent");
    }
    assert!(value["source"].get("playerName").is_none());
}

#[test]
fn heal_and_death_match_the_contract() {
    let heal = CorvidEvent::Combat(CombatEvent::Heal(HealEvent {
        ts: 5,
        fight_id: None,
        source: me(),
        target: me(),
        spell: Spell {
            id: 5227,
            name: "Touch of the Grave".into(),
        },
        amount: 6,
        overheal: Some(6),
        critical: false,
    }));
    let value = to_value(&heal).unwrap();
    assert_eq!(value["type"], "combat");
    assert_eq!(value["kind"], "heal");
    assert_eq!(value["overheal"], 6);

    let death = CorvidEvent::Combat(CombatEvent::Death(DeathEvent {
        ts: 6,
        fight_id: Some("f-0001".into()),
        target: zombie(),
    }));
    assert_eq!(
        to_value(&death).unwrap(),
        json!({ "type": "combat", "kind": "death", "ts": 6, "fightId": "f-0001", "target": to_value(zombie()).unwrap() })
    );
}

#[test]
fn position_fights_and_raw_lines_match_the_contract() {
    let position = CorvidEvent::Position(PositionEvent {
        ts: 1,
        unit: me(),
        map_id: 1420,
        x: 0.3301,
        y: 0.65,
        source: PositionSource::Log,
    });
    let value = to_value(&position).unwrap();
    assert_eq!(value["type"], "position");
    assert_eq!(value["mapId"], 1420);
    assert_eq!(value["x"], 0.3301);
    assert_eq!(value["source"], "log");

    let start = CorvidEvent::FightStart(FightStartEvent {
        ts: 2,
        fight_id: "f-0001".into(),
        kind: FightKind::Trash,
        name: "Mindless Zombie".into(),
        zone: Some("Tirisfal Glades".into()),
        map_id: None,
    });
    assert_eq!(
        to_value(&start).unwrap(),
        json!({ "type": "fightStart", "ts": 2, "fightId": "f-0001", "kind": "trash", "name": "Mindless Zombie", "zone": "Tirisfal Glades" })
    );

    let end = CorvidEvent::FightEnd(FightEndEvent {
        ts: 3,
        fight_id: "f-0001".into(),
        success: None,
    });
    assert_eq!(
        to_value(&end).unwrap(),
        json!({ "type": "fightEnd", "ts": 3, "fightId": "f-0001" })
    );

    let raw = CorvidEvent::RawLine(RawLineEvent {
        ts: 4,
        log: LogKind::Combat,
        line: "UNIT_DIED,...".into(),
    });
    assert_eq!(
        to_value(&raw).unwrap(),
        json!({ "type": "rawLine", "ts": 4, "log": "combat", "line": "UNIT_DIED,..." })
    );
}

#[test]
fn a_batch_is_a_json_array() {
    let batch = vec![CorvidEvent::FightEnd(FightEndEvent {
        ts: 3,
        fight_id: "f".into(),
        success: Some(true),
    })];
    assert_eq!(
        serde_json::to_string(&batch).unwrap(),
        r#"[{"type":"fightEnd","ts":3,"fightId":"f","success":true}]"#
    );
}
