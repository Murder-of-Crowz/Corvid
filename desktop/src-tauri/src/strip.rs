//! Reads your live position from the strip of squares the addon paints in the
//! game's top-left corner. `addon/Corvid.lua` describes the strip.

use crate::events::{PositionEvent, PositionSource, Timestamp, Unit};

/// Squares in the strip: two markers, then 19 holding the data.
pub const SQUARES: usize = 21;
/// Each square's width and height in screen pixels.
const SQUARE_PX: usize = 3;
/// Positions from the log come at most once a second, so these do too.
const READ_INTERVAL_MS: Timestamp = 1000;

/// A square's color, 0-255 per channel, as a screen capture sees it.
pub type Rgb = [u8; 3];

/// Where the strip says you are: a uiMapID and 0-1 coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripPosition {
    pub map_id: u32,
    pub x: f64,
    pub y: f64,
}

/// Reads the strip off the screen, remembering what it read last.
#[derive(Default)]
pub struct Strip {
    read_at: Option<Timestamp>,
    last: Option<StripPosition>,
}

impl Strip {
    pub fn new() -> Self {
        Self::default()
    }

    /// Your position, read once a second and sent when you've moved. `None`
    /// when it isn't time yet, you haven't moved, or the game isn't showing
    /// the strip.
    pub fn read(&mut self, now: Timestamp, me: &Unit) -> Option<PositionEvent> {
        if self.read_at.is_some_and(|at| now - at < READ_INTERVAL_MS) {
            return None;
        }
        self.read_at = Some(now);
        let position = decode(&capture::squares()?)?;
        if self.last == Some(position) {
            return None;
        }
        if self.last.is_none() {
            println!("[corvid] reading your position from the game");
        }
        self.last = Some(position);
        Some(PositionEvent {
            ts: now,
            unit: me.clone(),
            map_id: position.map_id,
            x: position.x,
            y: position.y,
            source: PositionSource::Strip,
        })
    }
}

/// The position in a strip's squares, left to right. `None` when the markers
/// are missing or the checksum fails, so anything else in the corner is
/// ignored, and when the strip says there's no position (map 0).
pub fn decode(squares: &[Rgb; SQUARES]) -> Option<StripPosition> {
    // Each channel is fully on or off, so halfway tells them apart whatever
    // the gamma or brightness.
    let bits: Vec<u32> = squares
        .iter()
        .flatten()
        .map(|&channel| u32::from(channel >= 128))
        .collect();
    // Magenta, then green.
    if bits[..6] != [1, 0, 1, 0, 1, 0] {
        return None;
    }
    // Seven bytes, highest bit first. The last bit is unused.
    let bytes: Vec<u32> = bits[6..62]
        .chunks(8)
        .map(|byte| byte.iter().fold(0, |n, bit| n * 2 + bit))
        .collect();
    let sum: u32 = bytes[..6].iter().sum();
    if bytes[6] != 255 - sum % 256 {
        return None;
    }
    let map_id = bytes[0] * 256 + bytes[1];
    let coordinate = |high: u32, low: u32| round4(f64::from(high * 256 + low) / 65535.0);
    (map_id !=0).then(|| StripPosition {
        map_id,
        x: coordinate(bytes[2], bytes[3]),
        y: coordinate(bytes[4], bytes[5]),
    })
}

/// To 4 decimals, like positions from the log.
fn round4(n: f64) -> f64 {
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

    use super::{Rgb, SQUARE_PX, SQUARES};

    const WIDTH: usize = SQUARES * SQUARE_PX;
    const HEIGHT: usize = SQUARE_PX;

    /// The color in the middle of each square, or `None` when the game isn't
    /// open or is minimized.
    pub fn squares() -> Option<[Rgb; SQUARES]> {
        let pixels = grab(game_corner()?)?;
        let middle = SQUARE_PX / 2;
        Some(std::array::from_fn(|i| {
            let at = (middle * WIDTH + i * SQUARE_PX + middle) * 4;
            [pixels[at + 2], pixels[at + 1], pixels[at]]
        }))
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