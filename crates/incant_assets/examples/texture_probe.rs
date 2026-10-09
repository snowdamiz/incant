//! Nonvisual format fixtures for an independent `ktx validate` run.
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba, RgbaImage};
use incant_assets::{TextureUsage, encode_ktx2, import_image};
use std::{fs, io::Cursor, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("output directory required")?,
    );
    fs::create_dir_all(&directory)?;
    let ldr = DynamicImage::ImageRgba8(
        RgbaImage::from_raw(3, 1, vec![0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 128]).unwrap(),
    );
    let hdr = DynamicImage::ImageRgba32F(
        ImageBuffer::<Rgba<f32>, Vec<f32>>::from_raw(2, 1, vec![4., 2., 0., 1., 0., 0.5, 1., 1.])
            .unwrap(),
    );
    for (name, img, format, usage) in [
        ("color", ldr.clone(), ImageFormat::Png, TextureUsage::Color),
        ("linear", ldr, ImageFormat::Png, TextureUsage::Linear),
        ("hdr", hdr, ImageFormat::OpenExr, TextureUsage::Color),
    ] {
        let mut encoded = Cursor::new(Vec::new());
        img.write_to(&mut encoded, format)?;
        let texture = import_image(encoded.get_ref(), usage)?;
        fs::write(
            directory.join(format!("{name}.ktx2")),
            encode_ktx2(&texture)?,
        )?;
        println!(
            "{name}: {}x{}, {} mips, {:?}",
            texture.width,
            texture.height,
            texture.levels.len(),
            texture.format
        );
    }
    Ok(())
}
