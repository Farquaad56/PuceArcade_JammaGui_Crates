//! Captures d'écran PNG : image native APRÈS rotation, `<saves_dir>/<jeu>_<trame>.png`.

use std::path::{Path, PathBuf};

use puce_jamma::util::rotate;
use puce_jamma::video::VideoInfo;

/// Chemin de la capture d'un jeu à une trame donnée.
pub fn screenshot_path(saves_dir: &Path, game: &str, frame: u64) -> PathBuf {
    saves_dir.join(format!("{game}_{frame}.png"))
}

/// Écrit `pixels` (0x00RRGGBB, `info.width` × `info.height`) tournés selon `info.rotation`, en PNG RGB 8 bits.
pub fn write_png(path: &Path, pixels: &[u32], info: &VideoInfo) -> Result<(), String> {
    let (img, w, h) = rotate(pixels, info.width, info.height, info.rotation);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    }
    let file = std::fs::File::create(path).map_err(|e| format!("{} : {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    let mut data = Vec::with_capacity(img.len() * 3);
    for p in img {
        data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
    }
    writer.write_image_data(&data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma::board::JammaBoard;
    use puce_jamma::video::Rotation;
    use puce_jamma_testboard::TestBoard;

    fn read_png(path: &Path) -> (u32, u32, Vec<u32>) {
        let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
        let mut reader = dec.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        let px = buf[..info.buffer_size()]
            .chunks(3)
            .map(|c| (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
            .collect();
        (info.width, info.height, px)
    }

    #[test]
    fn vertical_test_pattern_is_rotated() {
        let mut b = TestBoard::new(Rotation::Rot270);
        b.run_frame();
        let v = b.video();
        let dir = std::env::temp_dir().join(format!("jamma_capture_{}", std::process::id()));
        let path = screenshot_path(&dir, "mire_v", 1);
        write_png(&path, v.pixels, &v.info).unwrap();
        let (w, h, px) = read_png(&path);
        assert_eq!((w, h), (224, 288));
        let (expected, _, _) = rotate(v.pixels, v.info.width, v.info.height, v.info.rotation);
        assert_eq!(px, expected);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_format() {
        assert!(screenshot_path(Path::new("s"), "ghouls", 1200).ends_with("ghouls_1200.png"));
    }
}
