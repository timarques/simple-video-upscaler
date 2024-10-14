#[derive(Clone, Copy, Debug)]
pub enum Model {
    RealCugan(u8),
    RealEsrAnime(u8),
    RealEsrgan,
    RealEsrganAnime,
}

impl Model {
    pub fn get_scale(&self) -> u8 {
        match self {
            Model::RealCugan(scale) | Model::RealEsrAnime(scale) => *scale,
            Model::RealEsrgan | Model::RealEsrganAnime => 4,
        }
    }
}

impl std::fmt::Display for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::RealCugan(scale) => write!(f, "realcugan-x{}", scale),
            Model::RealEsrAnime(scale) => write!(f, "realesr-anime-x{}", scale),
            Model::RealEsrgan => write!(f, "realesrgan-x4"),
            Model::RealEsrganAnime => write!(f, "realesrgan-anime-x4"),
        }
    }
}