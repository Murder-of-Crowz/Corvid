//! Reads where you and your party are from the strip of squares the addon
//! paints in the game's top-left corner. `addon/Corvid.lua` describes it.

use std::collections::HashMap;

use crate::events::{Affiliation, PositionEvent, PositionSource, Timestamp, Unit, UnitKind};

/// Squares in the strip: two markers, then 19 holding the data.
pub const SQUARES: usize = 21;
/// Squares in each party member's row.
pub const PARTY_SQUARES: usize = 30;
/// Party rows under yours, for party1 to party4
pub const PARTY_ROWS: usize = 4;
/// Each square's width and height in screen pixels.
const SQUARE_PX: usize = 3;
/// Positions from the log come at most once a second, so these do too.
const READ_INTERVAL_MS: Timestamp = 1000;

/// A square's color, 0-255 per channel, as a screen capture sees it.
pub type Rgb = [u8; 3];

/// Every square in the strip, row by row.
pub struct Squares {
    pub you: [Rgb; SQUARES],
    pub party: [[Rgb; PARTY_SQUARES]; PARTY_ROWS]
}

/// Where the strip says you are: a uiMapID and 0-1 coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripPosition {
    pub map_id: u32,
    pub x: f64,
    pub y: f64,
}

/// Your row: where you are, `none` for map 0, and wheter party rows follow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YourRow {
    pub position: Option<StripPosition>,
    pub party: bool,
}

/// A party member on your map, from their row.
#[derive(Debug, Clone, PartialEq)]
pub struct PartyMember {
    pub guid: String,
    pub x: f64,
    pub y: f64,
}

/// Reads the strip off the screen, remembering what it read last.
#[derive(Default)]
pub struct Strip {
    read_at: Option<Timestamp>,
    last: Option<StripPosition>,
    party: HashMap<String, (usize, StripPosition)>,
}

impl Strip {
    pub fn new() -> Self {
        Self::default()
    }

    /// Where you and your party are, read once a second, for whoever moved.
    /// A party member who left your map or the group gets a position with map
    /// 0. `group` names the party members the log has seen. Nothing when it
    /// isn't time yet or the game isn't showing the strip.
    pub fn read(
        &mut self,
        now: Timestamp,
        me: &Unit,
        group: &HashMap<String, Unit>,
    ) -> Vec<PositionEvent> {
        if self.read_at.is_some_and(|at| now - at < READ_INTERVAL_MS) {
            return Vec::new();
        }
        self.read_at = Some(now);
        match capture::squares() {
            Some(squares) => self.read_squares(now, me, group, &squares),
            None => Vec::new(),
        }
    }

    /// What `read` does with the squares it captures.
    pub fn read_squares(
        &mut self,
        now: Timestamp,
        me: &Unit,
        group: &HashMap<String, Unit>,
        squares: &Squares,
    ) -> Vec<PositionEvent> {
        let Some(yours) = decode(&squares.you) else {
            return Vec::new();
        };
        let mut events = Vec::new();
        if let Some(position) = yours.position
            && self.last != Some(position)
        {
            if self.last.is_none() {
                println!("[corvid] reading your position from the game")
            }
            self.last = Some(position);
            events.push(position_event(now, me.clone(), position));
        }

        // Who's on your map now. A row that fails its checksum keeps whoever
        // was in it, since it may only be covered for a moment.
        let map_id = yours.position.map_or(0, |position| position.map_id);
        let mut party = HashMap::new();
        for (row, row_squares) in squares.party.iter().enumerate() {
            let slot = if yours.party { decode_party(row_squares) } else { Some(None) };
            match slot {
                None => party.extend(
                    self.party
                        .iter()
                        .filter(|(_, (was_row, _))| *was_row == row)
                        .map(|(guid, seen)| (guid.clone(), *seen)),
                ),
                Some(Some(member)) if map_id != 0 => {
                    let position = StripPosition { map_id, x: member.x, y: member.y };
                    party.insert(member.guid, (row, position));
                }
                Some(_) => {}
            }
        }

        for (guid, (_, position)) in &party {
            if self.party.get(guid).map(|(_, was)| was) != Some(position) {
                events.push(position_event(now, member(group, guid), *position));
            }
        }
        for guid in self.party.keys().filter(|guid| !party.contains_key(*guid)) {
            let gone = StripPosition { map_id: 0 , x: 0.0, y: 0.0 };
            events.push(position_event(now, member(group, guid), gone));
        }
        self.party = party;
        events
    }
}

fn position_event(ts: Timestamp, unit: Unit, position: StripPosition) -> PositionEvent {
    PositionEvent {
        ts,
        unit,
        map_id: position.map_id,
        x: position.x,
        y: position.y,
        source: PositionSource::Strip,
    }
}

fn member(group: &HashMap<String, Unit>, guid: &str) -> Unit {
    group.get(guid).cloned().unwrap_or_else(|| Unit {
        guid: guid.to_string(),
        name: "Party member".to_string(),
        kind: UnitKind::Player,
        affiliation: Affiliation::Party,
        player_name: None,
        owner_guid: None,
    })
}

/// Your row, left to right. `None` when the markers are missing or the
/// checksum fails, so anything else in the corner is ignored.
pub fn decode(squares: &[Rgb; SQUARES]) -> Option<YourRow> {
    let bits = bits(squares);
    // Magenta, then green.
    if bits[..6] != [1, 0, 1, 0, 1, 0] {
        return None;
    }
    let bytes = checked_bytes(&bits[6..], 6)?;
    let map_id = bytes[0] * 256 + bytes[1];
    Some(YourRow {
        position: (map_id != 0).then(|| StripPosition {
            map_id,
            x: coordinate(bytes[2], bytes[3]),
            y: coordinate(bytes[4], bytes[5]),
        }),
        party: bits[62] == 1,
    })
}

pub fn decode_party(squares: &[Rgb; PARTY_SQUARES]) -> Option<Option<PartyMember>> {
    let bytes = checked_bytes(&bits(squares), 10)?;
    let realm = bytes[0] * 256 + bytes[1];
    let id = bytes[2..6].iter().fold(0, |n, byte| n * 256 + byte);
    Some((realm != 0 || id != 0).then(|| PartyMember {
        guid: format!("Player-{realm}-{id:08X}"),
        x: coordinate(bytes[6], bytes[7]),
        y: coordinate(bytes[8], bytes[9]),
    }))
}

/// The bits in some squares, 3 a square: red, green, blue. Each channel is
/// fully on or off, so halfway tells them apart whatever the gamma or brightness.
fn bits(squares: &[Rgb]) -> Vec<u32> {
    squares
        .iter()
        .flatten()
        .map(|&channel| u32::from(channel >= 128))
        .collect()
}

/// The first `count` bytes in `bits`, highest bit first, if the byte after them
/// is their checksum.
fn checked_bytes(bits: &[u32], count: usize) -> Option<Vec<u32>> {
    let mut bytes: Vec<u32> = bits[..(count + 1) * 8]
        .chunks(8)
        .map(|byte| byte.iter().fold(0, |n, bit| n * 2 + bit))
        .collect();
    let checksum = bytes.pop()?;
    let sum: u32 = bytes.iter().sum();
    (checksum == 255 - sum % 256).then_some(bytes)
}

/// Two bytes as 0-1, to 4 decimals like positions from the log.
fn coordinate(high: u32, low: u32) -> f64 {
    let n = f64::from(high * 256 + low) / 65535.0;
    (n * 10_000.0).round() / 10_000.0
}

#[cfg(windows)]
mod capture {
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, ClientToScreen, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, IsIconic};

    use super::{PARTY_ROWS, PARTY_SQUARES, Rgb, SQUARE_PX, Squares};

    const WIDTH: usize = PARTY_SQUARES * SQUARE_PX;
    const HEIGHT: usize = (1 + PARTY_ROWS) * SQUARE_PX;

    /// The color in the middle of each square, or `None` when the game isn't
    /// open or is minimized.
    pub fn squares() -> Option<Squares> {
        let pixels = grab(game_corner()?)?;
        let middle = SQUARE_PX / 2;
        let square = |row: usize, i: usize| -> Rgb {
            let at = ((row * SQUARE_PX + middle) * WIDTH + i * SQUARE_PX + middle) *4;
            [pixels[at + 2], pixels[at + 1], pixels[at]]
        };
        Some(Squares {
            you: std::array::from_fn(|i| square(0, i)),
            party: std::array::from_fn(|row| std::array::from_fn(|i| square(row + 1, i))),
        })
    }

    /// Where the inside of the game window starts on screen.
    fn game_corner() -> Option<POINT> {
        let class = wide("GxWindowClass");
        let title = wide("World of Warcraft");
        // SAFETY: both strings end in a 0 and live until the calls return.
        unsafe {
            let mut window = FindWindowW(class.as_ptr(), null());
            if window.is_null() {
                window = FindWindowW(null(), title.as_ptr());
            }
            if window.is_null() || IsIconic(window) != 0 {
                return None;
            }
            let mut corner = POINT { x: 0, y: 0 };
            (ClientToScreen(window, &mut corner) != 0).then_some(corner)
        }
    }

    /// The strip's pixels on screen from `corner`: 4 bytes each, top row first.
    /// It copies from the screen, not the game window, because GDI often sees
    /// a game's window as black.
    fn grab(corner: POINT) -> Option<Vec<u8>> {
        let mut pixels = vec![0; WIDTH * HEIGHT * 4];
        // SAFETY: everything made here is freed before returning, and GetDIBits
        // writes at most HEIGHT rows of WIDTH 4-byte pixels into `pixels`.
        unsafe {
            let screen = GetDC(null_mut());
            let memory = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, WIDTH as i32, HEIGHT as i32);
            let previous = SelectObject(memory, bitmap);
            let copied = BitBlt(
                memory, 0, 0, WIDTH as i32, HEIGHT as i32, screen, corner.x, corner.y, SRCCOPY,
            ) != 0;
            SelectObject(memory, previous);

            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = WIDTH as i32;
            info.bmiHeader.biHeight = -(HEIGHT as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let rows = GetDIBits(
                memory,
                bitmap,
                0,
                HEIGHT as u32,
                pixels.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            );

            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(null_mut(), screen);
            (copied && rows == HEIGHT as i32).then_some(pixels)
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain([0]).collect()
    }
}

/// Only Windows can read the strip so far.
#[cfg(not(windows))]
mod capture {
    pub fn squares() -> Option<[super::Rgb; super::SQUARES]> {
        None
    }
}