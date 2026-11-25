#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum Model {
    RealCugan(u8),
    RealEsrAnime(u8),
    RealEsrgan,
    RealEsrganAnime,
}

impl std::fmt::Display for Model {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::RealCugan(scale) => write!(formatter, "realcugan-x{scale}"),
            Model::RealEsrAnime(scale) => write!(formatter, "realesr-anime-x{scale}"),
            Model::RealEsrgan => write!(formatter, "realesrgan-x4"),
            Model::RealEsrganAnime => write!(formatter, "realesrgan-anime-x4"),
        }
    }
}
