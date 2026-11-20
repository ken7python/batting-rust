use macroquad::prelude::{Vec2, screen_height, screen_width, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayProfile {
    Desktop,
    Mobile,
    MobileWeb,
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

#[cfg(target_os = "ios")]
unsafe extern "C" {
    fn batting_ios_screen_scale() -> f32;
}

pub fn camera_layout_size() -> Vec2 {
    let size = vec2(screen_width(), screen_height());
    #[cfg(target_os = "ios")]
    {
        // Miniquad reports iOS framebuffer dimensions even when its DPI scale is 1.
        let ui_scale = unsafe { batting_ios_screen_scale() };
        logical_view_size(size, macroquad::prelude::screen_dpi_scale(), ui_scale)
    }
    #[cfg(not(target_os = "ios"))]
    size
}

#[cfg(any(test, target_os = "ios"))]
pub(crate) fn logical_view_size(render_size: Vec2, render_scale: f32, ui_scale: f32) -> Vec2 {
    render_size * (render_scale / ui_scale.max(1.0))
}

impl DisplayProfile {
    pub fn current() -> Self {
        #[cfg(target_arch = "wasm32")]
        // The Web loader reports whether the browser uses a coarse primary pointer.
        let mobile = unsafe { batting_uses_mobile_layout() != 0 };
        #[cfg(not(target_arch = "wasm32"))]
        let mobile = cfg!(any(target_os = "ios", target_os = "android"));
        if !mobile {
            Self::Desktop
        } else if cfg!(target_arch = "wasm32") {
            Self::MobileWeb
        } else {
            Self::Mobile
        }
    }

    pub fn mobile(self) -> bool {
        self != Self::Desktop
    }

    pub fn ui_scale(self, fit: f32) -> f32 {
        fit.min(if self.mobile() { 1.85 } else { 1.0 })
    }
}
