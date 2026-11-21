//! Zone maps, drawn from the game's own files, the first time each si wanted.
//! fully explored, and kept as PNGs until the game is patched.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Mutex;

use crate::blp;
use crate::casc::{self, Storage};
use crate::db2::{self, Row};

const UI_MAP_X_MAP_ART: u32 = 1957217;
const UI_MAP_ART: u32 = 1957202;
const UI_MAP_ART_STYLE_LAYER: u32 = 1957208;
const UI_MAP_ART_TILE: u32 = 1957210;
const WORLD_MAP_OVERLAY: u32 = 1134579;
const WORLD_MAP_OVERLAY_TILE: u32 = 1957212;

const OVERLAY_TILE: usize = 256;

static DRAWING: Mutex<()> = Mutex::new(());

pub fn png(flavor: &Path, cache: &Path, map: u32) -> io::Result<Option<Vec<u8>>> {
    let folder = cache.join(casc::version(flavor)?);
    let path = folder.join(format!("{map}.png"));
    let _drawing = DRAWING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Ok(bytes) = fs::read(&path) {
        return Ok((!bytes.is_empty()).then_some(bytes));
    }
    let png = draw(&Storage::open(flavor)?, map)?;
    if let Err(error) = save(cache, &folder, &path, png.as_deref()) {
        eprintln!("[corvid] couldn't save map {map} in {}: {error}", folder.display());
    }
    Ok(png)
}

/// Saves a drawn map, or an empty file for a map the game doesn't have. The
/// first map for a new game version removes the old version's folders.
fn save(cache: &Path, folder: &Path, path: &Path, png: Option<&[u8]>) -> io::Result<()> {
    if !folder.exists() {
        if let Ok(old) = fs::read_dir(cache) {
            for entry in old.flatten() {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
        fs::create_dir_all(folder)?;
    }
    fs::write(path, png.unwrap_or_default())
}

struct Grid {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    tiles: Vec<(usize, usize, u32)>,
}

fn draw(storage: &Storage, map: u32) -> io::Result<Option<Vec<u8>>> {
    let Some((base, overlays)) = art(storage, map)? else {
        return Ok(None);
    };
    let files: HashSet<u32> = overlays
        .iter()
        .chain([&base])
        .flat_map(|grid| grid.tiles.iter().map(|tile| tile.2))
        .collect();
    let images: HashMap<u32, (usize, usize, Vec<u8>)> = storage
        .read_files(&files)
        .iter()
        .filter_map(|(&id, blp)| Some((id, blp::decode(blp)?)))
        .collect();

    let mut rgba = vec![0; base.width * base.height * 4];
    paint(&mut rgba, base.width, &base, &images, false);
    for overlay in &overlays {
        paint(&mut rgba, base.width, overlay, &images, true);
    }
    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, base.width as u32, base.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder
        .write_header()
        .and_then(|mut writer| writer.write_image_data(&rgba))
        .map_err(io::Error::other)?;
    Ok(Some(png))
}

fn paint(
    rgba: &mut [u8],
    width: usize,
    grid: &Grid,
    images: &HashMap<u32, (usize, usize, Vec<u8>)>,
    blend: bool,
) {
    let height = rgba.len() / 4 / width;
    for (row, column, file) in &grid.tiles {
        let Some((w, h, pixels)) = images.get(file) else {
            continue;
        };
        let (left, top) = (column * grid.tile_width, row * grid.tile_height);
        let across = (*w)
            .min(grid.tile_width)
            .min(grid.width.saturating_sub(left));
        let down = (*h)
            .min(grid.tile_height)
            .min(grid.height.saturating_sub(top));
        for y in 0..down {
            for x in 0..across {
                let (mx, my) = (grid.left + left + x, grid.top + top + y);
                if mx >= width || my >= height {
                    continue;
                }
                let from = &pixels[(y * w + x) * 4..][..4];
                let to = &mut rgba[(my * width + mx) * 4..][..4];
                if blend {
                    let alpha = u32::from(from[3]);
                    for c in 0..3 {
                        to[c] = ((u32::from(from[c]) * alpha + u32::from(to[c]) * (255 - alpha)) / 255) as u8;
                    }
                    to[3] = to[3].max(from[3]);
                } else {
                    to.copy_from_slice(from);
                }
            }
        }
    }
}

fn art(storage: &Storage, map: u32) -> io::Result<Option<(Grid, Vec<Grid>)>> {
    let ids = [
        UI_MAP_X_MAP_ART,
        UI_MAP_ART,
        UI_MAP_ART_STYLE_LAYER,
        UI_MAP_ART_TILE,
        WORLD_MAP_OVERLAY,
        WORLD_MAP_OVERLAY_TILE,
    ];
    let files = storage.read_files(&ids.into_iter().collect());
    let table = |id: u32| -> io::Result<Vec<Row>> {
        files
            .get(&id)
            .and_then(|bytes| db2::rows(bytes))
            .ok_or_else(|| io::Error::other(format!("couldn't read map table {id}")))
    };

    let mut links: Vec<Row> = table(UI_MAP_X_MAP_ART)?
        .into_iter()
        .filter(|row| row.parent == Some(map))
        .collect();
    links.sort_by_key(|row| row.fields[0] != 0);
    let Some(art) = links.first().map(|row| row.fields[1]) else {
        return Ok(None);
    };
    let Some(style) = table(UI_MAP_ART)?
        .iter()
        .find(|row| row.id == art)
        .map(|row| row.fields[2])
    else {
        return Ok(None);
    };
    let layers = table(UI_MAP_ART_STYLE_LAYER)?;
    let Some(layer) = layers
        .iter()
        .find(|row| row.parent == Some(style) && row.fields[0] == 0)
    else {
        return Ok(None);
    };

    let tiles_of = |tiles: &[Row], owner: u32| -> Vec<(usize, usize, u32)> {
        tiles
            .iter()
            .filter(|tile| tile.parent == Some(owner) && tile.fields[2] == 0)
            .map(|tile| {
                (
                    tile.fields[0] as usize,
                    tile.fields[1] as usize,
                    tile.fields[3],
                )
            })
            .collect()
    };
    let base = Grid {
        left: 0,
        top: 0,
        width: layer.fields[1] as usize,
        height: layer.fields[2] as usize,
        tile_width: layer.fields[3] as usize,
        tile_height: layer.fields[4] as usize,
        tiles: tiles_of(&table(UI_MAP_ART_TILE)?, art),
    };

    let overlay_tiles = table(WORLD_MAP_OVERLAY_TILE)?;
    let overlays = table(WORLD_MAP_OVERLAY)?
        .iter()
        .filter(|overlay| overlay.fields[1] == art)
        .map(|overlay| Grid {
            left: overlay.fields[4] as usize,
            top: overlay.fields[5] as usize,
            width: overlay.fields[2] as usize,
            height: overlay.fields[3] as usize,
            tile_width: OVERLAY_TILE,
            tile_height: OVERLAY_TILE,
            tiles: tiles_of(&overlay_tiles, overlay.id),
        })
        .collect();
    Ok(Some((base, overlays)))
}