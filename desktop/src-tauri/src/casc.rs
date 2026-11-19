//! Reads files from the game's own storage (CASC) in the WoW folder, bu
//! FileDataID. It only reads what's installed: no downloads, no encrypted files.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use flate2::read::ZlibDecoder;

/// An index entry: 9 bytes of key, 5 of where the file is, 4 of its size.
const ENTRY: usize = 18;

/// The storage a flavor of the game (like `_forever_`) plays from.
pub struct Storage {
    archives: PathBuf,
    buckets: Vec<Vec<u8>>,
    encoding: Vec<u8>,
    root: Vec<u8>,
}

impl Storage {
    /// Opens the storage the flavor folder `flavor` plays from.
    pub fn open(flavor: &Path) -> io::Result<Self> {
        let build = build_info(flavor)?;
        let key = build
            .get("Build Key")
            .ok_or_else(|| invalid(".build.info has no build key"))?;
        let data = flavor
            .parent()
            .ok_or_else(|| invalid("the flavor folder has no parent"))?
            .join("Data");
        let config = fs::read_to_string(
            data.join("config")
                .join(&key[0..2])
                .join(&key[2..4])
                .join(key),
        )?;
        let config_key = |name: &str, nth: usize| {
            config
                .lines()
                .find_map(|line| line.strip_prefix(name)?.strip_prefix(" = "))
                .and_then(|keys| parse_key(keys.split(' ').nth(nth)?))
                .ok_or_else(|| invalid(&format!("the build config has no {name}")))
        };

        let archives = data.join("data");
        let mut storage = Self {
            buckets: read_buckets(&archives)?,
            archives,
            encoding: Vec::new(),
            root: Vec::new(),
        };
        storage.encoding = storage.read(&config_key("encoding", 1)?)?;
        let root = storage
            .ekey(&config_key("root", 0)?)
            .ok_or_else(|| invalid("the root file isn't in the encoding table"))?;
        storage.root = storage.read(&root)?;
        Ok(storage)
    }

    /// The files with these FileDataIDs. One that isn;'t installed is left out.
    pub fn read_files(&self, ids:&HashSet<u32>) -> HashMap<u32, Vec<u8>> {
        let mut files = HashMap::new();
        // a file can have on entry per language. The first installed one wins.
        walk_root(&self.root, |id, ckey| {
            if ids.contains(&id)
                && !files.contains_key(&id)
                && let Some(bytes) = self.ekey(ckey).and_then(|ekey| self.read(&ekey).ok())
            {
                files.insert(id, bytes);
            }
        });
        files
    }

    /// The encoding key the file with content key `ckey` is stored under.
    fn ekey(&self, ckey: &[u8]) -> Option<[u8; 16]> {
        let e = &self.encoding;
        if e.get(0..2)? != b"EN" {
            return None;
        }
        let page_size = usize::from(u16::from_be_bytes([e[5], e[6]])) * 1024;
        let page_count = u32::from_be_bytes(e[9..13].try_into().ok()?) as usize;
        let index = 22 + u32::from_be_bytes(e[18..22].try_into().ok()?) as usize;
        // Pages are in key order, and the index has each one's first key.
        let page = lower_bound(
            page_count,
            |i| &e[index + i * 32..index + i * 32 + 16],
            ckey,
        );
        let page = if page < page_count && &e[index + page * 32..index + page * 32 + 16] == ckey {
            page
        } else {
            page.checked_sub(1)?
        };
        let mut at = index + page_count * 32 + page * page_size;
        let end = (at + page_size).min(e.len());
        // each entry: key count, 5 bytes of size, the content key, then its encoding keys.
        while at + 38 <= end && e[at] > 0 {
            if &e[at + 6..at + 22] == ckey {
                return e[at + 22..at + 38].try_into().ok();
            }
            at += 22 + usize::from(e[at]) * 16;
        }
        None
    }

    /// Reads and decodes the file stored under the encoding key `ekey`.
    fn read(&self, ekey: &[u8; 16]) -> io::Result<Vec<u8>> {
        let key = &ekey[..9];
        let bucket = &self.buckets[bucket_of(key)];
        let i = lower_bound(
            bucket.len() / ENTRY,
            |i| &bucket[i * ENTRY..i * ENTRY + 9],
            key,
        );
        let entry = bucket
            .get(i * ENTRY..(i + 1) * ENTRY)
            .filter(|entry| &entry[..9] == key)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "that file isn't installed"))?;
        // The top bits say which data.## file, the low 30 where in it.
        let place = entry[9..14]
            .iter()
            .fold(0u64, |n, &b| n << 8 | u64::from(b));
        let size = u32::from_le_bytes(entry[14..18].try_into().unwrap()) as usize;
        
        let mut file = File::open(self.archives.join(format!("data.{:03}", place >> 30)))?;
        file.seek(SeekFrom::Start(place & 0x3FFF_FFFF))?;
        let mut stored = vec![0; size];
        file.read_exact(&mut stored)?;
        // Each file has a 30-byte header before its BLTE data.
        blte(
            stored
                .get(30..)
                .ok_or_else(|| invalid("a stored file is too short"))?,
        )
    }
}

/// The row for `flavor`'s product in `.build.info`, by column name.
fn build_info(flavor: &Path) -> io::Result<HashMap<String, String>> {
    // .flavor.info's second line names the product, like "wowf"
    let flavor_info = fs::read_to_string(flavor.join(".flavor.info"))?;
    let product = flavor_info
        .lines()
        .nth(1)
        .map(str::trim)
        .ok_or_else(|| invalid(".flavor.info has no product"))?;
    let text = fs::read_to_string(flavor.parent().unwrap_or(flavor).join(".build.info"))?;
    let mut lines = text.lines();
    // Cols look like "Build Key!HEX:16"
    let columns : Vec<&str> = lines
        .next()
        .unwrap_or("")
        .split('|')
        .map(|c| c.split('!').next().unwrap_or(""))
        .collect();
    lines
        .map(|line| {
            columns
                .iter()
                .zip(line.split('|'))
                .map(|(c, v)| (c.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>()
        })
        .find(|row| row.get("Product").map(String::as_str) == Some(product))
        .ok_or_else(|| invalid(&format!("{product} isn't in .build.info")))
}

/// the game's version, like "1.60.1.70959", which changes with each patch.
pub fn version(flavor: &Path) -> io::Result<String> {
    build_info(flavor)?
        .remove("Version")
        .ok_or_else(|| invalid(".build.info has no version"))
}

/// Calls `each(FileDataID, content key)` for every file in the root.
fn walk_root(root: &[u8], mut each: impl FnMut(u32, &[u8])) {
    let u32_at = |at: usize| u32::from_le_bytes(root[at..at + 4].try_into().unwrap());
    if root.get(0..4) != Some(b"TSFM") {
        return;
    }
    // Newer roots start with their header's size, older ones with a file count.
    let (mut at, version) = if u32_at(4) < 100 {
        (u32_at(4) as usize, u32_at(8))
    } else {
        (12, 0)
    };
    while at + 12 <= root.len() {
        let count = u32_at(at) as usize;
        let flags = if version >= 2 {
            at += 17;
            u32_at(at - 9) | u32_at(at - 5) | u32::from(root[at - 1]) << 17
        } else {
            at += 12;
            u32_at(at -8)
        };
        let (deltas, ckeys) = (at, at + count * 4);
        // IDs are stored as gaps from the one before.
        let mut id = -1i64;
        for i in 0..count {
            id += 1 + i64::from(i32::from_le_bytes(
                root[deltas + i * 4..deltas + i * 4 + 4].try_into().unwrap(),
            ));
            each(id as u32, &root[ckeys + i * 16..ckeys + i * 16 + 16]);
        }
        // Then name hashes, unless the block has none.
        at = ckeys
            + count * 16
            + if flags & 0x1000_0000 == 0 {
                count * 8
            } else {
                0
            };
    }
}

/// Decodes BLTE: chunks stored as is, zlib-compressed, or nested.
fn blte(data: &[u8]) -> io::Result<Vec<u8>> {
    if data.get(0..4) != Some(b"BLTE") {
        return Err(invalid("a stored file isn't BLTE"));
    }
    let header = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
    let mut chunks = Vec::new();
    if header == 0 {
        chunks.push(&data[8..]);
    } else {
        let count = usize::from(data[9]) << 16 | usize::from(data[10]) << 8 | usize::from(data[11]);
        let mut at = header;
        for i in 0..count {
            let size =
                u32::from_be_bytes(data[12 + i * 24..16 + i * 24].try_into().unwrap()) as usize;
            chunks.push(
                data.get(at..at + size)
                    .ok_or_else(|| invalid("a BLTE chunk is cut short"))?,
            );
            at += size;
        }
    }
    let mut out = Vec::new();
    for chunk in chunks {
        match chunk.first() {
            Some(b'N') => out.extend_from_slice(&chunk[1..]),
            Some(b'Z') => {
                ZlibDecoder::new(&chunk[1..]).read_to_end(&mut out)?;
            }
            Some(b'F') => out.extend(blte(&chunk[1..])?),
            Some(&mode) => {
                return Err(invalid(&format!(
                    "BLTE mode {} isn't supported",
                    mode as char
                )));
            }
            None => {}
        }
    }
    Ok(out)
}

/// the newest index of each of the 16 buckets, as its sorted entries.
fn read_buckets(archives: &Path) -> io::Result<Vec<Vec<u8>>> {
    // Index files are named by bucket then version, like 0a00000024.idx
    let mut newest: Vec<Option<(u32, PathBuf)>> = vec![None; 16];
    for file in fs::read_dir(archives)? {
        let path = file?.path();
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str()?.strip_suffix(".idx"))
        else {
            continue;
        };
        let (Some(bucket), Some(version)) = (name.get(..2), name.get(2..)) else {
            continue;
        };
        let (Ok(bucket), Ok(version)) = (
            usize::from_str_radix(bucket, 16),
            u32::from_str_radix(version, 16),
        ) else {
            continue;
        };
        if let Some(slot) = newest.get_mut(bucket)
            && slot.as_ref().is_none_or(|(was, _)| version > *was)
        {
            *slot = Some((version, path));
        }
    }
    newest
        .into_iter()
        .map(|slot| {
            let Some((_, path)) = slot else {
                return Ok(Vec::new());
            };
            let bytes = fs::read(path)?;
            let size = u32::from_le_bytes(
                bytes
                    .get(0x20..0x24)
                    .ok_or_else(|| invalid("an index is too short"))?
                    .try_into()
                    .unwrap(),
            );
            Ok(bytes
                .get(0x28..0x28 + size as usize)
                .ok_or_else(|| invalid("an index is cut short"))?
                .to_vec())
        })
        .collect()
}

/// Which of the 16 buckets a key's index entry is in.
fn bucket_of(key: &[u8]) -> usize {
    let x = key.iter().fold(0, |x, b| x ^ b);
    usize::from((x & 0xF) ^ (x >> 4))
}

/// The first of `count` sorted keys that isn't less than `key`.
fn lower_bound<'a>(count: usize, key_at: impl Fn(usize) -> &'a [u8], key: &[u8]) -> usize {
    let (mut low, mut high) = (0, count);
    while low < high {
        let middle = (low + high) / 2;
        if key_at(middle) < key {
            low = middle + 1
        } else {
            high = middle
        }
    }
    low
}

fn parse_key(hex: &str) -> Option<[u8; 16]> {
    let mut key = [0; 16];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(key)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}