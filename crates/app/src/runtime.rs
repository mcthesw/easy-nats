#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeMode {
    Native,
    InteractiveDemo,
}

impl RuntimeMode {
    pub(crate) const fn supports_local_files(self) -> bool {
        matches!(self, Self::Native)
    }

    pub(crate) const fn shows_connection_in_tab_title(self) -> bool {
        matches!(self, Self::Native)
    }

    pub(crate) const fn preloads_demo_resources(self) -> bool {
        matches!(self, Self::InteractiveDemo)
    }
}
