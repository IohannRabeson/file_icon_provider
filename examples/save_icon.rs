//! Extract and save the system icon associated with any file.
//!
//! Usage: cargo run --example save_icon <source_file> <output_name>
//! Example: cargo run --example save_icon document.pdf icon.png

use clap::Parser;
use file_icon_provider::get_file_icon;
use image::{DynamicImage, RgbaImage};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Extract and save the icon associated with any file.", long_about = None)]
struct Cli {
    /// The file we want to extract the icon.
    file_path: PathBuf,
    /// The file path of the extracted image.
    output_path: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let icon = get_file_icon(cli.file_path, 32).expect("Failed to get icon");
    let image = RgbaImage::from_raw(icon.width, icon.height, icon.pixels)
        .map(DynamicImage::ImageRgba8)
        .expect("Failed to convert Icon to Image");

    match image.save_with_format(&cli.output_path, image::ImageFormat::Png) {
        Err(error) => {
            println!("Failed to save the image: {}", error);
        }
        _ => println!("Saved image: '{}'", cli.output_path.display()),
    }
}
