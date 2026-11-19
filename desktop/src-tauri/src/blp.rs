//! Decodes the game's BLP textures, as far as maps need: DXT1, DXT3, and DXT5

/// An image's width, height and RGBA pixels, or `None` if it's another kind.
pub fn decode(blp: &[u8]) -> Option<(usize, usize, Vec<u8>)> {
    let u32_at =
        |at: usize| Some(u32::from_le_bytes(blp.get(at..at + 4)?.try_into().ok()?) as usize);
    if blp.get(0..4)? != b"BLP2" || blp[8] != 2 {
        return None;
    }
    let (alpha_bits, kind) = (blp[9], blp[10]);
    let block_size = match kind {
        0 => 8,
        1 | 7 => 16,
        _ => return None,
    };
    let (width, height) = (u32_at(12)?, u32_at(16)?);
    let (offset, size) = (u32_at(20)?, u32_at(84)?);
    let blocks = blp.get(offset..offset + size)?;

    let mut rgba = vec![0; width * height * 4];
    let across = width.div_ceil(4);
    for (i, block) in blocks
        .chunks_exact(block_size)
        .enumerate()
        .take(across * height.div_ceil(4))
    {
        let (alpha, color) = block.split_at(block_size - 8);
        let pixels = colors(color, kind == 0, alpha_bits > 0);
        let alphas = match kind {
            1 => explicit_alpha(alpha),
            7 => interpolated_alpha(alpha),
            _ => pixels.map(|p| p[3]),
        };
        for p in 0..16 {
            let (x, y) = ((i % across) * 4 + p % 4, (i / across) * 4 + p / 4);
            if x < width && y < height {
                let at = (y * width + x) * 4;
                rgba[at..at + 3].copy_from_slice(&pixels[p][..3]);
                rgba[at + 3] = alphas[p];
            }
        }
    }
    Some((width, height, rgba))
}

/// A block's 16 colors. In DXT1, when the first color is no greater than the
/// second, there are only three, and the fourth is black or see-through.
fn colors(block: &[u8], dxt1: bool, see_through: bool) -> [[u8; 4]; 16] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (a, b) = (rgb(c0), rgb(c1));
    let mix = |wa: u32, wb: u32| {
        let channel = |i: usize| ((u32::from(a[i]) * wa + u32::from(b[i]) * wb) / (wa + wb)) as u8;
        [channel(0), channel(1), channel(2), 255]
    };
    let palette = if !dxt1 || c0 > c1 {
        [
            [a[0], a[1], a[2], 255],
            [b[0], b[1], b[2], 255],
            mix(2, 1),
            mix(1, 2),
        ]
    } else {
        let last = if see_through {
            [0, 0, 0, 0]
        } else {
            [0, 0, 0, 255]
        };
        [
            [a[0], a[1], a[2], 255],
            [b[0], b[1], b[2], 255],
            mix(1, 1),
            last,
        ]
    };
    let indices = u32::from_le_bytes([block[4], block[5], block[6], block[7]]);
    std::array::from_fn(|p| palette[(indices >> (p * 2)) as usize & 3])
}

/// DXT3: 4 bits of alpha for each pixel.
fn explicit_alpha(block: &[u8]) -> [u8; 16] {
    let bits = u64::from_le_bytes(block.try_into().unwrap());
    std::array::from_fn(|p| ((bits >> (p * 4)) & 15) as u8 * 17)
}

/// DXT5: two alphas, and 3 bits a pixel to pick one or a mix of them.
fn interpolated_alpha(block: &[u8]) -> [u8; 16] {
    let (a0, a1) = (u32::from(block[0]), u32::from(block[1]));
    let levels: [u8; 8] = std::array::from_fn(|i| {
        let i = i as u32;
        match i {
            0 => a0 as u8,
            1 => a1 as u8,
            _ if a0 > a1 => (((8 - i) * a0 + (i - 1) * a1) / 7) as u8,
            6 => 0,
            7 => 255,
            _ => (((6 - i) * a0 + (i - 1) * a1) / 5) as u8,
        }
    });
    let bits = block[2..8]
        .iter()
        .rev()
        .fold(0u64, |n, &b| n << 8 | u64::from(b));
    std::array::from_fn(|p| levels[((bits >> (p * 3)) & 7) as usize])
}

fn rgb(c: u16) -> [u8; 3] {
    let (r, g, b) = ((c >> 11) & 31, (c >> 5) & 63, c & 31);
    [
        (r << 3 | r >> 2) as u8,
        (g << 2 | g >> 4) as u8,
        (b << 3 | b >> 2) as u8,
    ]
}