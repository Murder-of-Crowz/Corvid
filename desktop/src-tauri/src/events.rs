//! Events sent to the UI. This mirrors `desktop/src/data/event.ts`, so a change
//! to one needs the same change in the other. `tests/events.rs` pins the JSON.

use serde::Serialize;

/// The Tauri event the UI listens on. Each payload is a `Vec<CorvidEvent>`.
pub const EVENTS_CHANNEL: &str = "corvid://events";

/// Unix epoch ms, UTC.
pub type Timestamp = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnitKind {
    Player,
    Pet,
    Npc,
}

/// From the combat log's unit flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Affiliation {
    /// The player running the app (flags 0x511). `Self` is a Rust
    /// keyword, so the variant is `Me` and only the JSON says "self".
    #[serde(rename = "self")]
    Me,
    Party,
    Raid,
    Outsider,
}

/// The four parts of a player name such as `FirstName-Surname-GameMode-Region`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerName {
    pub first_name: String,
    pub surname: String,
    pub mode: String,
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub guid: String,
    pub name: String,
    pub kind: UnitKind,
    pub affiliation: Affiliation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub player_name: Option<PlayerName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_guid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Spell {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PositionSource {
    Strip,
    Log,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionEvent {
    pub ts: Timestamp,
    pub unit: Unit,
    pub map_id: u32,
    pub x: f64,
    pub y: f64,
    pub source: PositionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FightKind {
    Trash,
    Boss,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightStartEvent {
    pub ts: Timestamp,
    pub fight_id: String,
    pub kind: FightKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FightEndEvent {
    pub ts: Timestamp,
    pub fight_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageEvent {
    pub ts: Timestamp,
    /// Absent outside fights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fight_id: Option<String>,
    pub source: Unit,
    pub target: Unit,
    /// Absent for melee swings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell: Option<Spell>,
    /// Includes overkill, as the log reports it.
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overkill: Option<u64>,
    /// Left out of the JSON when false, like the optional field in event.ts.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealEvent {
    pub ts: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fight_id: Option<String>,
    pub source: Unit,
    pub target: Unit,
    pub spell: Spell,
    /// Includes overheal, as the log reports it.
    pub amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overheal: Option<u64>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeathEvent {
    pub ts: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fight_id: Option<String>,
    pub target: Unit,
}

/// Serialized with a `"kind"` field, inside an event whose `"type"` is `"combat"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CombatEvent {
    Damage(DamageEvent),
    Heal(HealEvent),
    Death(DeathEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogKind {
    Combat,
    Chat,
}

/// A log line sent as is: before the parser exists, and later for lines it skips.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RawLineEvent {
    pub ts: Timestamp,
    pub log: LogKind,
    pub line: String,
}

/// One event for the UI. Serialized with a `"type"` field naming the variant.
/// Chat isn't here yet; it's on hold, and Rust only needs what it sends.
// Combat events make every variant a few hundred bytes. They're built, sent and
// dropped, never stored in bulk, so boxing them isn't worth the extra noise.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CorvidEvent {
    Position(PositionEvent),
    FightStart(FightStartEvent),
    FightEnd(FightEndEvent),
    Combat(CombatEvent),
    RawLine(RawLineEvent),
}
