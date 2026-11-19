#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Park {
    #[default]
    Central,
    Sunset,
}

impl Park {
    pub fn label(self) -> &'static str {
        match self {
            Self::Central => "中央球場",
            Self::Sunset => "夕景球場",
        }
    }

    pub fn fence(self) -> (f32, f32) {
        match self {
            Self::Central => (90.0, 3.0),
            Self::Sunset => (77.0, 2.2),
        }
    }
}
