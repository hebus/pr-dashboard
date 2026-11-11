//! Icon assets: the app's own SVGs (Lucide icons missing from
//! `gpui-component-assets`, plus the GitHub/GitLab brand marks), falling back
//! to the icon set bundled with gpui-component for its own widgets.

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$(
            (concat!("icons/", $name, ".svg"), include_bytes!(concat!("../assets/icons/", $name, ".svg")).as_slice()),
        )*]
    };
}

const OWN_ICONS: &[(&str, &[u8])] = icons![
    "git-pull-request",
    "git-merge",
    "git-branch",
    "clock",
    "refresh-cw",
    "circle-alert",
    "power",
    "x",
    "trash-2",
    "check-check",
    "download",
    "github",
    "gitlab",
];

pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = OWN_ICONS.iter().find(|(p, _)| *p == path) {
            return Ok(Some(Cow::Borrowed(*bytes)));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all = gpui_component_assets::Assets.list(path)?;
        all.extend(
            OWN_ICONS
                .iter()
                .filter(|(p, _)| p.starts_with(path))
                .map(|(p, _)| SharedString::from(*p)),
        );
        Ok(all)
    }
}
