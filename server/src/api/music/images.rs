use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer, SrcCropping};
use image::{DynamicImage, ImageBuffer};
use lofty::prelude::*;
use std::path::{Path, PathBuf};

const COVER_DIR: &str = "db/covers/";

pub fn get_embedded_cover(path: &Path) -> Option<DynamicImage> {
    let tagged_file = lofty::read_from_path(path).ok()?;
    let tag = tagged_file.primary_tag()?;
    let picture = tag.pictures().first()?;
    image::load_from_memory(picture.data()).ok()
}

pub fn get_image_from_file(path: &Path) -> Option<DynamicImage> {
    image::open(path).ok()
}

pub fn resize_and_save_cover(cover_id: i64, img: DynamicImage) {
    let img = img.to_rgb8();
    let mut resized_cover = DynamicImage::ImageRgb8(ImageBuffer::new(300, 300));
    let mut resizer = Resizer::new();
    let mut opts = ResizeOptions::new();
    opts.algorithm = ResizeAlg::Convolution(FilterType::Hamming);
    opts.cropping = SrcCropping::FitIntoDestination((300.0, 300.0));
    let _ = resizer.resize(&img, &mut resized_cover, Some(&opts));

    if let Ok(encoder) = webp::Encoder::from_image(&resized_cover) {
        let cover = encoder.encode(50.0);
        let cover_path = PathBuf::from(COVER_DIR).join(format!("{}.webp", cover_id));
        let _ = std::fs::create_dir_all(COVER_DIR);
        let _ = std::fs::write(cover_path, &*cover);
    }
}

pub fn generate_cover(cover_id: i64, source_path: PathBuf, is_embedded: bool) {
    log::info!(
        "generating cover {} from {} (embedded: {})",
        cover_id,
        source_path.display(),
        is_embedded
    );

    let img = if is_embedded {
        get_embedded_cover(&source_path)
    } else {
        get_image_from_file(&source_path)
    };

    if let Some(img) = img {
        resize_and_save_cover(cover_id, img);
    }
}
