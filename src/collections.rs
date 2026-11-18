//! Wallpaper collections (fork feature): every wallpaper belongs to exactly one collection and
//! only the active collection rotates. Holidays switch on by themselves during their dates,
//! custom collections can follow a weekly or monthly schedule, and a manual pick wins until
//! the next automatic switch begins.

use serde::{Deserialize, Serialize};
use time::{Date, Month, OffsetDateTime, Weekday};

/// The default collection: every wallpaper not placed anywhere else.
pub const PHOTOS: &str = "photos";
/// Custom collections are stored as this prefix plus their name.
const CUSTOM: &str = "c:";

/// Built-in holiday collections, in calendar order, with their display names.
pub const HOLIDAYS: [(&str, &str); 8] = [
    ("new-year", "New Year's"),
    ("valentines", "Valentine's Day"),
    ("st-patricks", "St. Patrick's Day"),
    ("easter", "Easter"),
    ("july-4", "July 4th"),
    ("halloween", "Halloween"),
    ("thanksgiving", "Thanksgiving"),
    ("christmas", "Christmas"),
];

/// When a custom collection takes over on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Schedule {
    Saturdays,
    Sundays,
    Weekends,
    FirstWeek,
}

impl Schedule {
    pub const ALL: [Option<Self>; 5] = [
        None,
        Some(Self::Saturdays),
        Some(Self::Sundays),
        Some(Self::Weekends),
        Some(Self::FirstWeek),
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Saturdays => "Saturdays",
            Self::Sundays => "Sundays",
            Self::Weekends => "Weekends",
            Self::FirstWeek => "First week of the month",
        }
    }

    /// The first day of the run that includes `date`, if the schedule covers it.
    fn start(self, date: Date) -> Option<Date> {
        let weekday = date.weekday();
        match self {
            Self::Saturdays => (weekday == Weekday::Saturday).then_some(date),
            Self::Sundays => (weekday == Weekday::Sunday).then_some(date),
            Self::Weekends => match weekday {
                Weekday::Saturday => Some(date),
                Weekday::Sunday => date.previous_day(),
                _ => None,
            },
            Self::FirstWeek => (date.day() <= 7)
                .then(|| date.replace_day(1).ok())
                .flatten(),
        }
    }

    pub fn next(current: Option<Self>) -> Option<Self> {
        let index = Self::ALL
            .iter()
            .position(|item| *item == current)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

/// A manual choice of collection, and when it was made.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Pick {
    pub id: String,
    pub at: i64,
}

pub fn custom_id(name: &str) -> Option<String> {
    let name = clean_name(name)?;
    let lower = name.to_lowercase();
    if lower == "photos" {
        return Some(PHOTOS.into());
    }
    if let Some((id, _)) = HOLIDAYS
        .iter()
        .find(|(_, label)| label.to_lowercase() == lower)
    {
        return Some((*id).into());
    }
    Some(format!("{CUSTOM}{name}"))
}

/// A collection name from the phone: trimmed, single-spaced, printable, at most 32 characters.
pub fn clean_name(name: &str) -> Option<String> {
    let name = name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .take(32)
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

pub fn is_custom(id: &str) -> bool {
    id.starts_with(CUSTOM)
}

pub fn name(id: &str) -> String {
    if id == PHOTOS {
        return "Photos".into();
    }
    if let Some((_, label)) = HOLIDAYS.iter().find(|(key, _)| *key == id) {
        return (*label).into();
    }
    id.strip_prefix(CUSTOM).unwrap_or(id).into()
}

/// Inclusive date window of a holiday collection in `year`.
pub fn holiday_window(id: &str, year: i32) -> Option<(Date, Date)> {
    let day = |month: Month, day: u8| Date::from_calendar_date(year, month, day).ok();
    Some(match id {
        // Straddles the turn of the year; the January part belongs to this year's window.
        "new-year" => (day(Month::January, 1)?, day(Month::January, 2)?),
        "valentines" => (day(Month::February, 10)?, day(Month::February, 14)?),
        "st-patricks" => (day(Month::March, 14)?, day(Month::March, 17)?),
        "easter" => {
            let sunday = easter(year)?;
            (
                sunday.checked_sub(time::Duration::days(6))?,
                sunday.next_day()?,
            )
        }
        "july-4" => (day(Month::July, 1)?, day(Month::July, 5)?),
        "halloween" => (day(Month::October, 15)?, day(Month::October, 31)?),
        "thanksgiving" => {
            let thursday = thanksgiving(year)?;
            (
                thursday.checked_sub(time::Duration::days(3))?,
                thursday.checked_add(time::Duration::days(3))?,
            )
        }
        "christmas" => (day(Month::December, 1)?, day(Month::December, 26)?),
        _ => return None,
    })
}

/// A holiday starting within the next week (1 to 7 days ahead), with its first day.
pub fn upcoming_holiday(today: Date) -> Option<(&'static str, Date)> {
    HOLIDAYS.iter().find_map(|(id, _)| {
        [today.year(), today.year() + 1]
            .into_iter()
            .find_map(|year| {
                let (start, _) = holiday_window(id, year)?;
                // New Year's opens on New Year's Eve.
                let start = if *id == "new-year" {
                    start.previous_day()?
                } else {
                    start
                };
                let days = (start - today).whole_days();
                (1..=7).contains(&days).then_some((*id, start))
            })
    })
}

/// The holiday whose window contains `date`, with the first day of that window.
fn holiday_on(date: Date) -> Option<(&'static str, Date)> {
    // New Year's Eve opens next year's New Year window.
    if date.month() == Month::December && date.day() == 31 {
        return Some(("new-year", date));
    }
    HOLIDAYS.iter().find_map(|(id, _)| {
        let (start, end) = holiday_window(id, date.year())?;
        let start = if *id == "new-year" {
            start.previous_day()?
        } else {
            start
        };
        (start..=end).contains(&date).then_some((*id, start))
    })
}

pub fn month_name(month: Month) -> &'static str {
    match month {
        Month::January => "January",
        Month::February => "February",
        Month::March => "March",
        Month::April => "April",
        Month::May => "May",
        Month::June => "June",
        Month::July => "July",
        Month::August => "August",
        Month::September => "September",
        Month::October => "October",
        Month::November => "November",
        Month::December => "December",
    }
}

/// Fourth Thursday of November.
fn thanksgiving(year: i32) -> Option<Date> {
    let first = Date::from_calendar_date(year, Month::November, 1).ok()?;
    let offset = (Weekday::Thursday.number_days_from_monday() as i64
        - first.weekday().number_days_from_monday() as i64)
        .rem_euclid(7);
    first.checked_add(time::Duration::days(offset + 21))
}

/// Western Easter Sunday (anonymous Gregorian algorithm).
fn easter(year: i32) -> Option<Date> {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    Date::from_calendar_date(year, Month::try_from(month as u8).ok()?, day as u8).ok()
}

/// What `active` decided, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Active {
    pub id: String,
    /// "Christmas until Dec 26", "Manual", "Sundays", or empty for Photos by default.
    pub reason: Reason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    Default,
    Manual,
    Holiday(Date),
    Scheduled(Schedule),
}

/// The collection that should be showing now. Holidays beat schedules; a manual pick beats
/// both until the next automatic run starts after it was made. Empty collections are skipped.
pub fn active<'a>(
    now: OffsetDateTime,
    pick: Option<&Pick>,
    schedules: impl Iterator<Item = (&'a String, &'a Schedule)>,
    has_wallpapers: impl Fn(&str) -> bool,
) -> Active {
    let today = now.date();
    let midnight = |date: Date| {
        time::PrimitiveDateTime::new(date, time::Time::MIDNIGHT)
            .assume_offset(now.offset())
            .unix_timestamp()
    };
    let automatic = holiday_on(today)
        .filter(|(id, _)| has_wallpapers(id))
        .map(|(id, start)| {
            let end = holiday_window(
                id,
                if id == "new-year" && today.month() == Month::December {
                    today.year() + 1
                } else {
                    today.year()
                },
            )
            .map_or(today, |(_, end)| end);
            (id.to_string(), midnight(start), Reason::Holiday(end))
        })
        .or_else(|| {
            let mut scheduled: Vec<_> = schedules
                .filter(|(id, _)| has_wallpapers(id))
                .filter_map(|(id, schedule)| {
                    let start = schedule.start(today)?;
                    Some((id.clone(), midnight(start), Reason::Scheduled(*schedule)))
                })
                .collect();
            // Several schedules on one day: the most recently started run wins, then by name.
            scheduled.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            scheduled.into_iter().next()
        });
    if let Some(pick) = pick.filter(|pick| pick.id == PHOTOS || has_wallpapers(&pick.id))
        && automatic
            .as_ref()
            .is_none_or(|(_, start, _)| pick.at >= *start)
    {
        return Active {
            id: pick.id.clone(),
            reason: Reason::Manual,
        };
    }
    match automatic {
        Some((id, _, reason)) => Active { id, reason },
        None => Active {
            id: PHOTOS.into(),
            reason: Reason::Default,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i32, month: Month, day: u8, hour: u8) -> OffsetDateTime {
        time::PrimitiveDateTime::new(
            Date::from_calendar_date(year, month, day).unwrap(),
            time::Time::from_hms(hour, 0, 0).unwrap(),
        )
        .assume_offset(time::UtcOffset::from_hms(-7, 0, 0).unwrap())
    }

    #[test]
    fn movable_holidays_land_on_the_right_days() {
        assert_eq!(
            thanksgiving(2026),
            Date::from_calendar_date(2026, Month::November, 26).ok()
        );
        assert_eq!(
            thanksgiving(2027),
            Date::from_calendar_date(2027, Month::November, 25).ok()
        );
        assert_eq!(
            easter(2026),
            Date::from_calendar_date(2026, Month::April, 5).ok()
        );
        assert_eq!(
            easter(2027),
            Date::from_calendar_date(2027, Month::March, 28).ok()
        );
    }

    #[test]
    fn holidays_beat_schedules_and_a_manual_pick_lasts_until_the_next_switch() {
        let everything = |_: &str| true;
        let porsche = ("c:Vintage Porsche".to_string(), Schedule::Sundays);
        let schedules = || std::iter::once((&porsche.0, &porsche.1));
        // An ordinary Wednesday: Photos.
        let wednesday = at(2026, Month::November, 18, 12);
        assert_eq!(active(wednesday, None, schedules(), everything).id, PHOTOS);
        // Sunday: the Porsche schedule.
        let sunday = at(2026, Month::November, 15, 12);
        assert_eq!(
            active(sunday, None, schedules(), everything).id,
            "c:Vintage Porsche"
        );
        // Thanksgiving week beats the Sunday schedule.
        let thanksgiving_sunday = at(2026, Month::November, 29, 12);
        let decided = active(thanksgiving_sunday, None, schedules(), everything);
        assert_eq!(decided.id, "thanksgiving");
        // A pick made on a Wednesday holds until the Sunday run starts...
        let pick = Pick {
            id: "c:Art".into(),
            at: at(2026, Month::November, 11, 12).unix_timestamp(),
        };
        assert_eq!(
            active(
                at(2026, Month::November, 13, 12),
                Some(&pick),
                schedules(),
                everything
            )
            .id,
            "c:Art"
        );
        assert_eq!(
            active(sunday, Some(&pick), schedules(), everything).id,
            "c:Vintage Porsche"
        );
        // ...and comes back once the run is over.
        assert_eq!(
            active(
                at(2026, Month::November, 16, 12),
                Some(&pick),
                schedules(),
                everything
            )
            .id,
            "c:Art"
        );
        let _ = wednesday;
        // A pick made during Christmas wins over Christmas.
        let in_december = Pick {
            id: PHOTOS.into(),
            at: at(2026, Month::December, 10, 9).unix_timestamp(),
        };
        assert_eq!(
            active(
                at(2026, Month::December, 12, 9),
                Some(&in_december),
                schedules(),
                everything
            )
            .id,
            PHOTOS
        );
        // An empty holiday is skipped.
        let no_christmas = |id: &str| id != "christmas";
        assert_eq!(
            active(
                at(2026, Month::December, 12, 9),
                None,
                schedules(),
                no_christmas
            )
            .id,
            PHOTOS
        );
        // New Year's Eve and New Year's Day belong to New Year's.
        for when in [
            at(2026, Month::December, 31, 20),
            at(2027, Month::January, 2, 9),
        ] {
            assert_eq!(active(when, None, schedules(), everything).id, "new-year");
        }
    }

    #[test]
    fn names_from_the_phone_map_onto_collections() {
        assert_eq!(
            custom_id("  Vintage   Porsche "),
            Some("c:Vintage Porsche".into())
        );
        assert_eq!(custom_id("christmas"), Some("christmas".into()));
        assert_eq!(custom_id("Photos"), Some(PHOTOS.into()));
        assert_eq!(custom_id("   "), None);
        assert_eq!(name("c:Vintage Porsche"), "Vintage Porsche");
        assert_eq!(name("july-4"), "July 4th");
    }
}
