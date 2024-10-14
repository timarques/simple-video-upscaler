use crate::error::Error;

use std::io::Cursor;
use image::{DynamicImage, ImageFormat};


pub struct Frame {
    pub index: usize,
    pub duplicates: usize,
    pub image: DynamicImage,
}

impl Frame {
    pub fn new(index: usize, image: DynamicImage) -> Self {
        Self {
            index,
            image,
            duplicates: 0
        }
    }

    pub fn from_bytes(index: usize, bytes: &[u8]) -> Result<Self, Error> {
        image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .map_err(|e| Error::new(format!("Failed to load image from bytes: {}", e)))
            .map(|image| Self::new(index, image))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(Vec::new());
        self.image.write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| Error::new(format!("Failed to write image to bytes: {}", e)))?;
        Ok(cursor.into_inner())
    }

    pub fn add_duplicate(&mut self) {
        self.duplicates += 1;
    }
}