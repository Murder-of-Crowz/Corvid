//! Tests decoding the game's BLP textures, which map tiles are stored as, on
//! small images made here. Each is one 4×4 block of DXT-compressed pixels.

use desktop_lib::blp;

/// DXT kinds, as the BLP header names them.
const DXT1: u8 = 0;
const DXT3: u8 = 1;
const DXT5: u8 = 7;

/// 5-6-5 colors.
const RED: u16 = 0xF800;
const BLUE: u16 = 0x001F;

/// A BLP2 image `width` by `height`, of one DXT block.
fn image(kind: u8, alpha_bits: u8, width: u32, height: u32, block: &[u8]) -> Vec<u8> {
    // The header is 148 bytes, then a 1024-byte palette DXT doesn't use.
    let mut blp = vec![0; 1172];
    blp[0..4].copy_from_slice(b"BLP2");
    blp[4..8].copy_from_slice(&1u32.to_le_bytes());
    blp[8] = 2; // DXT compressed
    blp[9] = alpha_bits;
    blp[10] = kind;
    blp[12..16].copy_from_slice(&width.to_le_bytes());
    blp[16..20].copy_from_slice(&height.to_le_bytes());
    // Where the full-size image starts, and its size.
    blp[20..24].copy_from_slice(&1172u32.to_le_bytes());
    blp[84..88].copy_from_slice(&(block.len() as u32).to_le_bytes());
    blp.extend_from_slice(block);
    blp
}

/// The color half of a block: two colors, then 2 bits a pixel picking one of
/// four. Every row is `row`, its first pixel in the lowest bits.
fn colors(c0: u16, c1: u16, row: u8) -> Vec<u8> {
    let mut block = Vec::new();
    block.extend_from_slice(&c0.to_le_bytes());
    block.extend_from_slice(&c1.to_le_bytes());
    block.extend_from_slice(&[row; 4]);
    block
}

/// Pixel `i` of a decoded image.
fn pixel(rgba: &[u8], i: usize) -> [u8; 4] {
    rgba[i * 4..i * 4 + 4].try_into().unwrap()
}

#[test]
fn dxt1_mixes_two_colors_into_four() {
    // Each row picks colors 0, 1, 2 and 3.
    let blp = image(DXT1, 0, 4, 4, &colors(RED, BLUE, 0b11_10_01_00));
    let (width, height, rgba) = blp::decode(&blp).unwrap();
    assert_eq!((width, height, rgba.len()), (4, 4, 64));
    assert_eq!(pixel(&rgba, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&rgba, 1), [0, 0, 255, 255]);
    assert_eq!(pixel(&rgba, 2), [170, 0, 85, 255]);
    assert_eq!(pixel(&rgba, 3), [85, 0, 170, 255]);
    assert_eq!(pixel(&rgba, 15), [85, 0, 170, 255]);
}

#[test]
fn dxt1_with_the_smaller_color_first_has_a_see_through_fourth() {
    let blp = image(DXT1, 1, 4, 4, &colors(BLUE, RED, 0b11_10_01_00));
    let (_, _, rgba) = blp::decode(&blp).unwrap();
    assert_eq!(pixel(&rgba, 2), [127, 0, 127, 255]);
    assert_eq!(pixel(&rgba, 3), [0, 0, 0, 0]);
}

#[test]
fn dxt1_without_alpha_has_a_black_fourth() {
    let blp = image(DXT1, 0, 4, 4, &colors(BLUE, RED, 0b11_10_01_00));
    let (_, _, rgba) = blp::decode(&blp).unwrap();
    assert_eq!(pixel(&rgba, 3), [0, 0, 0, 255]);
}

#[test]
fn dxt3_has_4_bits_of_alpha_a_pixel() {
    // Pixel 0 is see-through and pixel 1 solid. The rest are see-through.
    let mut block = vec![0xF0, 0, 0, 0, 0, 0, 0, 0];
    block.extend(colors(RED, BLUE, 0));
    let (_, _, rgba) = blp::decode(&image(DXT3, 4, 4, 4, &block)).unwrap();
    assert_eq!(pixel(&rgba, 0), [255, 0, 0, 0]);
    assert_eq!(pixel(&rgba, 1), [255, 0, 0, 255]);
    assert_eq!(pixel(&rgba, 2), [255, 0, 0, 0]);
}

#[test]
fn dxt5_mixes_two_alphas_and_always_has_four_colors() {
    // Alphas 255 and 0, so pixel 0 (level 1) is see-through, pixel 1
    // (level 2) is 6/7 solid, and the rest (level 0) are solid.
    let mut block = vec![255, 0, 1 | 2 << 3, 0, 0, 0, 0, 0];
    // The smaller color comes first, which only makes DXT1 see-through.
    block.extend(colors(BLUE, RED, 0b11_11_11_11));
    let (_, _, rgba) = blp::decode(&image(DXT5, 8, 4, 4, &block)).unwrap();
    assert_eq!(pixel(&rgba, 0), [170, 0, 85, 0]);
    assert_eq!(pixel(&rgba, 1), [170, 0, 85, 218]);
    assert_eq!(pixel(&rgba, 2), [170, 0, 85, 255]);
}

#[test]
fn an_image_smaller_than_a_block_is_cut_to_size() {
    let blp = image(DXT1, 0, 2, 2, &colors(RED, BLUE, 0));
    let (width, height, rgba) = blp::decode(&blp).unwrap();
    assert_eq!((width, height, rgba.len()), (2, 2, 16));
    assert!(rgba.chunks(4).all(|p| p == [255, 0, 0, 255]));
}

#[test]
fn other_kinds_of_image_arent_decoded() {
    let mut palettized = image(DXT1, 0, 4, 4, &colors(RED, BLUE, 0));
    palettized[8] = 1;
    assert_eq!(blp::decode(&palettized), None);
    assert_eq!(blp::decode(b"not a texture"), None);
}
