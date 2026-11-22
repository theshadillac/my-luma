//! Automatic background cycling at a chosen interval, with no per-image exclusions.
use serde::{Deserialize, Deserializer, Serialize};
use std::time::Duration;

/// How often the wallpaper changes while rotation is on (fork option; upstream is fixed at
/// 15 minutes, which stays the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Interval {
    FiveMinutes,
    #[default]
    FifteenMinutes,
    Hour,
    FourHours,
    Day,
}

impl Interval {
    pub const ALL: [Self; 5] = [
        Self::FiveMinutes,
        Self::FifteenMinutes,
        Self::Hour,
        Self::FourHours,
        Self::Day,
    ];

    pub fn seconds(self) -> u64 {
        match self {
            Self::FiveMinutes => 5 * 60,
            Self::FifteenMinutes => 15 * 60,
            Self::Hour => 60 * 60,
            Self::FourHours => 4 * 60 * 60,
            Self::Day => 24 * 60 * 60,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::FiveMinutes => "cycle5m",
            Self::FifteenMinutes => "cycle15m",
            Self::Hour => "cycle1h",
            Self::FourHours => "cycle4h",
            Self::Day => "cycleDaily",
        }
    }

    /// The button's next step: Off, then each interval from shortest to longest, then Off.
    pub fn step(enabled: bool, interval: Self) -> (bool, Self) {
        if !enabled {
            return (true, Self::FiveMinutes);
        }
        match Self::ALL.iter().position(|step| *step == interval) {
            Some(index) if index + 1 < Self::ALL.len() => (true, Self::ALL[index + 1]),
            _ => (false, interval),
        }
    }
}

pub fn due(enabled: bool, interval: Interval, changed: u64, now: u64) -> bool {
    enabled && now.saturating_sub(changed) >= interval.seconds()
}

pub fn startup_due(enabled: bool, pending: bool, catalog_ready: bool, elapsed: Duration) -> bool {
    enabled && pending && (catalog_ready || elapsed >= Duration::from_secs(3))
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Bool(enabled) => enabled,
        // Preserve enabled preferences from the former duration-based UI.
        serde_json::Value::String(mode) => matches!(
            mode.as_str(),
            "on" | "launch" | "quarter" | "hourly" | "daily"
        ),
        _ => false,
    })
}

/// Advance in gallery order and wrap; unavailable images are absent from `ids`.
pub fn next(ids: &[String], current: &str) -> Option<String> {
    let start = ids
        .iter()
        .position(|id| id == current)
        .map_or(0, |index| index + 1);
    (0..ids.len())
        .map(|offset| &ids[(start + offset) % ids.len()])
        .find(|id| *id != current)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_available_background_participates_in_order_and_wraps() {
        let ids = vec!["slate".into(), "dunes".into(), "custom-id".into()];
        assert_eq!(next(&ids, "slate").as_deref(), Some("dunes"));
        assert_eq!(next(&ids, "dunes").as_deref(), Some("custom-id"));
        assert_eq!(next(&ids, "custom-id").as_deref(), Some("slate"));
        assert_eq!(next(&ids, "missing").as_deref(), Some("slate"));
        assert_eq!(next(&["slate".into()], "slate"), None);
        assert_eq!(next(&[], "slate"), None);
    }

    #[test]
    fn cycling_is_due_only_when_enabled_after_the_chosen_interval() {
        let fifteen = Interval::default();
        assert_eq!(fifteen.seconds(), 15 * 60);
        assert!(!due(false, fifteen, 0, u64::MAX));
        assert!(!due(true, fifteen, 100, 100 + 15 * 60 - 1));
        assert!(due(true, fifteen, 100, 100 + 15 * 60));
        assert!(!due(true, fifteen, 1_800_000_000, 1_577_836_800));
        assert!(!due(true, fifteen, u64::MAX, u64::MAX));
        assert!(due(true, Interval::FiveMinutes, 100, 100 + 300));
        assert!(!due(true, Interval::Hour, 100, 100 + 3599));
        assert!(!due(true, Interval::Day, 100, 100 + 4 * 3600));
        assert!(due(true, Interval::Day, 100, 100 + 24 * 3600));
    }

    #[test]
    fn the_button_steps_off_then_shortest_to_longest_then_off() {
        let mut setting = (false, Interval::default());
        let mut seen = vec![];
        for _ in 0..6 {
            setting = Interval::step(setting.0, setting.1);
            seen.push(setting.0.then_some(setting.1));
        }
        assert_eq!(
            seen,
            [
                Some(Interval::FiveMinutes),
                Some(Interval::FifteenMinutes),
                Some(Interval::Hour),
                Some(Interval::FourHours),
                Some(Interval::Day),
                None,
            ]
        );
        // Turning on from the old 15-minute default keeps its place in the steps.
        assert_eq!(
            Interval::step(true, Interval::FifteenMinutes),
            (true, Interval::Hour)
        );
    }

    #[test]
    fn enabled_rotation_advances_once_after_startup_catalog_or_timeout() {
        assert!(!startup_due(false, true, true, Duration::ZERO));
        assert!(!startup_due(true, false, true, Duration::ZERO));
        assert!(!startup_due(true, true, false, Duration::from_secs(2)));
        assert!(startup_due(true, true, true, Duration::ZERO));
        assert!(startup_due(true, true, false, Duration::from_secs(3)));
    }

    #[test]
    fn on_off_preferences_default_off_round_trip_and_load_legacy_modes() {
        assert!(!crate::model::Preferences::default().wallpaper_cycle);
        for enabled in [false, true] {
            let preferences: crate::model::Preferences =
                serde_json::from_value(serde_json::json!({"wallpaper_cycle": enabled})).unwrap();
            assert_eq!(preferences.wallpaper_cycle, enabled);
            assert_eq!(
                serde_json::to_value(preferences).unwrap()["wallpaper_cycle"],
                enabled
            );
        }
        for mode in ["on", "launch", "quarter", "hourly", "daily"] {
            let preferences: crate::model::Preferences =
                serde_json::from_value(serde_json::json!({"wallpaper_cycle": mode})).unwrap();
            assert!(preferences.wallpaper_cycle);
        }
        for value in [
            serde_json::json!("off"),
            serde_json::json!("weekly"),
            serde_json::json!(3),
            serde_json::Value::Null,
        ] {
            let preferences: crate::model::Preferences = serde_json::from_value(
                serde_json::json!({"wallpaper_cycle": value, "greeting_name": "Ana"}),
            )
            .unwrap();
            assert!(!preferences.wallpaper_cycle);
            assert_eq!(preferences.greeting_name, "Ana");
        }
    }
}
