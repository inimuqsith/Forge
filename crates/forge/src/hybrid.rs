
pub struct HybridAdapter {
    pub cachyos_enabled: bool,
    pub arch_enabled: bool,
}

impl HybridAdapter {
    pub fn new(cachyos_enabled: bool, arch_enabled: bool) -> Self {
        Self {
            cachyos_enabled,
            arch_enabled,
        }
    }

    pub fn lookup_cachyos_package(&self, pkgname: &str) -> Option<String> {
        if self.cachyos_enabled {
            Some(format!("https://mirror.cachyos.org/repo/x86_64_v4/cachyos_v4/{}.pkg.tar.zst", pkgname))
        } else {
            None
        }
    }
}
