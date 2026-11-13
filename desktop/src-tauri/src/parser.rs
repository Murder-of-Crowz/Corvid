//! Turns combat log lines into events for the UI. It keeps what involves the
//! player running the app (unit flags 0x511): the damage and healing they do
//! and take, deaths in their fights, and where they are.
//!
//! The log has no fight boundaries. A fight starts with the first damage to or
//! from the player and ends after [`FIGHT_GAP_MS`] without any.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use crate::events::{
    Affiliation, CombatEvent, CorvidEvent, DamageEvent, DeathEvent, FightEndEvent, FightKind,
    FightStartEvent, HealEvent, PlayerName, PositionEvent, PositionSource, Spell, Timestamp, Unit,
    UnitKind,
};

/// A fight ends after this long with no damage to or from the player.
pub const FIGHT_GAP_MS: Timestamp = 6000;
/// Lines reach the file a moment after their timestamp, so the clock only ends
/// a fight once this much longer has passed.
const WRITE_DELAY_MS: Timestamp = 1000;
/// At most one position per unit per interval.
const POSITION_INTERVAL_MS: Timestamp = 1000;

// Unit flags (COMBATLOG_OBJECT_*).
const AFFILIATION_MINE: u32 = 0x1;
const AFFILIATION_PARTY: u32 = 0x2;
const AFFILIATION_RAID: u32 = 0x4;
const TYPE_PLAYER: u32 = 0x400;
const TYPE_PET: u32 = 0x1000;

/// Advanced params are 19 fields: infoGUID, ownerGUID, hp, maxHp, ap, sp, armor, absorb,
/// 2 unknowns, powerType, power, maxPower, cost, posX, posY, uiMapID, facing, level.
const ADVANCED_LEN: usize = 19;

/// Reads the log on eline at a time, remembering the zone, map and fight.
#[derive(Default)]
pub struct Parser {
    zone: Option<String>,
    map_id: Option<u32>,
    map_bounds: HashMap<u32, [f64; 4]>,
    last_position_at: HashMap<String, Timestamp>,
    fight: Option<Fight>,
    fight_count: u32,
}

struct Fight {
    id: String,
    last_ts: Timestamp,
    enemies: HashSet<String>,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the events from one log line to `events`. Lines it has no use for add nothing.
    pub fn parse_line(&mut self, line: &str, events: &mut Vec<CorvidEvent>) {
        let Some((ts, f)) = split_line(line) else {
            return;
        };
        if self.learn_zone_and_map(&f) {
            return;
        }
        if self
            .fight
            .as_ref()
            .is_some_and(|fight| ts - fight.last_ts > FIGHT_GAP_MS)
        {
            self.end_fight(events);
        }
        if f.len() < 9 {
            return;
        }

        let event = f[0];
        let source = unit(f[1], f[2], f[3]);
        let target = unit(f[5], f[6], f[7]);
        let involves_me =
            source.affiliation == Affiliation::Me || target.affiliation == Affiliation::Me;
        let advanced = if event.starts_with("SWING_") { 9 } else { 12 };

        match event {
            "SWING_DAMAGE" | "RANGE_DAMAGE" | "SPELL_DAMAGE" | "SPELL_PERIODIC_DAMAGE" => {
                self.add_position(ts, &f, advanced, &source, &target, events);
                if involves_me {
                    self.add_damage(ts, &f, advanced, source, target, events);
                }
            }
            "SPELL_HEAL" | "SPELL_PERIODIC_HEAL" => {
                self.add_position(ts, &f, advanced, &source, &target, events);
                if involves_me {
                    self.add_heal(ts, &f, source, target, events);
                }
            }
            "SWING_DAMAGE_LANDED" | "SPELL_CAST_SUCCESS" | "SPELL_ENERGIZE" => {
                self.add_position(ts, &f, advanced, &source, &target, events);
            }
            "UNIT_DIED" => self.add_death(ts, target, events),
            _ => {}
        }
    }

    /// Reads only the zone and map from a line, making no events. The feed uses
    /// this on the part of a log written before it started following it.
    pub fn read_zone_and_map(&mut self, line: &str) {
        // A cheap check first: this runs on every line of a log that may be hours long.
        if !line.contains("ZONE_CHANGE,") && !line.contains("MAP_CHANGE,") {
            return;
        }
        if let Some((_, f)) = split_line(line) {
            self.learn_zone_and_map(&f);
        }
    }

    /// Ends the current fight once it has been quiet for a while. The log only
    /// shows that when its next line comes, so the feed calls this on a timer.
    pub fn end_idle_fight(&mut self, now: Timestamp, events: &mut Vec<CorvidEvent>) {
        if self
            .fight
            .as_ref()
            .is_some_and(|fight| now - fight.last_ts > FIGHT_GAP_MS + WRITE_DELAY_MS)
        {
            self.end_fight(events);
        }
    }

    /// Notes ZONE_CHANGE and MAP_CHANGE lines. Returns whether it was one.
    fn learn_zone_and_map(&mut self, f: &[&str]) -> bool {
        match f[0] {
            // ZONE_CHANGE,0,"Tirisfal Glades",0
            "ZONE_CHANGE" => self.zone = f.get(2).map(|name| name.to_string()),
            // MAP_CHANGE,1420,"Tirisfal Glades",x0,x1,y0,y1
            "MAP_CHANGE" => {
                if let (Some(id), Some(bounds)) = (field(f, 1), map_bounds(f)) {
                    self.map_bounds.insert(id, bounds);
                    //Positions say which map the player is on. This is only a first guess.
                    self.map_id.get_or_insert(id);
                }
            }
            _ => return false,
        }
        true
    }

    /// Sends the player's position, at most once a second. The advanced params
    /// describe the unit named by their infoGUID, which may be either side.
    fn add_position(
        &mut self,
        ts: Timestamp,
        f: &[&str],
        advanced: usize,
        source: &Unit,
        target: &Unit,
        events: &mut Vec<CorvidEvent>,
    ) {
        let info = f.get(advanced).copied();
        let unit = if info == Some(source.guid.as_str()) {
            source
        } else if info == Some(target.guid.as_str()) {
            target
        } else {
            return;
        };
        if unit.affiliation != Affiliation::Me || f.len() < advanced + ADVANCED_LEN {
            return;
        }
        let (Some(pos_x), Some(pos_y), Some(map_id)) = (
            field::<f64>(f, advanced + 14),
            field::<f64>(f, advanced + 15),
            field::<u32>(f, advanced + 16),
        ) else {
            return;
        };
        let Some(&[x0, x1, y0, y1]) = self.map_bounds.get(&map_id) else {
            return;
        };
        if self
            .last_position_at
            .get(&unit.guid)
            .is_some_and(|&at| ts - at < POSITION_INTERVAL_MS)
        {
            return;
        }
        self.last_position_at.insert(unit.guid.clone(), ts);
        self.map_id = Some(map_id);
        events.push(CorvidEvent::Position(PositionEvent {
            ts,
            unit: unit.clone(),
            map_id,
            // World X runs north-south and world Y east-west, so they swap.
            x: round4((y0 - pos_y) / (y0 - y1)),
            y: round4((x0 - pos_x) / (x0 - x1)),
            source: PositionSource::Log,
        }));
    }

    fn add_damage(
        &mut self,
        ts: Timestamp,
        f: &[&str],
        advanced: usize,
        source: Unit,
        target: Unit,
        events: &mut Vec<CorvidEvent>,
    ) {
        // The damage fields: amount, base, overkill, school, resisted, blocked,
        // absorbed, critical, glancing, crushing.
        let s = advanced + ADVANCED_LEN;
        let Some(amount) = field(f, s) else {
            return;
        };
        let enemy = if source.affiliation == Affiliation::Me {
            &target
        } else {
            &source
        };
        let fight_id = self.join_fight(ts, enemy, events);
        let spell = if f[0] == "SWING_DAMAGE" {
            None
        } else {
            spell(f)
        };
        events.push(CorvidEvent::Combat(CombatEvent::Damage(DamageEvent {
            ts,
            fight_id: Some(fight_id),
            source,
            target,
            spell,
            amount,
            overkill: positive(f, s + 2),
            critical: f.get(s + 7) == Some(&"1"),
        })));
    }

    fn add_heal(
        &mut self,
        ts: Timestamp,
        f: &[&str],
        source: Unit,
        target: Unit,
        events: &mut Vec<CorvidEvent>,
    ) {
        // The heal fields: amount, base, overheal, absorbed, critical.
        let s = 12 + ADVANCED_LEN;
        let (Some(spell), Some(amount)) = (spell(f), field(f, s)) else {
            return;
        };
        events.push(CorvidEvent::Combat(CombatEvent::Heal(HealEvent {
            ts,
            fight_id: self.fight.as_ref().map(|fight| fight.id.clone()),
            source,
            target,
            spell,
            amount,
            overheal: positive(f, s + 2),
            critical: f.get(s + 4) == Some(&"1"),
        })));
    }

    /// The player's death, or the death of something they're fighting. The log
    /// has a UNIT_DIED for every mob anyone kills, so the rest are skipped.
    fn add_death(&mut self, ts: Timestamp, target: Unit, events: &mut Vec<CorvidEvent>) {
        let mine = target.affiliation == Affiliation::Me;
        let fight_id = match &mut self.fight {
            Some(fight) if mine || fight.enemies.contains(&target.guid) => {
                fight.last_ts = fight.last_ts.max(ts);
                Some(fight.id.clone())
            }
            _ if mine => None,
            _ => return,
        };
        events.push(CorvidEvent::Combat(CombatEvent::Death(DeathEvent {
            ts,
            fight_id,
            target,
        })));
    }

    /// The current fight's ID, after starting one agianst `enemy` if needed.
    fn join_fight(&mut self, ts: Timestamp, enemy: &Unit, events: &mut Vec<CorvidEvent>) -> String {
        let fight = self.fight.get_or_insert_with(|| {
            self.fight_count += 1;
            let id = format!("f-{:04}", self.fight_count);
            events.push(CorvidEvent::FightStart(FightStartEvent {
                ts,
                fight_id: id.clone(),
                kind: FightKind::Trash,
                name: enemy.name.clone(),
                zone: self.zone.clone(),
                map_id: self.map_id,
            }));
            Fight {
                id,
                last_ts: ts,
                enemies: HashSet::new(),
            }
        });
        // max, because lines written late after a logging restart can be older.
        fight.last_ts = fight.last_ts.max(ts);
        fight.enemies.insert(enemy.guid.clone());
        fight.id.clone()
    }

    fn end_fight(&mut self, events: &mut Vec<CorvidEvent>) {
        if let Some(fight) = self.fight.take() {
            events.push(CorvidEvent::FightEnd(FightEndEvent {
                ts: fight.last_ts + 1,
                fight_id: fight.id,
                success: None,
            }));
        }
    }
}

/// Splits a line into its timestamp and fields. Lines look like
/// `11/5/2026 23:17:28.943-8 SWING_DAMAGE,Player...,"Name-...",0x511,...`.
fn split_line(line: &str) -> Option<(Timestamp, Vec<&str>)> {
    let (time, rest) = line.split_once("  ")?;
    Some((parse_timestamp(time)?, split_fields(rest)))
}

// Vanilla timestap to Unix ms. The number at the ned is the UTC offset in hours.
fn parse_timestamp(text: &str) -> Option<Timestamp> {
    let (date, time) = text.split_once(' ')?;
    let mut date = date.split('/').map(|part| part.parse::<i64>().ok());
    let (month, day, year) = (date.next()??, date.next()??, date.next()??);

    let (clock, offset) = time.split_at(time.rfind(['-', '+'])?);
    let offset: f64 = offset.parse().ok()?;
    let (hms, fraction) = clock.split_once('.')?;
    let mut hms = hms.split(':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (hms.next()??, hms.next()??, hms.next()??);
    // Pad or cut the fraction to 3 digits: ".9" is 900 ms.
    let ms: i64 = format!("{fraction:0<3.3}").parse().ok()?;

    let days = days_from_civil(year, month, day);
    let local = (((days * 24 + hour) * 60 + minute) * 60 + second) * 1000 + ms;
    Some(local - (offset * 3_600_000.0).round() as i64)
}

/// Days from 1970-01-01 to a date. Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Splits on commas outside double quotes, and drops the quotes.
fn split_fields(text: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (i, byte) in text.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b',' if !quoted => {
                fields.push(unquote(&text[start..i]));
                start = i + 1;
            }
            _ => {}
        }
    }
    fields.push(unquote(&text[start..]));
    fields
}

fn unquote(field: &str) -> &str {
    field
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(field)
}

fn field<T: FromStr>(f: &[&str], i: usize) -> Option<T> {
    f.get(i)?.parse().ok()
}

/// A count where the log writes -1 or 0 for none, like overkill.
fn positive(f: &[&str], i: usize) -> Option<u64> {
    field(f, i).filter(|&n| n > 0)
}

fn spell(f: &[&str]) -> Option<Spell> {
    Some(Spell {
        id: field(f, 9)?,
        name: f.get(10)?.to_string(),
    })
}

fn map_bounds(f: &[&str]) -> Option<[f64; 4]> {
    Some([field(f, 3)?, field(f, 4)?, field(f, 5)?, field(f, 6)?])
}

fn round4(n: f64) -> f64 {
    (n * 10_000.0).round() / 10_000.0
}

/// A unit from its GUID, name and flags fields.
fn unit(guid: &str, name: &str, flags: &str) -> Unit {
    let flags = u32::from_str_radix(flags.trim_start_matches("0x"), 16).unwrap_or(0);
    let kind = if flags & TYPE_PLAYER != 0 {
        UnitKind::Player
    } else if flags & TYPE_PET != 0 {
        UnitKind::Pet
    } else {
        UnitKind::Npc
    };
    let affiliation = if flags & AFFILIATION_MINE != 0 {
        Affiliation::Me
    } else if flags & AFFILIATION_PARTY != 0 {
        Affiliation::Party
    } else if flags & AFFILIATION_RAID != 0 {
        Affiliation::Raid
    } else {
        Affiliation::Outsider
    };

    // Players are named First-Surname-Mode-Region. Show "First Surname".
    let parts: Vec<&str> = name.split('-').collect();
    let player_name = match parts[..] {
        [first, surname, mode, region] if guid.starts_with("Player-") => Some(PlayerName {
            first_name: first.into(),
            surname: surname.into(),
            mode: mode.into(),
            region: region.into(),
        }),
        _ => None,
    };
    Unit {
        guid: guid.into(),
        name: match &player_name {
            Some(player) => format!("{} {}", player.first_name, player.surname),
            None => name.into(),
        },
        kind,
        affiliation,
        player_name,
        owner_guid: None,
    }
}
