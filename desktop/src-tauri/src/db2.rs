//! Reads the game's data tables (DB2 files in the WDC5 format): each row's ID,
//! its whole-number fields, and the row in another table it belongs to.

use std::collections::HashMap;

/// One row: its ID, each field's (first) value, and the row it belongs to.
#[derive(Debug, Clone)]
pub struct Row {
    pub id: u32,
    pub fields: Vec<u32>,
    pub parent: Option<u32>,
}

/// how a field is stored in each record.
struct Field {
    offset: usize,
    size: usize,
    compression: u32,
    values: [u32; 3],
    data: usize,
    data_size: usize,
}

/// Every row of a WDC5 table, or `None` if it's a kind this can't read.
pub fn rows(db2: &[u8]) -> Option<Vec<Row>> {
    let u32_at = |at: usize| Some(u32::from_le_bytes(db2.get(at..at + 4)?.try_into().ok()?));
    if db2.get(0..4)? != b"WDC5" {
        return None;
    }
    let field_count = u32_at(140)? as usize;
    let record_size = u32_at(144)? as usize;
    let flags = u16::from_le_bytes(db2.get(172..174)?.try_into().ok()?);
    let id_field = usize::from(u16::from_le_bytes(db2.get(174..176)?.try_into().ok()?));
    let (info_size, common_size, pallet_size) = (
        u32_at(188)? as usize,
        u32_at(192)? as usize,
        u32_at(196)? as usize,
    );
    let section_count = u32_at(200)? as usize;
    if flags & 1 != 0 {
        return None;
    }

    // After the header: 40 bytes a section, 4 a field, then each field's
    // storage info, the pallet data and the common data.
    let info_at = 204 + section_count * 40 + field_count * 4;
    let (mut pallet, mut common) = (info_at + info_size, info_at + info_size + pallet_size);
    let mut fields = Vec::new();
    for i in 0..info_size / 24 {
        let at = info_at + i * 24;
        let compression = u32_at(at + 8)?;
        let data_size = u32_at(at + 4)? as usize;
        let data = match compression {
            2 => {
                common += data_size;
                common - data_size
            }
            3 | 4 => {
                pallet += data_size;
                pallet - data_size
            }
            _ => 0,
        };
        fields.push(Field {
            offset: usize::from(u16::from_le_bytes(db2.get(at..at + 2)?.try_into().ok()?)),
            size: usize::from(u16::from_le_bytes(
                db2.get(at + 2..at + 4)?.try_into().ok()?,
            )),
            compression,
            values: [u32_at(at + 12)?, u32_at(at + 16)?, u32_at(at + 20)?],
            data,
            data_size,
        });
    }
    if common != info_at + info_size + pallet_size + common_size {
        return None;
    }

    let mut rows: Vec<Row> = Vec::new();
    for section in 0..section_count {
        let at = 204 + section * 40;
        let encrypted = db2.get(at..at + 8)? != [0; 8];
        let (start, count, strings) = (
            u32_at(at + 8)? as usize,
            u32_at(at + 12)? as usize,
            u32_at(at + 16)? as usize,
        );
        let (ids_size, parent_size, copies) = (
            u32_at(at + 24)? as usize,
            u32_at(at + 28)? as usize,
            u32_at(at + 36)? as usize,
        );
        let records = db2.get(start..start + count * record_size)?;
        if encrypted && records.iter().all(|&b| b == 0) {
            continue;
        }
        let ids_at = start + count * record_size + strings;
        let copies_at = ids_at + ids_size;
        let parents_at = copies_at + copies * 8;
        let mut parents = HashMap::new();
        if parent_size > 0 {
            for i in 0..u32_at(parents_at)? as usize {
                let entry = parents_at + 12 + i * 8;
                parents.insert(u32_at(entry + 4)? as usize, u32_at(entry)?);
            }
        }

        let first = rows.len();
        for (r, record) in records.chunks_exact(record_size).enumerate() {
            let fields = fields
                .iter()
                .map(|field| value(db2, record, field))
                .collect::<Option<Vec<_>>>()?;
            let id = if ids_size > 0 {
                u32_at(ids_at + r * 4)?
            } else {
                *fields.get(id_field)?
            };
            rows.push(Row {
                id,
                fields,
                parent: parents.get(&r).copied(),
            });
        }
        for (i, field) in fields
            .iter()
            .enumerate()
            .filter(|(_, field)| field.compression == 2)
        {
            let pairs = db2.get(field.data..field.data + field.data_size)?;
            let changed: HashMap<u32, u32> = pairs
                .as_chunks::<8>()
                .0
                .iter()
                .map(|pair| {
                    (
                        u32::from_le_bytes(pair[..4].try_into().unwrap()),
                        u32::from_le_bytes(pair[4..].try_into().unwrap()),
                    )
                })
                .collect();
            for row in &mut rows[first..] {
                row.fields[i] =  changed.get(&row.id).copied().unwrap_or(field.values[0]);
            }
        }

        for c in 0..copies {
            let (new_id, from) = (u32_at(copies_at + c * 8)?, u32_at(copies_at + c * 8 + 4)?);
            if let Some(row) = rows[first..].iter().find(|row| row.id == from).cloned() {
                rows.push(Row { id: new_id, ..row });
            }
        }
    }
    Some(rows)
}

/// A field's first value in `record`. Common data is filled in afterwards.
fn value(db2: &[u8], record: &[u8], field: &Field) -> Option<u32> {
    match field.compression {
        0 => {
            let bytes = record.get(field.offset / 8..field.offset / 8 + (field.size / 8).min(4))?;
            Some(bytes.iter().rev().fold(0, |n, &b| n << 8 | u32::from(b)))
        }
        1 | 5 => bits(record, field.offset, field.values[1]),
        2 => Some(field.values[0]),
        3 | 4 => {
            let length = if field.compression == 4 {
                field.values[2] as usize
            } else {
                1
            };
            let at =
                field.data + bits(record, field.offset, field.values[1])? as usize * length * 4;
            Some(u32::from_le_bytes(db2.get(at..at + 4)?.try_into().ok()?))
        }
        _ => None,
    }
}

/// `width` bits, `offset` bits into `record`.
fn bits(record: &[u8], offset: usize, width: u32) -> Option<u32> {
    let start = offset / 8;
    let mut bytes = [0; 8];
    let available = record.len().checked_sub(start)?.min(8);
    bytes[..available].copy_from_slice(&record[start..start + available]);
    let value = u64::from_le_bytes(bytes) >> (offset % 8);
    Some((value & ((1u64 << width) - 1)) as u32)
}