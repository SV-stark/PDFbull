//! Colorspace definitions

#[derive(Debug, Clone)]
pub struct Colorspace {
    name: String,
    n: u8,
}

impl Colorspace {
    pub fn device_gray() -> Self {
        Self {
            name: "DeviceGray".into(),
            n: 1,
        }
    }
    pub fn device_rgb() -> Self {
        Self {
            name: "DeviceRGB".into(),
            n: 3,
        }
    }
    pub fn device_cmyk() -> Self {
        Self {
            name: "DeviceCMYK".into(),
            n: 4,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn n(&self) -> u8 {
        self.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_gray() {
        let cs = Colorspace::device_gray();
        assert_eq!(cs.name(), "DeviceGray");
        assert_eq!(cs.n(), 1);
    }

    #[test]
    fn test_device_rgb() {
        let cs = Colorspace::device_rgb();
        assert_eq!(cs.name(), "DeviceRGB");
        assert_eq!(cs.n(), 3);
    }

    #[test]
    fn test_device_cmyk() {
        let cs = Colorspace::device_cmyk();
        assert_eq!(cs.name(), "DeviceCMYK");
        assert_eq!(cs.n(), 4);
    }

    #[test]
    fn test_colorspace_clone() {
        let cs1 = Colorspace::device_rgb();
        let cs2 = cs1.clone();
        assert_eq!(cs1.name(), cs2.name());
        assert_eq!(cs1.n(), cs2.n());
    }

    #[test]
    fn test_colorspace_debug() {
        let cs = Colorspace::device_rgb();
        let debug = format!("{:?}", cs);
        assert!(debug.contains("DeviceRGB"));
        assert!(debug.contains("3"));
    }
}
