use std::fmt::Formatter;
use std::str::FromStr;
use std::{env, fmt};

/// Types of supported platforms.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOs,
    Windows,
    Unsupported,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Platform::Linux => "linux",
            Platform::MacOs => "darwin",
            Platform::Windows => "windows",
            Platform::Unsupported => "Unsupported-platform",
        };
        f.write_str(s)
    }
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "linux" => Ok(Platform::Linux),
            "macosx" => Ok(Platform::MacOs),
            "darwin" => Ok(Platform::MacOs),
            "windows" => Ok(Platform::Windows),
            s => Err(format!("unsupported platform {}", s)),
        }
    }
}

pub fn is_nixos() -> bool {
    std::path::Path::new("/etc/NIXOS").exists()
}

/// Read the current machine's platform.
pub fn platform() -> Platform {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Platform::Linux,
        ("linux", "aarch64") => Platform::Linux,
        ("macos", "x86_64") => Platform::MacOs,
        ("macos", "aarch64") => Platform::MacOs,
        ("windows", "x86_64") => Platform::Windows,
        _ => Platform::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::Linux);
    }

    #[test]
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::Linux);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOs);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::MacOs);
    }

    #[test]
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    fn get_platform() {
        assert_eq!(platform(), Platform::Windows);
    }
}
