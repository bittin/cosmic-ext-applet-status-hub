use cosmic::cosmic_config::{Config, ConfigGet, ConfigSet, CosmicConfigEntry, Error};
use serde::{Deserialize, Serialize};

use crate::applet::pins::CONFIG_VERSION;

pub const COLOUR_ICONS_KEY: &str = "colour-icons";
pub const HUB_SIDE_KEY: &str = "hub-side";

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub enum HubSide {
    Left,
    #[default]
    Right,
}

impl HubSide {
    pub const ALL: [Self; 2] = [Self::Left, Self::Right];

    pub fn at(index: usize) -> Option<Self> {
        Self::ALL.get(index).copied()
    }

    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    pub fn leads(self) -> bool {
        matches!(self, Self::Left)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Appearance {
    colour_icons: bool,
    hub_side: HubSide,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            colour_icons: true,
            hub_side: HubSide::default(),
        }
    }
}

impl Appearance {
    pub fn colour_icons(self) -> bool {
        self.colour_icons
    }

    pub fn set_colour_icons(&mut self, colour_icons: bool) {
        self.colour_icons = colour_icons;
    }

    pub fn hub_side(self) -> HubSide {
        self.hub_side
    }

    pub fn set_hub_side(&mut self, hub_side: HubSide) {
        self.hub_side = hub_side;
    }
}

impl CosmicConfigEntry for Appearance {
    const VERSION: u64 = CONFIG_VERSION;

    fn write_entry(&self, config: &Config) -> Result<(), Error> {
        config.set(COLOUR_ICONS_KEY, self.colour_icons)?;
        config.set(HUB_SIDE_KEY, self.hub_side)
    }

    fn get_entry(config: &Config) -> Result<Self, (Vec<Error>, Self)> {
        let mut appearance = Self::default();
        let mut errors = Vec::new();

        match config.get::<bool>(COLOUR_ICONS_KEY) {
            Ok(colour_icons) => appearance.colour_icons = colour_icons,
            Err(error) => errors.push(error),
        }

        match config.get::<HubSide>(HUB_SIDE_KEY) {
            Ok(hub_side) => appearance.hub_side = hub_side,
            Err(error) => errors.push(error),
        }

        if errors.is_empty() {
            Ok(appearance)
        } else {
            Err((errors, appearance))
        }
    }

    fn update_keys<T: AsRef<str>>(
        &mut self,
        config: &Config,
        changed_keys: &[T],
    ) -> (Vec<Error>, Vec<&'static str>) {
        let mut errors = Vec::new();
        let mut updated = Vec::new();

        for key in changed_keys {
            match key.as_ref() {
                COLOUR_ICONS_KEY => match config.get::<bool>(COLOUR_ICONS_KEY) {
                    Ok(colour_icons) if self.colour_icons != colour_icons => {
                        self.colour_icons = colour_icons;
                        updated.push(COLOUR_ICONS_KEY);
                    }
                    Ok(_) => {}
                    Err(error) => errors.push(error),
                },
                HUB_SIDE_KEY => match config.get::<HubSide>(HUB_SIDE_KEY) {
                    Ok(hub_side) if self.hub_side != hub_side => {
                        self.hub_side = hub_side;
                        updated.push(HUB_SIDE_KEY);
                    }
                    Ok(_) => {}
                    Err(error) => errors.push(error),
                },
                _ => {}
            }
        }

        (errors, updated)
    }
}

pub struct AppearanceStore {
    config: Option<Config>,
}

impl AppearanceStore {
    pub fn open(app_id: &str) -> Self {
        match Config::new(app_id, CONFIG_VERSION) {
            Ok(config) => Self {
                config: Some(config),
            },
            Err(error) => {
                tracing::warn!(%error, "appearance will not persist, the config is unavailable");
                Self { config: None }
            }
        }
    }

    pub fn load(&self) -> Appearance {
        let Some(config) = self.config.as_ref() else {
            return Appearance::default();
        };

        let mut appearance = Appearance::default();

        match config.get::<bool>(COLOUR_ICONS_KEY) {
            Ok(colour_icons) => appearance.colour_icons = colour_icons,
            Err(error) => tracing::debug!(%error, "no icon preference stored yet"),
        }

        match config.get::<HubSide>(HUB_SIDE_KEY) {
            Ok(hub_side) => appearance.hub_side = hub_side,
            Err(error) => tracing::debug!(%error, "no side preference stored yet"),
        }

        appearance
    }

    pub fn save(&self, appearance: Appearance) {
        let Some(config) = self.config.as_ref() else {
            return;
        };

        if let Err(error) = config.set(COLOUR_ICONS_KEY, appearance.colour_icons) {
            tracing::warn!(%error, "could not store the icon preference");
        }

        if let Err(error) = config.set(HUB_SIDE_KEY, appearance.hub_side) {
            tracing::warn!(%error, "could not store the side preference");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store(name: &str) -> (AppearanceStore, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "cosmic-status-hub-appearance-{}-{name}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        let config = Config::with_custom_path(name, CONFIG_VERSION, path.clone())
            .expect("a config under a temporary path");
        (
            AppearanceStore {
                config: Some(config),
            },
            path,
        )
    }

    #[test]
    fn icons_follow_the_theme_until_the_user_chooses_otherwise() {
        assert!(Appearance::default().colour_icons());
    }

    #[test]
    fn the_hub_button_sits_at_the_right_of_the_strip_until_the_user_moves_it() {
        assert_eq!(Appearance::default().hub_side(), HubSide::Right);
        assert!(!HubSide::Right.leads());
        assert!(HubSide::Left.leads());
    }

    #[test]
    fn every_side_survives_the_trip_through_its_dropdown_index() {
        for side in HubSide::ALL {
            assert_eq!(HubSide::at(side.index()), Some(side));
        }
        assert_eq!(HubSide::at(HubSide::ALL.len()), None);
    }

    #[test]
    fn the_chosen_preferences_survive_a_round_trip() {
        let (store, path) = test_store("appearance-round-trip");
        let mut appearance = Appearance::default();
        appearance.set_colour_icons(false);
        appearance.set_hub_side(HubSide::Left);

        store.save(appearance);

        assert_eq!(store.load(), appearance);
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn a_missing_preference_keeps_icons_adapted_to_the_theme() {
        let (store, path) = test_store("appearance-default");

        assert_eq!(store.load(), Appearance::default());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn one_stored_preference_does_not_drag_the_other_away_from_its_default() {
        let (store, path) = test_store("appearance-partial");
        let config = store.config.as_ref().unwrap();
        config.set(HUB_SIDE_KEY, HubSide::Left).unwrap();

        let appearance = store.load();

        assert_eq!(appearance.hub_side(), HubSide::Left);
        assert!(
            appearance.colour_icons(),
            "the icon preference was never written, so it keeps its default"
        );
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn a_changed_preference_is_reported_once() {
        let (store, path) = test_store("appearance-change");
        let config = store.config.as_ref().unwrap();
        config.set(COLOUR_ICONS_KEY, false).unwrap();
        let mut appearance = Appearance::default();

        let (errors, changed) = appearance.update_keys(config, &[COLOUR_ICONS_KEY]);

        assert!(errors.is_empty(), "nothing failed to read");
        assert_eq!(changed, [COLOUR_ICONS_KEY]);
        assert!(!appearance.colour_icons());

        let (_, unchanged) = appearance.update_keys(config, &[COLOUR_ICONS_KEY]);
        assert!(unchanged.is_empty(), "an identical reload is not a change");
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn a_changed_side_is_reported_without_touching_the_icon_preference() {
        let (store, path) = test_store("appearance-side-change");
        let config = store.config.as_ref().unwrap();
        config.set(HUB_SIDE_KEY, HubSide::Left).unwrap();
        let mut appearance = Appearance::default();

        let (errors, changed) = appearance.update_keys(config, &[COLOUR_ICONS_KEY, HUB_SIDE_KEY]);

        assert_eq!(
            errors.len(),
            1,
            "only the key that was never written fails to read"
        );
        assert_eq!(changed, [HUB_SIDE_KEY]);
        assert_eq!(appearance.hub_side(), HubSide::Left);
        assert!(appearance.colour_icons());
        let _ = std::fs::remove_dir_all(path);
    }
}
