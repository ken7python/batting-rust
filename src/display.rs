#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayProfile {
    Desktop,
    Mobile,
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    fn batting_uses_mobile_layout() -> i32;
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn batting_display_crate_version() -> u32 {
    1
}

impl DisplayProfile {
    pub fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        // The Web loader reports whether the browser uses a coarse primary pointer.
        let mobile = unsafe { batting_uses_mobile_layout() != 0 };
        #[cfg(not(target_arch = "wasm32"))]
        let mobile = cfg!(any(target_os = "ios", target_os = "android"));
        if mobile { Self::Mobile } else { Self::Desktop }
    }

    pub fn mobile(self) -> bool {
        self == Self::Mobile
    }

    pub fn ui_scale(self, fit: f32) -> f32 {
        fit.min(if self.mobile() { 1.85 } else { 1.0 })
    }
}
