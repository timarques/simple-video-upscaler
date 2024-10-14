use crate::error::Error;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::io::Cursor;
use image::{DynamicImage, ImageFormat};

static COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct Frame {
    pub index: usize,
    pub duplicates: usize,
    pub image: DynamicImage,
}

impl Frame {
    pub fn new(image: DynamicImage) -> Self {
        let index = COUNT.fetch_add(1, Ordering::Relaxed);
        Self {
            index,
            image,
            duplicates: 0
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)?;
        Ok(Self::new(image))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut cursor = Cursor::new(Vec::new());
        self.image.write_to(&mut cursor, ImageFormat::Png)?;
        Ok(cursor.into_inner())
    }

    pub fn add_duplicate(&mut self) {
        self.duplicates += 1;
    }

    pub fn is_duplicate(&self, frame: &Frame) -> bool {
        let result = image_compare::rgb_hybrid_compare(&self.image.to_rgb8(), &frame.image.to_rgb8());
        if let Ok(result) = result {
            if result.score > 0.98 {
                return true
            }
        }
        false
    }
}

impl TryFrom<&[u8]> for Frame {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl TryInto<Vec<u8>> for &Frame {
    type Error = Error;

    fn try_into(self) -> Result<Vec<u8>, Self::Error> {
        self.to_bytes()
    }
}