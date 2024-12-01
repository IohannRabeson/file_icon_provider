# File Icon Provider

[![version](https://img.shields.io/crates/v/file_icon_provider.svg)](https://crates.io/crates/file_icon_provider)
[![Documentation](https://docs.rs/file_icon_provider/badge.svg)](https://docs.rs/file_icon_provider)

**File Icon Provider** is a cross-platform Rust library designed to simplify the retrieval of file icons on Windows, MacOS and Linux (Gnome).

## Features

- **Simple Functionality**: Use the `get_file_icon` function to retrieve the icon for a specific file path.
- **Efficient Caching**: Leverage the `FileIconProvider` struct to cache icons based on file extensions, reducing repetitive lookups and improving performance.

## Examples
```rust
//! Extract and save the system icon associated with any file.
//!
//! Usage: cargo run --example save_icon <source_file> <output_name>
//! Example: cargo run --example save_icon document.pdf icon.png

use clap::Parser;
use file_icon_provider::get_file_icon;
use image::{DynamicImage, RgbaImage};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Retrieve and save the icon associated with any file.", long_about = None)]
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
```

Examples are available in the `examples` directory.

## Installation

On Linux you need to install theses packages:
```
sudo apt install libgtk-4-dev libgtk-3-dev libatk1.0-dev
```
