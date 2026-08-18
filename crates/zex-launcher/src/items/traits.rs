//! Item behaviors, with blanket implementations over `Item`

use super::Item;
use std::path::PathBuf;

pub trait Identify {
    fn headline(&self) -> String;
    fn footnote(&self) -> Option<String>;
    fn icon(&self) -> Option<PathBuf>;
}

pub trait Launchable {
    fn launch(&self) -> anyhow::Result<()>;
}

impl Identify for Item {
    fn headline(&self) -> String {
        self.title()
    }

    fn footnote(&self) -> Option<String> {
        self.subtitle()
    }

    fn icon(&self) -> Option<PathBuf> {
        self.icon_path()
    }
}

impl Launchable for Item {
    fn launch(&self) -> anyhow::Result<()> {
        super::dispatch::dispatch(self)
    }
}
