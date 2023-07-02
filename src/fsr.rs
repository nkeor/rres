pub enum Fsr {
    Ultra,
    Quality,
    Balanced,
    Performance,
}

impl TryFrom<&str> for Fsr {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_ref() {
            "ultra" => Ok(Self::Ultra),
            "quality" => Ok(Self::Quality),
            "balanced" => Ok(Self::Balanced),
            "performance" => Ok(Self::Performance),
            _ => Err(()),
        }
    }
}

impl Fsr {
    pub fn generate(&self, target_res: (u16, u16)) -> (u16, u16) {
        if target_res == (1920, 1080) {
            match self {
                Self::Ultra => (1477, 831),
                Self::Quality => (1280, 720),
                Self::Balanced => (1129, 635),
                Self::Performance => (960, 540),
            }
        } else if target_res == (2560, 1440) {
            match self {
                Self::Ultra => (1970, 1108),
                Self::Quality => (1706, 960),
                Self::Balanced => (1506, 847),
                Self::Performance => (1280, 720),
            }
        } else if target_res == (3440, 1440) {
            match self {
                Self::Ultra => (2646, 1108),
                Self::Quality => (2293, 960),
                Self::Balanced => (2024, 847),
                Self::Performance => (1720, 720),
            }
        } else if target_res == (3840, 2160) {
            match self {
                Self::Ultra => (2954, 1662),
                Self::Quality => (2560, 1440),
                Self::Balanced => (2259, 1270),
                Self::Performance => (1920, 1080),
            }
        } else {
            let factor = match self {
                Self::Ultra => 1.3f32,
                Self::Quality => 1.5f32,
                Self::Balanced => 1.7f32,
                Self::Performance => 2.0f32,
            };

            return (
                (f32::from(target_res.0) / factor).floor() as u16,
                (f32::from(target_res.1) / factor).floor() as u16,
            );
        }
    }
}
