use image::{Rgb, RgbImage};
use macros::include_palettes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

type UMatrix<const N: usize> = [[usize; N]; N];

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum DitherPattern {
    #[default]
    Dither4x4,
}

#[derive(Debug, Error)]
pub enum DitherPatternError {
    #[error("Unsupported dither size")]
    UnsupportedSize,
}

impl DitherPattern {
    #[rustfmt::skip]
    const BAYER4: UMatrix<4> = [
        [0,  8, 2,10],
        [12, 4,14, 6],
        [ 3,11, 1, 9],
        [15, 7,13, 5],
    ];

    pub const fn new(size: usize) -> Result<Self, DitherPatternError> {
        match size {
            4 => Ok(DitherPattern::Dither4x4),
            _ => Err(DitherPatternError::UnsupportedSize),
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<usize> {
        match self {
            DitherPattern::Dither4x4 => Self::BAYER4.get(y).and_then(|row| row.get(x)).copied(),
        }
    }

    pub const fn size(&self) -> usize {
        match self {
            DitherPattern::Dither4x4 => 4,
        }
    }

    pub const fn max_level(&self) -> usize {
        self.size().pow(2) - 1
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DitherSettings {
    pattern: DitherPattern,
    level: usize,
}

impl DitherSettings {
    pub const fn new(size: DitherPattern, level: usize) -> Self {
        assert!(level <= size.max_level(), "Invalid dither level");
        Self {
            pattern: size,
            level,
        }
    }
    pub const fn size(&self) -> usize {
        self.pattern.size()
    }
    pub const fn level(&self) -> usize {
        self.level
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Palette {
    pub name: &'static str,
    #[serde(serialize_with = "serde_rgb")]
    pub primary: Rgb<u8>,
    #[serde(serialize_with = "serde_rgb")]
    pub secondary: Rgb<u8>,
}

fn serde_rgb<S>(color: &Rgb<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let hex_string = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
    serializer.serialize_str(&hex_string)
}

impl Palette {
    const BW_PALETTE: Palette = Palette {
        name: "b&w",
        primary: Rgb([0, 0, 0]),
        secondary: Rgb([255, 255, 255]),
    };
    const PALETTES: &[Palette] = include_palettes!("palettes");

    pub fn random() -> Self {
        use rand::seq::IndexedRandom;

        Self::PALETTES
            .choose(&mut rand::rng())
            .copied()
            .unwrap_or(Palette::BW_PALETTE)
    }
}

pub struct DitherImage {
    pub img: RgbImage,
    // palette: Palette,
    // dither: DitherSettings,
}

impl DitherImage {
    pub fn new(palette: Palette, dither: DitherSettings) -> Self {
        Self::new_with_scale(palette, dither, 1)
    }

    pub fn new_with_scale(palette: Palette, dither: DitherSettings, scale: usize) -> Self {
        let size = (dither.size() * scale) as u32;
        let img = RgbImage::from_fn(size, size, |x, y| {
            let x = x as usize / scale;
            let y = y as usize / scale;

            let bayer_val = dither.pattern.get(x, y).unwrap();

            if bayer_val < dither.level {
                palette.primary
            } else {
                palette.secondary
            }
        });

        Self {
            img,
            // palette,
            // dither,
        }
    }

    pub fn to_data_url(&self) -> String {
        use base64::prelude::*;
        use image::ImageFormat;
        use std::io::Cursor;

        let mut buf = Vec::new();
        self.img
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();

        let mut output = String::from("data:image/png;base64,");
        BASE64_STANDARD_NO_PAD.encode_string(buf, &mut output);
        output
    }

    pub fn to_png_bytes(&self) -> Vec<u8> {
        use image::ImageFormat;
        use std::io::Cursor;

        let mut buf = Vec::new();
        self.img
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        buf
    }
}
