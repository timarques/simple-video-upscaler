use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Frame {
    pub index: usize,
    pub image: DynamicImage,
}

impl Frame {
    pub fn new(index: usize, image: DynamicImage) -> Self {
        Self { index, image }
    }

    pub fn from_bytes(index: usize, bytes: &[u8]) -> Result<Self> {
        let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .with_context(|| "Failed to load image from bytes")?;

        Ok(Self::new(index, image))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        self.image
            .write_to(&mut cursor, ImageFormat::Png)
            .with_context(|| "Failed to write image to bytes")?;

        Ok(cursor.into_inner())
    }
}
