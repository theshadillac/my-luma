use crate::i18n::Language;
use crate::model::{
    App, BACKGROUNDS, CATEGORIES, CUSTOM_TILE_PREFIX, CustomTile, INPUT_NAME_LIMIT, InputMode,
    Inventory, Preferences, clean_input_name,
};
use crate::text_rendering::TextRendering;
use crate::weather::{self, Unit};
use serde::Serialize;
use std::time::{Duration, Instant};

pub const HOME_SECTION: usize = 0;
pub const BACKGROUND_SECTION: usize = 1;
pub const WIDGETS_SECTION: usize = 2;
pub const SYSTEM_SECTION: usize = 3;
pub const DRAWER_SECTION: usize = 4;
pub const AMBIENT_SECTION: usize = 5;
pub const CONTROLS_SECTION: usize = 6;
/// Sidebar order, top to bottom. Section numbers stay stable; only their order is chosen here.
pub const SECTION_ORDER: [usize; 7] = [
    HOME_SECTION,
    DRAWER_SECTION,
    BACKGROUND_SECTION,
    AMBIENT_SECTION,
    WIDGETS_SECTION,
    CONTROLS_SECTION,
    SYSTEM_SECTION,
];
/// Sidebar labels, by section number.
pub const SECTION_KEYS: [&str; 7] = [
    "settingsHome",
    "background",
    "widgets",
    "system",
    "settingsDrawer",
    "settingsAmbient",
    "settingsControls",
];

/// The setting rows each list section shows, in order (System lists its own below the update card).
pub fn section_rows(section: usize, ui_sounds: bool) -> Vec<usize> {
    match section {
        HOME_SECTION => vec![
            0,
            1,
            GLASS_CLOCK_ROW,
            LANDING_ROW,
            TILE_LIFT_ROW,
            MORE_APPS_ROW,
        ],
        DRAWER_SECTION => vec![DRAWER_TOOLBAR_ROW, APP_SORTING_ROW],
        AMBIENT_SECTION => vec![
            AMBIENT_ROW,
            POWER_ON_ROW,
            PIXEL_SHIFT_ROW,
            DAY_NIGHT_ROW,
            WALLPAPER_TINT_ROW,
            WEATHER_EFFECTS_ROW,
            HOLIDAYS_ROW,
            HOLIDAY_REMINDERS_ROW,
        ],
        CONTROLS_SECTION => {
            let mut rows = vec![BACK_RECENTS_ROW, QUICK_PANEL_ROW, UI_SOUNDS_ROW];
            if ui_sounds {
                rows.push(UI_SOUND_LEVEL_ROW);
                rows.push(UI_SOUND_OUTPUT_ROW);
            }
            rows
        }
        _ => vec![],
    }
}

fn section_position(section: usize) -> usize {
    SECTION_ORDER
        .iter()
        .position(|&candidate| candidate == section)
        .unwrap_or(0)
}
/// General settings row: whether the Back button opens Recent apps.
pub const BACK_RECENTS_ROW: usize = 14;
pub const SETTINGS_BOTTOM_PADDING: f32 = 64.;
pub const SETTINGS_LIST_BOTTOM_PADDING: f32 = 48.;
/// General settings row: the wallpaper-first Home.
pub const LANDING_ROW: usize = 11;
/// General settings row: the glass clock on Home.
pub const GLASS_CLOCK_ROW: usize = 15;
/// General settings row: the ambient screen's delay.
pub const AMBIENT_ROW: usize = 16;
/// General settings row: the selected tile's lift.
pub const TILE_LIFT_ROW: usize = 17;
/// General settings row: the "More apps" hint under Home's last row.
pub const MORE_APPS_ROW: usize = 18;
/// General settings row: which quick settings buttons show.
pub const QUICK_PANEL_ROW: usize = 19;
/// General settings row: OLED pixel shift of Home's clock block.
pub const PIXEL_SHIFT_ROW: usize = 20;
/// General settings row: day and night wallpapers.
pub const DAY_NIGHT_ROW: usize = 21;
/// General settings row: rain and snow over the wallpaper.
pub const WEATHER_EFFECTS_ROW: usize = 22;
pub const DRAWER_TOOLBAR_ROW: usize = 23;
pub const UI_SOUNDS_ROW: usize = 24;
pub const APP_SORTING_ROW: usize = 25;
pub const UI_SOUND_LEVEL_ROW: usize = 26;
pub const UI_SOUND_OUTPUT_ROW: usize = 27;
pub const WALLPAPER_TINT_ROW: usize = 28;
pub const HOLIDAY_REMINDERS_ROW: usize = 29;
pub const HOLIDAYS_ROW: usize = 30;
pub const POWER_ON_ROW: usize = 31;

/// Falling weather drawn over Home's wallpaper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Precipitation {
    Rain,
    Snow,
}
const QUICK_TOGGLES: [Target; 3] = [
    Target::QuickToggle(crate::model::QuickKey::Ambient),
    Target::QuickToggle(crate::model::QuickKey::Inputs),
    Target::QuickToggle(crate::model::QuickKey::TvSettings),
];
/// Changing page (Home, the app drawer, Settings) eases the new page in, in seconds.
pub const PAGE_MOTION: f32 = 0.42;
/// How long the selected tile takes to grow and lift (and the previous one to settle).
pub const TILE_LIFT_TIME: f32 = 0.18;
/// The ambient screen's delay choices, in minutes; 0 is off.
pub const AMBIENT_CHOICES: [u8; 4] = [0, 2, 5, 10];
/// After this long, the ambient screen lets LG's own screen saver take over.
pub const AMBIENT_HANDOFF: Duration = Duration::from_secs(60 * 60);
/// The ambient screen's timeline, in seconds: Home fades out, then the wallpaper dims;
/// the clock stays put until the first drift a minute in, and each drift glides.
pub const AMBIENT_UI_FADE: f32 = 2.5;
pub const AMBIENT_DIM_START: f32 = 2.;
pub const AMBIENT_DIM_FADE: f32 = 2.5;
pub const AMBIENT_DRIFT_EVERY: f32 = 60.;
pub const AMBIENT_DRIFT_GLIDE: f32 = 4.;
/// Waking from the ambient screen, in seconds: the wallpaper brightens, the clock block glides
/// home, then the top bar and the favourites settle back in. Reduced motion just crossfades.
pub const AMBIENT_WAKE: f32 = 0.8;
pub const AMBIENT_WAKE_REDUCED: f32 = 0.4;

/// The ambient screen as it was when woken, so the wake can run its transition in reverse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Waking {
    pub since: Instant,
    /// How far Home had faded out and the wallpaper had dimmed, each 0 to 1.
    pub ui: f32,
    pub dim: f32,
    /// Where the clock block was on its tour.
    pub stops: (AmbientSpot, AmbientSpot, f32),
}

/// A stop on the ambient clock block's tour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AmbientSpot {
    /// Where Home shows it, for the first minute.
    Home,
    /// The middle of the screen.
    Center,
    LowerLeft,
    UpperLeft,
    UpperRight,
    LowerRight,
}
pub const AMBIENT_TOUR: [AmbientSpot; 5] = [
    AmbientSpot::LowerLeft,
    AmbientSpot::UpperLeft,
    AmbientSpot::UpperRight,
    AmbientSpot::LowerRight,
    AmbientSpot::Center,
];
/// How far the favourites and inputs sit lower in the wallpaper-first Home.
pub const LANDING_DROP: f32 = 280.;
const LANDING_SECONDS: f32 = 0.35;
pub const APPS_COLUMNS: usize = 6;
/// Automatic inventory retries before the error shows: with the 6.5 s Luna timeout about 50 s.
pub const INVENTORY_RETRIES: u32 = 5;
pub const INVENTORY_RETRY_DELAY: Duration = Duration::from_secs(2);
pub const HOME_STRIP_LEFT: f32 = 84.;
// Six favourites in full and the seventh cut off and fading out at the right (Apple TV rows):
// the strip ends after the sixth tile, so a focused tile never sits in the fade.
pub const HOME_STRIP_RIGHT: f32 = 1748.;
pub const HOME_TILE_WIDTH: f32 = 256.;
pub const HOME_TILE_PITCH: f32 = 281.6;
pub const HOME_FADE_LEFT: f32 = 8.;
pub const HOME_FADE_RIGHT: f32 = 1910.;
pub const BACKGROUND_FADE: f32 = 0.4;
pub const CYCLE_FADE: f32 = 1.6;

pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Page {
    Home,
    Apps,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Target {
    /// Nav(2) is the gear button (Luma settings); Home and the app drawer have no tabs.
    Nav(usize),
    TvSettings,
    /// The faint "More apps" hint under Home's last row: opens the app drawer.
    MoreApps,
    /// A wallpaper's options: mark it for night (or day) use.
    NightBackground,
    /// A quick settings button, its "All settings" link, and its on/off row in Settings.
    Quick(crate::model::QuickKey),
    QuickAll,
    QuickToggle(crate::model::QuickKey),
    ClockPosition,
    ClockStyle,
    /// The top bar's Inputs button (Settings → Widgets → Inputs) and its menu's rows.
    InputsButton,
    InputPick(String),
    InputsMenu,
    InputInMenu,
    InstallUpdate,
    CheckUpdates,
    Recent(String),
    RecentEmpty,
    RecentRetry,
    App(String),
    Input(String),
    Browse,
    Filter(usize),
    ClearFilter,
    Search,
    ClearSearch,
    Sidebar(usize),
    Setting(usize),
    Widget(usize),
    GreetingName,
    GreetingNameClear,
    GreetingToggle,
    DateToggle,
    DateFormat,
    AppNamesHome,
    AppNamesApps,
    ClockFormat,
    ClockTimeFormat,
    ClockSize,
    Background(usize),
    UploadWallpaper,
    /// The Background page's collection button.
    WallpaperCollection,
    /// A wallpaper's options: move it to another collection.
    MoveBackground,
    /// The Background page's Discover button (with an Unsplash key saved).
    DiscoverOpen,
    /// Removes an empty collection made on the TV (its empty gallery's button).
    RemoveCollection,
    /// An Unsplash suggestion on the Discover page.
    Suggestion(usize),
    /// Loads more Discover suggestions.
    MoreSuggestions,
    WallpaperCycle,
    WallpaperGradient,
    UseBackground,
    DeleteBackground,
    InputMode(String),
    InputRename,
    InputIcon,
    InputNameReset,
    Choice(usize),
    CloseDialog,
    Retry,
    DismissNotice,
    Option(usize),
    OpenApp,
    ConfirmDelete,
    CancelDelete,
    Key(usize),
    WeatherConfigure,
    WeatherToggle,
    WeatherLocation,
    WeatherTemperature,
    WeatherForecast,
    WeatherQuery,
    WeatherSearch,
    WeatherClear,
    WeatherResult(usize),
    Tile(String),
    TileTest(String),
    TileRemove(String),
    TileEditor,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
    pub fn center(self) -> (f32, f32) {
        (self.x + self.w / 2., self.y + self.h / 2.)
    }
    pub fn inset(self, n: f32) -> Self {
        Self::new(self.x + n, self.y + n, self.w - 2. * n, self.h - 2. * n)
    }
}

#[derive(Clone, Debug)]
pub struct Control {
    pub target: Target,
    pub rect: Rect,
    pub scrollable: bool,
}

#[derive(Clone, Debug)]
pub enum Overlay {
    Options {
        id: String,
        restore: Target,
    },
    Delete {
        id: String,
        restore: Target,
    },
    Recents {
        restore: Target,
    },
    Keyboard,
    SystemKeyboard,
    /// The Inputs list, opened from the quick settings panel.
    Inputs,
    /// The quick settings panel behind the gear.
    Quick,
    WallpaperUpload,
    TileEditor,
    BackgroundOptions {
        id: String,
        restore: Target,
    },
    BackgroundDelete {
        id: String,
        restore: Target,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Subpage {
    AppNames,
    Greeting,
    Date,
    DateFormat,
    Weather,
    Shortcuts,
    Inputs,
    Clock,
    ClockFormat,
    TimeFormat,
    ClockSize,
    QuickPanel,
    Input(String),
    InputIcons(String),
    WeatherLocation,
    Temperature,
    Language,
    TextRendering,
    /// Wallpaper collections: choose the active one, then each custom one's schedule.
    Collections,
    /// Choose the collection for one wallpaper.
    MoveCollection(String),
    /// Unsplash suggestions for the collection the gallery shows.
    Discover,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum TextField {
    Apps,
    Location,
    Name,
    InputName,
    CollectionName,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, PartialEq)]
pub enum Effect {
    Save,
    /// Read the app inventory again, keeping decoded icons of unchanged apps.
    Refresh,
    /// Refresh apps: read the inventory and every app icon again.
    Reload,
    Launch(String),
    Uninstall(String),
    DeleteWallpaper(String),
    /// Ask Unsplash for Discover suggestions.
    Suggest {
        query: String,
        page: u32,
    },
    /// Save a Discover suggestion into a collection.
    AddSuggestion {
        id: String,
        collection: String,
    },
    Recent {
        generation: u64,
        command: crate::recents::Command,
    },
    /// Fire-and-forget HTTP request for a custom tile, off the SDL thread.
    FireWebhook(CustomTile),
    Quit,
    Weather(weather::Command),
    UpdateDownload(String),
}

pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Space",
    "Delete", "Clear", "Done",
];

pub struct State {
    pub page: Page,
    pub focus: Target,
    pub inventory: Inventory,
    pub preferences: Preferences,
    pub query: String,
    pub filter: usize,
    pub section: usize,
    pub subpage: Option<Subpage>,
    pub scroll: f32,
    pub home_scroll: f32,
    pub overlay: Option<Overlay>,
    pub moving: Option<String>,
    pub loading: bool,
    /// Set by Refresh apps: the next inventory also reloads every app icon.
    pub reload_artwork: bool,
    /// System action consumed by the SDL loop, with disk work delegated to artwork's worker.
    pub reset_icon_cache: bool,
    pub launching: bool,
    pub uninstalling: Option<String>,
    pub notice: Option<(String, Instant)>,
    pub error: Option<String>,
    pub available_update: Option<String>,
    pub update_status: crate::update::Status,
    pub update_check_requested: bool,
    /// Inventory loads that failed in a row while no apps were shown yet.
    pub inventory_failures: u32,
    pub dirty: bool,
    pub pointer: bool,
    /// The control under the pointer right now; unlike focus, cleared when it leaves.
    pub hover: Option<Target>,
    pub pressed: Option<Target>,
    pub previous_background: Option<(String, Instant)>,
    /// Wallpaper-first Home: from, to (1 = lowered) and start of the current slide.
    landing: (f32, f32, Instant),
    /// The last remote, pointer or keyboard input, for the ambient screen.
    pub last_input: Instant,
    /// When the ambient screen started, while it shows.
    pub ambient: Option<Instant>,
    /// The ambient transition last drawn (fades, tour stop and glide), so each changed
    /// frame is redrawn and a still one is not.
    ambient_drawn: Option<(f32, f32, usize, f32)>,
    /// The wake-up transition back to Home, while it runs.
    pub waking: Option<Waking>,
    /// The page just entered and when, while it eases in.
    pub page_motion: Option<(Page, Instant)>,
    /// The favourite the app drawer was opened from, focused again when it closes.
    drawer_return: Option<Target>,
    /// Ambient started from the quick panel, whatever its idle timer.
    ambient_manual: bool,
    /// The ambient screen was set up as the TV turned off (ambient at power on).
    ambient_armed: bool,
    /// Until the first app list arrives (or the user presses something), Home's focus belongs on
    /// the first app, not the tab bar it starts on while nothing is loaded yet.
    settle_focus: bool,
    /// Set when the TV refused the audio device for UI sounds; shown in Settings.
    pub ui_sounds_unavailable: bool,
    /// What the TV did with the last UI sound, e.g. "pulseaudio · playing"; shown in Settings.
    pub ui_sounds_detail: Option<String>,
    /// Playback outputs the TV offers for UI sounds (filled once audio is first used).
    pub ui_sound_outputs: Vec<String>,
    /// The focus the tile lift last saw, the one before it, and when it changed.
    lift_focus: (Target, Option<Target>, Instant),
    pub background_fade: f32,
    pub wallpapers: Vec<crate::wallpapers::Wallpaper>,
    /// The wallpaper collection showing now (see `refresh_collection`).
    pub collection: crate::collections::Active,
    /// A holiday just started mixing in; the next wallpaper comes from it.
    pub mix_due: bool,
    /// The collection the Background gallery lists; None follows the active one.
    pub gallery_collection: Option<String>,
    /// An Unsplash access key is saved on the TV, so Discover is offered.
    pub unsplash_ready: bool,
    pub discover: Discover,
    pub deleting_wallpaper: Option<String>,
    pub wallpaper_upload: Option<Result<crate::wallpapers::Session, String>>,
    pub tile_editor: Option<Result<crate::companion::Session, String>>,
    /// Luma update (Settings → System), driven by the update worker.
    pub update: crate::update::InstallStatus,
    pub panel_started: Option<Instant>,
    pub weather: weather::Weather,
    pub keyboard_field: TextField,
    pub greeting_name_draft: String,
    pub input_name: String,
    /// The name typed for a new collection on the TV.
    pub collection_name: String,
    pub recents: crate::recents::Recents,
}

impl State {
    pub fn new(preferences: Preferences) -> Self {
        let landing = if preferences.landing_layout { 1. } else { 0. };
        Self {
            landing: (landing, landing, Instant::now()),
            last_input: Instant::now(),
            ambient: None,
            ambient_drawn: None,
            waking: None,
            page_motion: None,
            drawer_return: None,
            ambient_manual: false,
            ambient_armed: false,
            settle_focus: true,
            ui_sounds_unavailable: false,
            ui_sounds_detail: None,
            ui_sound_outputs: Vec::new(),
            lift_focus: (
                Target::Nav(0),
                None,
                Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    .unwrap_or_else(Instant::now),
            ),
            page: Page::Home,
            focus: Target::Nav(0),
            inventory: Inventory::default(),
            preferences,
            query: String::new(),
            filter: 0,
            section: 0,
            subpage: None,
            scroll: 0.,
            home_scroll: 0.,
            overlay: None,
            moving: None,
            loading: false,
            reload_artwork: false,
            reset_icon_cache: false,
            launching: false,
            uninstalling: None,
            notice: None,
            error: None,
            available_update: None,
            update_status: crate::update::Status::Unchecked,
            update_check_requested: false,
            inventory_failures: 0,
            dirty: true,
            pointer: false,
            hover: None,
            pressed: None,
            previous_background: None,
            background_fade: BACKGROUND_FADE,
            wallpapers: vec![],
            collection: crate::collections::Active {
                id: crate::collections::PHOTOS.into(),
                reason: crate::collections::Reason::Default,
                mix: None,
            },
            mix_due: false,
            gallery_collection: None,
            unsplash_ready: false,
            discover: Discover::default(),
            deleting_wallpaper: None,
            wallpaper_upload: None,
            tile_editor: None,
            update: crate::update::InstallStatus::default(),
            panel_started: None,
            weather: weather::Weather::default(),
            keyboard_field: TextField::Apps,
            greeting_name_draft: String::new(),
            collection_name: String::new(),
            input_name: String::new(),
            recents: crate::recents::Recents::default(),
        }
    }

    /// A week before a holiday whose collection is still empty, a longer notice on Home,
    /// once a year per holiday, shown only while someone is using the TV.
    pub fn remind_holiday(&mut self, now: time::OffsetDateTime) -> Vec<Effect> {
        if !self.preferences.holiday_reminders
            || self.preferences.holiday_mode == crate::collections::HolidayMode::Off
            || self.page != Page::Home
            || self.overlay.is_some()
            || self.ambient.is_some()
            || self.notice.is_some()
            || self.last_input.elapsed() > Duration::from_secs(20)
        {
            return vec![];
        }
        let Some((id, start)) = crate::collections::upcoming_holiday(now.date()) else {
            return vec![];
        };
        if self.collection_size(id) > 0
            || self.preferences.holiday_reminded.get(id) == Some(&start.year())
        {
            return vec![];
        }
        self.preferences
            .holiday_reminded
            .insert(id.into(), start.year());
        let text = format!(
            "{} starts {} {}. Add some wallpapers to its collection from your phone or Discover.",
            crate::collections::name(id),
            crate::collections::month_name(start.month()),
            start.day()
        );
        // Shown for about 12 seconds rather than the usual 5.
        self.notice = Some((text, Instant::now() + Duration::from_secs(7)));
        self.dirty = true;
        vec![Effect::Save]
    }

    pub fn notify(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
        self.dirty = true;
    }

    pub fn update_response(&mut self, result: Result<Option<String>, String>) {
        match result {
            Ok(latest) => {
                let announce = latest.is_some() && latest != self.available_update;
                self.available_update = latest;
                self.update_status = crate::update::Status::Checked;
                if announce {
                    self.notify(self.preferences.language.t("updateAvailable"));
                }
            }
            Err(error) => {
                eprintln!("Luma update check: {error}");
                self.update_status = crate::update::Status::Failed;
            }
        }
        if matches!(self.focus, Target::CheckUpdates | Target::InstallUpdate) {
            let target = if self.available_update.is_some() {
                Target::InstallUpdate
            } else {
                Target::CheckUpdates
            };
            if self.focus != target {
                self.focus = target;
                self.pressed = None;
            }
        }
        self.dirty = true;
    }

    pub fn update_busy(&self) -> bool {
        self.update_status == crate::update::Status::Checking
            || matches!(
                self.update,
                crate::update::InstallStatus::Downloading(_)
                    | crate::update::InstallStatus::Installing(_)
            )
    }

    pub fn install_response(&mut self, response: crate::update::Response) {
        use crate::update::{InstallStatus, Response};
        match response {
            Response::Progress(percent) => self.update = InstallStatus::Downloading(percent),
            Response::Installing(version) => {
                self.update = InstallStatus::Installing(version);
                self.notify(self.preferences.language.t("updateRestart"));
            }
            Response::Finished(result) => {
                self.available_update = None;
                match result {
                    Ok(()) => {
                        self.update = InstallStatus::Idle;
                        // This process may still run the old executable. Recheck rather
                        // than claim that its in-memory version was replaced.
                        self.update_status = crate::update::Status::Unchecked;
                    }
                    Err(error) => {
                        eprintln!("Luma update: {error}");
                        self.update = InstallStatus::Failed(error);
                        self.update_status = crate::update::Status::Failed;
                        self.notify(self.preferences.language.t("updateError"));
                    }
                }
                if self.focus == Target::InstallUpdate {
                    self.focus = Target::CheckUpdates;
                    self.pressed = None;
                }
            }
        }
        self.dirty = true;
    }

    pub fn tick(&mut self) {
        if self.focus != self.lift_focus.0 {
            let previous = std::mem::replace(&mut self.lift_focus.0, self.focus.clone());
            // Only app tiles lift; other focus moves leave nothing to animate.
            let tiles = matches!(previous, Target::App(_)) || matches!(self.focus, Target::App(_));
            let since = if tiles {
                Instant::now()
            } else {
                Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    .unwrap_or_else(Instant::now)
            };
            self.lift_focus = (self.focus.clone(), Some(previous), since);
            self.dirty = true;
        }
        if self.lift_moving() {
            self.dirty = true;
        }
        // The ambient screen's fades and glides change every frame without input: redraw
        // whenever the drawn transition differs, including its last frame.
        let ambient = self.ambient_elapsed().map(|elapsed| {
            let (ui, dim) = self.ambient_progress();
            let (_, _, glide) = self.ambient_stops();
            (ui, dim, (elapsed / AMBIENT_DRIFT_EVERY) as usize, glide)
        });
        if ambient != self.ambient_drawn {
            self.ambient_drawn = ambient;
            self.dirty = true;
        }
        // Falling rain or snow redraws every frame while it shows.
        if self.precipitation().is_some() && !self.preferences.reduced_motion {
            self.dirty = true;
        }
        if let Some((_, since)) = self.page_motion {
            self.dirty = true;
            if since.elapsed().as_secs_f32() >= PAGE_MOTION {
                self.page_motion = None;
            }
        }
        if let Some(progress) = self.wake_progress() {
            self.dirty = true;
            if progress >= 1. {
                // Home is back: the selected tile lifts in as the last beat.
                self.waking = None;
                self.lift_focus = (self.focus.clone(), None, Instant::now());
            }
        }
        let target = self.landing_target();
        if target != self.landing.1 {
            self.landing = (self.landing(), target, Instant::now());
            self.dirty = true;
        }
        if self.landing() != self.landing.1 {
            self.dirty = true;
        }
        let recents_open = self.recents_open();
        if recents_open || self.recents.visibility > 0. {
            self.dirty |= self.recents.animate(
                recents_open,
                self.preferences.reduced_motion,
                self.pointer,
                Instant::now(),
            );
        }
        if let Some(start) = self.panel_started {
            self.dirty = true;
            let duration = if self.overlay.is_some() { 120 } else { 150 };
            if self.preferences.reduced_motion || start.elapsed() >= Duration::from_millis(duration)
            {
                self.panel_started = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120))
            && !self.preferences.reduced_motion
        {
            self.dirty = true;
        }
        if let Some((_, start)) = &self.previous_background {
            self.dirty = true;
            if self.preferences.reduced_motion
                || start.elapsed().as_secs_f32() >= self.background_fade
            {
                self.previous_background = None;
            }
        }
        if self
            .notice
            .as_ref()
            .is_some_and(|(_, start)| start.elapsed() > Duration::from_secs(5))
        {
            self.notice = None;
            self.dirty = true;
        }
    }

    pub fn page_index(&self) -> usize {
        match self.page {
            Page::Home => 0,
            Page::Apps => 1,
            Page::Settings => 2,
        }
    }

    pub fn animating(&self) -> bool {
        self.transitioning() || (self.precipitation().is_some() && !self.preferences.reduced_motion)
    }

    /// A one-off transition is running. Unlike `animating`, this ignores the endless
    /// rain/snow effect, so wallpaper changes are not held back while it falls.
    pub fn transitioning(&self) -> bool {
        ((self.recents_open() || self.recents.visibility > 0.)
            && (self.recents.visibility != if self.recents_open() { 1. } else { 0. }
                || self.recents.cards_animating(self.pointer)))
            || self.previous_background.is_some()
            || self.landing() != self.landing.1
            || self.panel_started.is_some()
            || self.ambient_fading()
            || self.waking.is_some()
            || self.page_motion.is_some()
            || self.lift_moving()
            || (!self.preferences.reduced_motion
                && self
                    .notice
                    .as_ref()
                    .is_some_and(|(_, start)| start.elapsed() < Duration::from_millis(120)))
    }

    /// Records input. Returns true when it woke the ambient screen: that input only wakes.
    pub fn note_input(&mut self) -> bool {
        self.last_input = Instant::now();
        self.settle_focus = false;
        self.ambient_manual = false;
        if self.ambient.is_some() {
            let (ui, dim) = self.ambient_progress();
            let stops = self.ambient_stops();
            self.ambient = None;
            self.waking = Some(Waking {
                since: Instant::now(),
                ui,
                dim,
                stops,
            });
            self.dirty = true;
            return true;
        }
        false
    }

    /// A press during the wake-up skips to its end, so the press acts at once.
    pub fn finish_wake(&mut self) {
        if self.waking.take().is_some() {
            self.lift_focus = (self.focus.clone(), None, Instant::now());
            self.dirty = true;
        }
    }

    /// How far the wake-up has run, 0 to 1, while it runs.
    pub fn wake_progress(&self) -> Option<f32> {
        let length = if self.preferences.reduced_motion {
            AMBIENT_WAKE_REDUCED
        } else {
            AMBIENT_WAKE
        };
        self.waking
            .map(|waking| (waking.since.elapsed().as_secs_f32() / length).min(1.))
    }

    /// How long until the ambient screen may start, when it can start at all.
    pub fn ambient_due(&self) -> Option<Duration> {
        let minutes = self.preferences.ambient_minutes;
        if minutes == 0
            || self.ambient.is_some()
            || self.page != Page::Home
            || self.overlay.is_some()
            || self.moving.is_some()
            || self.recents_open()
            || self.loading
        {
            return None;
        }
        Some(Duration::from_secs(u64::from(minutes) * 60).saturating_sub(self.last_input.elapsed()))
    }

    /// Settings restored from a backup (already checked and sanitized). Collections, the
    /// Home layout and the theme follow on the next frame; the wallpapers stay as they are.
    pub fn restore_preferences(&mut self, preferences: Preferences) {
        self.preferences = preferences;
        self.dirty = true;
    }

    /// The TV came back on from standby with Luma in front: show Home or the ambient screen,
    /// as chosen, whatever was showing when it went off.
    pub fn powered_on(&mut self) {
        self.overlay = None;
        self.panel_started = None;
        self.moving = None;
        self.waking = None;
        if self.page != Page::Home {
            self.open(0);
        }
        self.focus = self.first_content();
        self.last_input = Instant::now();
        match self.preferences.power_on {
            crate::model::PowerOn::Home => {
                self.ambient = None;
                self.ambient_manual = false;
            }
            crate::model::PowerOn::Ambient => {
                // Already up since power off: keep it, so Home never shows first.
                if !(self.ambient_armed && self.ambient.is_some()) {
                    self.ambient = Some(Instant::now());
                }
                // Like the quick panel's Ambient button: shown even with the timer off.
                self.ambient_manual = true;
                self.pointer = false;
            }
        }
        self.ambient_armed = false;
        self.dirty = true;
    }

    /// The TV started turning off with Luma in front. With ambient at power on, switch to a
    /// fully faded-in ambient screen now, while the panel goes dark, so the TV wakes up
    /// straight into it. Returns whether it did.
    pub fn arm_power_on_ambient(&mut self) -> bool {
        if self.preferences.power_on != crate::model::PowerOn::Ambient {
            return false;
        }
        self.overlay = None;
        self.panel_started = None;
        self.moving = None;
        self.waking = None;
        if self.page != Page::Home {
            self.open(0);
        }
        self.focus = self.first_content();
        let faded = Duration::from_secs_f32(AMBIENT_DIM_START + AMBIENT_DIM_FADE);
        self.ambient = Some(
            Instant::now()
                .checked_sub(faded)
                .unwrap_or_else(Instant::now),
        );
        self.ambient_manual = true;
        self.ambient_armed = true;
        self.pointer = false;
        self.dirty = true;
        true
    }

    /// The TV came back to another app or input: drop the ambient screen set up at power off,
    /// so opening Luma later shows Home as usual.
    pub fn power_on_missed(&mut self) {
        if std::mem::take(&mut self.ambient_armed) {
            self.ambient = None;
            self.ambient_manual = false;
            self.dirty = true;
        }
    }

    /// Starts the ambient screen once Home has been idle long enough; only while Luma is seen.
    pub fn ambient_tick(&mut self, shown: bool) {
        if self.ambient.is_some()
            && ((self.preferences.ambient_minutes == 0 && !self.ambient_manual)
                || self.page != Page::Home)
        {
            self.ambient = None;
            self.dirty = true;
        }
        if self.waking.is_some() && self.page != Page::Home {
            self.waking = None;
            self.dirty = true;
        }
        if shown && self.ambient_due() == Some(Duration::ZERO) {
            self.ambient = Some(Instant::now());
            self.pointer = false;
            self.dirty = true;
        }
    }

    /// How far a tile is lifted, 0 to 1: the selected tile rises, the one just left settles.
    pub fn tile_lift(&self, target: &Target) -> f32 {
        if !self.preferences.tile_lift || self.pointer && self.focus != *target {
            return 0.;
        }
        let progress = if self.preferences.reduced_motion {
            1.
        } else {
            ease((self.lift_focus.2.elapsed().as_secs_f32() / TILE_LIFT_TIME).clamp(0., 1.))
        };
        if *target == self.focus {
            if self.lift_focus.0 == *target {
                progress
            } else {
                0.
            }
        } else if self.lift_focus.1.as_ref() == Some(target) {
            1. - progress
        } else {
            0.
        }
    }

    fn lift_moving(&self) -> bool {
        self.preferences.tile_lift
            && !self.preferences.reduced_motion
            && self.lift_focus.2.elapsed().as_secs_f32() < TILE_LIFT_TIME
    }

    /// Seconds since the ambient screen started, while it shows.
    pub fn ambient_elapsed(&self) -> Option<f32> {
        self.ambient.map(|since| since.elapsed().as_secs_f32())
    }

    /// The ambient screen's transition: how far Home has faded out and how far the
    /// wallpaper has dimmed, each 0 to 1. Reduced motion skips straight to the end.
    pub fn ambient_progress(&self) -> (f32, f32) {
        let Some(elapsed) = self.ambient_elapsed() else {
            return (0., 0.);
        };
        if self.preferences.reduced_motion {
            return (1., 1.);
        }
        let ui = (elapsed / AMBIENT_UI_FADE).clamp(0., 1.);
        let dim = ((elapsed - AMBIENT_DIM_START) / AMBIENT_DIM_FADE).clamp(0., 1.);
        (ease(ui), ease(dim))
    }

    /// The ambient clock block's tour: where it is coming from, where it is going and how far
    /// along the glide it is. It holds its Home position for the first minute, then visits
    /// each corner (lower left, upper left, upper right, lower right) and the centre, one
    /// stop a minute, round and round.
    pub fn ambient_stops(&self) -> (AmbientSpot, AmbientSpot, f32) {
        let Some(elapsed) = self.ambient_elapsed() else {
            return (AmbientSpot::Home, AmbientSpot::Home, 1.);
        };
        let step = (elapsed / AMBIENT_DRIFT_EVERY) as usize;
        if step == 0 {
            return (AmbientSpot::Home, AmbientSpot::Home, 1.);
        }
        let stop = |step: usize| {
            if step == 0 {
                AmbientSpot::Home
            } else {
                AMBIENT_TOUR[(step - 1) % AMBIENT_TOUR.len()]
            }
        };
        let glide = if self.preferences.reduced_motion {
            1.
        } else {
            ease(
                ((elapsed - step as f32 * AMBIENT_DRIFT_EVERY) / AMBIENT_DRIFT_GLIDE).clamp(0., 1.),
            )
        };
        (stop(step - 1), stop(step), glide)
    }

    fn ambient_fading(&self) -> bool {
        let Some(elapsed) = self.ambient_elapsed() else {
            return false;
        };
        !self.preferences.reduced_motion
            && (elapsed < AMBIENT_DIM_START + AMBIENT_DIM_FADE
                || (elapsed >= AMBIENT_DRIFT_EVERY
                    && elapsed % AMBIENT_DRIFT_EVERY < AMBIENT_DRIFT_GLIDE))
    }

    /// Whether Luma should keep LG's screen saver away: the ambient screen is on and has not
    /// yet run for its hand-off time.
    pub fn ambient_holds_screen_saver(&self) -> bool {
        self.preferences.ambient_minutes > 0
            && self
                .ambient
                .is_none_or(|since| since.elapsed() < AMBIENT_HANDOFF)
    }

    /// Home's lifecycle relaunch returns directly to Home, even from an overlay.
    pub fn return_home(&mut self) -> Vec<Effect> {
        // Commit keyboard drafts before open() clears their settings subpage.
        let pending = self.finish_text_input();
        self.recents.pending = None;
        self.recents.visibility = 0.;
        self.recents.closing_focus = None;
        self.wallpaper_upload = None;
        self.pointer = false;
        self.pressed = None;
        self.open(0);
        self.home_scroll = 0.;
        self.focus = self.first_content();
        self.settle_focus = !matches!(self.focus, Target::App(_));
        pending
    }

    /// The quick settings buttons switched on, in panel order.
    pub fn quick_keys(&self) -> Vec<crate::model::QuickKey> {
        use crate::model::QuickKey;
        QuickKey::ALL
            .into_iter()
            .filter(|key| self.preferences.quick_shown(*key))
            .filter(|key| *key != QuickKey::Inputs || !self.input_choices().is_empty())
            .collect()
    }

    /// The gear opens the quick settings panel.
    pub fn open_quick(&mut self) -> Vec<Effect> {
        self.overlay = Some(Overlay::Quick);
        self.focus = self
            .quick_keys()
            .first()
            .map_or(Target::QuickAll, |key| Target::Quick(*key));
        self.pressed = None;
        self.panel_started = (!self.preferences.reduced_motion).then(Instant::now);
        self.dirty = true;
        vec![]
    }

    fn close_quick(&mut self) {
        self.overlay = None;
        self.panel_started = None;
        self.focus = Target::Nav(2);
        self.pressed = None;
        self.dirty = true;
    }

    /// Starts easing in the page just entered (not with reduced motion).
    fn ease_in_page(&mut self) {
        self.page_motion = (!self.preferences.reduced_motion).then(|| (self.page, Instant::now()));
    }

    /// How far the page just entered has eased in, 0 to 1, while it does.
    pub fn page_motion_progress(&self) -> Option<(Page, f32)> {
        self.page_motion
            .map(|(page, since)| (page, (since.elapsed().as_secs_f32() / PAGE_MOTION).min(1.)))
    }

    /// Down past Home's last row (or the "More apps" hint): the app drawer, which is the Apps
    /// page, eases up over the wallpaper. Back or Up from its top row returns to the favourite.
    pub fn open_drawer(&mut self) {
        let from = matches!(self.focus, Target::App(_)).then(|| self.focus.clone());
        self.open(1);
        self.ease_in_page();
        self.focus = self.first_content();
        self.drawer_return = from;
    }

    /// Leaves the app drawer for Home, on the favourite it was opened from where possible.
    pub fn close_drawer(&mut self) -> Vec<Effect> {
        let back = self.drawer_return.take();
        let effects = self.return_home();
        self.ease_in_page();
        if let Some(target) = back
            && let Target::App(id) = &target
            && self.apps().iter().any(|app| &app.id == id)
        {
            self.focus = target;
            self.settle_focus = false;
        }
        effects
    }

    pub fn open(&mut self, index: usize) {
        self.weather.cancel_search();
        self.page = match index {
            1 => Page::Apps,
            2 => Page::Settings,
            _ => Page::Home,
        };
        if self.page != Page::Apps {
            self.drawer_return = None;
        }
        self.focus = Target::Nav(self.page_index());
        self.scroll = 0.;
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        self.query.clear();
        self.filter = 0;
        self.overlay = None;
        self.moving = None;
        self.subpage = None;
        self.panel_started = None;
        // Arriving on Home settles the wallpaper-first rows where Home's focus wants them,
        // even when the pointer clicked the tab: its freeze applies only to rows already on
        // Home, so it must not carry the raised layout over from another page.
        let target = self.landing_resting_target();
        if target != self.landing.1 {
            self.landing = (self.landing(), target, Instant::now());
        }
        self.dirty = true;
    }

    /// What the top bar's Inputs menu lists, by app ID: Live TV, then the inputs not hidden from
    /// it (whatever their Home card mode), plugged-in ones first (port order kept within each).
    pub fn input_choices(&self) -> Vec<String> {
        let live = self
            .inventory
            .apps
            .iter()
            .filter(|app| app.id == "com.webos.app.livetv")
            .map(|app| app.id.clone());
        let (plugged, rest): (Vec<_>, Vec<_>) = self
            .inventory
            .inputs
            .iter()
            .filter(|input| !self.preferences.inputs_menu_hidden.contains(&input.id))
            .partition(|input| input.connected == Some(true));
        live.chain(
            plugged
                .into_iter()
                .chain(rest)
                .map(|input| input.id.clone()),
        )
        .collect()
    }

    pub fn apps(&self) -> Vec<&App> {
        let ids = self.preferences.favorites(&self.inventory.apps);
        if self.page == Page::Home {
            return ids
                .iter()
                .filter(|id| !self.preferences.hidden.contains(id))
                .filter_map(|id| self.inventory.apps.iter().find(|app| &app.id == id))
                .collect();
        }
        let query = self.query.to_lowercase();
        let mut apps: Vec<_> = self
            .inventory
            .apps
            .iter()
            .filter(|app| {
                self.preferences.hidden.contains(&app.id) == (self.filter == 5)
                    && (self.filter == 0
                        || self.filter == 5
                        || app.category == CATEGORIES[self.filter - 1])
                    && app.title.to_lowercase().contains(&query)
            })
            .collect();
        // Each key is built once per app instead of on every comparison (upstream aa007ba).
        apps.sort_by_cached_key(|app| {
            let rank = ids
                .iter()
                .position(|id| id == &app.id)
                .unwrap_or(usize::MAX);
            let usage = match self.preferences.app_sorting {
                crate::model::AppSorting::Alphabetical => 0,
                crate::model::AppSorting::Usage => self
                    .preferences
                    .app_launch_counts
                    .get(&app.id)
                    .copied()
                    .unwrap_or(0),
            };
            (
                rank,
                std::cmp::Reverse(usage),
                app.title.to_lowercase(),
                app.id.as_str(),
            )
        });
        apps
    }

    /// Counts successful app launches only (never inputs, shortcuts or preview cards) for the
    /// "Most used" drawer order. Counting runs in both orders so switching later has history.
    pub fn record_app_launch(&mut self, id: &str) -> Vec<Effect> {
        if !self.inventory.apps.iter().any(|app| app.id == id)
            || crate::scroll_preview::is_placeholder(id)
            || self.preferences.custom_tile(id).is_some()
        {
            return vec![];
        }
        let count = self
            .preferences
            .app_launch_counts
            .entry(id.into())
            .or_default();
        *count = count.saturating_add(1);
        vec![Effect::Save]
    }

    /// With the wallpaper-first Home on, the favourites sit lower while focus is in the header
    /// of Home; moving down into them slides them up to their usual place.
    fn landing_target(&self) -> f32 {
        // Pointer focus must not slide its hit target away. Remote navigation resumes the
        // focus-led transition; overlays and move mode still use the normal layout.
        if self.preferences.landing_layout
            && self.page == Page::Home
            && self.overlay.is_none()
            && self.moving.is_none()
            && self.pointer
        {
            return self.landing.1;
        }
        self.landing_resting_target()
    }

    /// Where the rows rest for the current page and focus, ignoring the pointer freeze.
    fn landing_resting_target(&self) -> f32 {
        // With no input cards below the favourites there is nothing to make room for, so the
        // row stays lowered when focus moves into it.
        let lowered = self.preferences.landing_layout
            && self.page == Page::Home
            && matches!(self.overlay, None | Some(Overlay::Quick | Overlay::Inputs))
            && self.moving.is_none()
            && (matches!(
                self.focus,
                Target::Nav(_)
                    | Target::TvSettings
                    | Target::InputsButton
                    | Target::Quick(_)
                    | Target::QuickAll
                    | Target::InputPick(_)
            ) || !self.home_inputs_shown());
        if lowered { 1. } else { 0. }
    }

    /// Whether Home shows any input cards below the favourites.
    fn home_inputs_shown(&self) -> bool {
        self.inventory
            .inputs
            .iter()
            .any(|input| self.preferences.input_visible(input))
    }

    /// 0 in the usual layout, 1 fully lowered; eased while it slides.
    pub fn landing(&self) -> f32 {
        let (from, to, start) = self.landing;
        if self.preferences.reduced_motion {
            return to;
        }
        let t = (start.elapsed().as_secs_f32() / LANDING_SECONDS).min(1.);
        from + (to - from) * (1. - (1. - t).powi(3))
    }

    pub fn content_viewport(&self) -> Rect {
        Rect::new(64., 108.8, 1792., 1080. - 108.8)
    }

    fn scroll_bottom(&self) -> f32 {
        let viewport = self.content_viewport();
        viewport.y + viewport.h
            - if self.page == Page::Settings {
                if self.subpage == Some(Subpage::Language)
                    || (self.section == 1 && self.subpage.is_none())
                {
                    SETTINGS_LIST_BOTTOM_PADDING
                } else {
                    SETTINGS_BOTTOM_PADDING
                }
            } else {
                12.
            }
    }

    pub fn favorites_viewport(&self) -> Rect {
        let content = self.content_viewport();
        let left = if self.home_scroll > 0. {
            HOME_FADE_LEFT
        } else {
            0.
        };
        Rect::new(left, content.y, HOME_FADE_RIGHT - left, content.h)
    }

    pub fn apps_viewport(&self) -> Rect {
        let content = self.content_viewport();
        Rect::new(content.x, 244.8, content.w, content.y + content.h - 244.8)
    }

    pub fn backgrounds_viewport(&self) -> Rect {
        let content = self.content_viewport();
        Rect::new(620., 244.8, 1160., content.y + content.h - 244.8)
    }

    pub fn hit_test(&self, controls: &[Control], x: f32, y: f32) -> Option<Target> {
        controls
            .iter()
            .rev()
            .find(|control| {
                let viewport =
                    if self.page == Page::Home && matches!(control.target, Target::App(_)) {
                        self.favorites_viewport()
                    } else if matches!(control.target, Target::Background(_)) {
                        self.backgrounds_viewport()
                    } else if self.page == Page::Apps {
                        self.apps_viewport()
                    } else {
                        self.content_viewport()
                    };
                control.rect.contains(x, y) && (!control.scrollable || viewport.contains(x, y))
            })
            .map(|control| control.target.clone())
    }

    /// Returns whether this wheel event scrolled favorites, so a pending card press is cancelled.
    pub fn wheel_scroll(
        &mut self,
        controls: &[Control],
        position: Option<(f32, f32)>,
        x: i32,
        y: i32,
    ) -> bool {
        let backgrounds =
            self.page == Page::Settings && self.section == 1 && self.subpage.is_none();
        if backgrounds && position.is_some_and(|(x, y)| !self.backgrounds_viewport().contains(x, y))
        {
            return false;
        }
        let over_favorites = self.page == Page::Home
            && position.is_some_and(|(x, y)| {
                self.favorites_viewport().contains(x, y)
                    && controls.iter().any(|control| {
                        matches!(control.target, Target::App(_)) && control.rect.contains(x, y)
                    })
            });
        let favorites = self.page == Page::Home && (x != 0 || over_favorites);
        if favorites {
            self.scroll_favorites((if x != 0 { x } else { -y }) as f32 * HOME_TILE_PITCH);
        } else {
            let bottom = self.scroll_bottom();
            let max = controls
                .iter()
                .filter(|c| {
                    c.scrollable && (!backgrounds || matches!(c.target, Target::Background(_)))
                })
                .map(|c| c.rect.y + c.rect.h + self.scroll - bottom)
                .fold(0., f32::max);
            self.scroll = (self.scroll - y as f32 * 90.).clamp(0., max.max(0.));
        }
        self.pointer = true;
        self.dirty = true;
        favorites
    }

    pub fn scroll_favorites(&mut self, delta: f32) {
        let count = self
            .preferences
            .favorites(&self.inventory.apps)
            .iter()
            .filter(|id| {
                !self.preferences.hidden.contains(id)
                    && self.inventory.apps.iter().any(|app| &app.id == *id)
            })
            .count();
        let content = if count == 0 {
            0.
        } else {
            (count - 1) as f32 * HOME_TILE_PITCH + HOME_TILE_WIDTH
        };
        let max = (content - (HOME_STRIP_RIGHT - HOME_STRIP_LEFT)).max(0.);
        let next = (self.home_scroll + delta).clamp(0., max);
        if next != self.home_scroll {
            self.home_scroll = next;
            self.dirty = true;
        }
    }

    pub fn set_inventory(&mut self, next: Inventory, warning: Option<String>) {
        self.inventory.apps = next.apps;
        self.sync_custom_tiles();
        if let Some(warning) = warning {
            self.notify(warning);
        } else {
            self.inventory.inputs = next.inputs;
        }
        self.loading = false;
        self.error = None;
        self.inventory_failures = 0;
        self.dirty = true;
        self.reconcile_focus();
        self.settle_home_focus();
    }

    /// Moves a cold start's tab-bar focus onto the first app once apps exist. Pressing anything
    /// first, or being anywhere but Home without an overlay, leaves focus alone.
    fn settle_home_focus(&mut self) {
        if !self.settle_focus || self.page != Page::Home || self.overlay.is_some() {
            return;
        }
        if !matches!(self.focus, Target::Nav(0) | Target::Browse | Target::Retry) {
            self.settle_focus = false;
            return;
        }
        self.home_scroll = 0.;
        if let first @ Target::App(_) = self.first_content() {
            self.focus = first;
            self.settle_focus = false;
        }
    }

    /// Replace only input status; leave installed apps, artwork and saved preferences alone.
    pub fn set_inputs(&mut self, inputs: Vec<crate::model::Input>) {
        if self.inventory.inputs == inputs {
            return;
        }
        self.inventory.inputs = inputs;
        self.dirty = true;
        self.reconcile_focus();
        let missing = |id: &str| !self.inventory.inputs.iter().any(|input| input.id == id);
        if self.page == Page::Settings
            && (matches!(&self.subpage, Some(Subpage::Input(id) | Subpage::InputIcons(id)) if missing(id))
                || matches!(&self.focus, Target::InputMode(id) if missing(id)))
        {
            self.subpage = Some(Subpage::Inputs);
            self.scroll = 0.;
            self.focus = self
                .inventory
                .inputs
                .first()
                .map_or(Target::Sidebar(WIDGETS_SECTION), |input| {
                    Target::InputMode(input.id.clone())
                });
        }
    }

    /// A failed inventory load. Right after a cold boot, or when the TV is short of memory, the
    /// Homebrew service that reads the inventory can take longer than one request allows. While
    /// no apps are shown yet, the screen keeps loading and the caller retries after the returned
    /// delay; the error and its Retry button only appear once those retries are used up.
    /// Custom tiles come from preferences, not the TV, so they don't count as loaded apps.
    pub fn inventory_failed(&mut self, error: String) -> Option<Duration> {
        self.dirty = true;
        let no_tv_apps = self
            .inventory
            .apps
            .iter()
            .all(|app| app.id.starts_with(CUSTOM_TILE_PREFIX));
        if no_tv_apps && self.inventory_failures < INVENTORY_RETRIES {
            self.inventory_failures += 1;
            return Some(INVENTORY_RETRY_DELAY);
        }
        self.inventory_failures = 0;
        self.loading = false;
        self.error = Some(error);
        None
    }

    /// Custom webhook tiles are never read from the TV. They are merged into
    /// the inventory on every refresh and edit, so favoriting, hiding and
    /// search treat them like any other app.
    pub fn sync_custom_tiles(&mut self) {
        self.inventory
            .apps
            .retain(|app| !app.id.starts_with(CUSTOM_TILE_PREFIX));
        self.inventory.apps.extend(self.preferences.custom_apps());
        self.dirty = true;
    }

    pub fn reconcile_focus(&mut self) {
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if let Target::App(id) = &self.focus
            && !self.apps().iter().any(|app| &app.id == id)
        {
            self.focus = self.first_content();
        }
        if let Target::Input(id) = &self.focus
            && !self
                .inventory
                .inputs
                .iter()
                .any(|input| &input.id == id && self.preferences.input_visible(input))
        {
            self.focus = self.first_content();
        }
    }

    pub fn first_content(&self) -> Target {
        match self.page {
            Page::Home => self
                .apps()
                .get(((self.home_scroll - 0.01).max(0.) / HOME_TILE_PITCH).ceil() as usize)
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or(if self.error.is_some() {
                    Target::Retry
                } else {
                    Target::Browse
                }),
            Page::Apps => self
                .apps()
                .first()
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or_else(|| self.drawer_top().unwrap_or(Target::Filter(1))),
            Page::Settings => Target::Sidebar(0),
        }
    }

    pub fn focus(&mut self, target: Target, controls: &[Control]) {
        self.pointer = false;
        self.focus = target;
        self.reveal_focus(controls);
        if let Target::Sidebar(section) = self.focus {
            if self.subpage == Some(Subpage::WeatherLocation) {
                self.weather.cancel_search();
            }
            self.section = section;
            self.subpage = None;
        }
        self.dirty = true;
    }

    /// Remote focus is revealed after layout changes, including hide/reorder.
    /// Pointer paths deliberately do not call this method.
    pub fn reveal_focus(&mut self, controls: &[Control]) -> bool {
        let previous = self.scroll;
        let previous_home = self.home_scroll;
        if let Some(control) = controls.iter().find(|c| c.target == self.focus) {
            if self.page == Page::Home && matches!(self.focus, Target::App(_)) {
                if control.rect.x < HOME_STRIP_LEFT {
                    self.home_scroll += control.rect.x - HOME_STRIP_LEFT;
                } else if control.rect.x + control.rect.w > HOME_STRIP_RIGHT {
                    self.home_scroll += control.rect.x + control.rect.w - HOME_STRIP_RIGHT;
                }
            }
            if matches!(
                self.focus,
                Target::Nav(_)
                    | Target::Filter(_)
                    | Target::ClearFilter
                    | Target::Search
                    | Target::ClearSearch
                    | Target::Sidebar(_)
            ) {
                self.scroll = 0.;
            } else if control.scrollable {
                let top = if matches!(self.focus, Target::Background(_)) {
                    self.backgrounds_viewport().y + 8.
                } else if self.page == Page::Apps {
                    self.apps_viewport().y + 8.
                } else {
                    120.8
                };
                // Where the control ends up once the wallpaper-first Home finished sliding.
                let y = if self.page == Page::Home {
                    control.rect.y - (self.landing() - self.landing_target()) * LANDING_DROP
                } else {
                    control.rect.y
                };
                if y < top {
                    self.scroll = (self.scroll + y - top).max(0.);
                }
                let bottom = self.scroll_bottom();
                if y + control.rect.h > bottom {
                    self.scroll += y + control.rect.h - bottom;
                }
            }
        }
        if self.page == Page::Home {
            self.scroll_favorites(0.);
        }
        if self.scroll != previous || self.home_scroll != previous_home {
            self.dirty = true;
        }
        self.scroll != previous || self.home_scroll != previous_home
    }

    /// Hover and pointer-down must not move the control before pointer-up.
    pub fn pointer_focus(&mut self, target: Target) {
        // Stop an in-flight landing slide at the geometry used for this pointer event,
        // before changing focus. Keep that geometry for hover, down and release.
        let landing = self.landing();
        self.landing = (landing, landing, Instant::now());
        if !self.pointer {
            self.dirty = true;
        }
        self.pointer = true;
        if self.focus == target {
            return;
        }
        // Hovering a sidebar section only highlights it; a click opens it (see activate).
        self.focus = target;
        self.dirty = true;
    }

    /// Whether the drawer shows its category filters and search box.
    pub fn drawer_toolbar(&self) -> bool {
        self.preferences.drawer_toolbar
    }

    /// The drawer's top-row control above the first apps, if it shows one. Without the
    /// toolbar only the Hidden pill remains, so hidden apps can still be found and restored.
    pub fn drawer_top(&self) -> Option<Target> {
        if self.drawer_toolbar() {
            Some(Target::Filter(1))
        } else if self.hidden_count() > 0 || self.filter == 5 {
            Some(Target::Filter(5))
        } else {
            None
        }
    }

    pub fn hidden_count(&self) -> usize {
        self.inventory
            .apps
            .iter()
            .filter(|app| self.preferences.hidden.contains(&app.id))
            .count()
    }

    pub fn background_count(&self) -> usize {
        self.background_ids().count()
    }

    /// Whether it is night where the TV is: between today's sunset and sunrise from the
    /// weather service, or 7 pm to 7 am without it.
    /// The time-of-day wash over the wallpaper as colour and opacity: rose around sunrise,
    /// nothing through the day, amber in the golden hour, violet at dusk and deep blue at
    /// night. Follows real sunrise and sunset for the weather location (7:00 and 19:00
    /// without one) and blends smoothly between those moments.
    pub fn wallpaper_tint(&self, now: time::OffsetDateTime) -> Option<(u32, f32)> {
        let strength = self.preferences.wallpaper_tint.alpha();
        if strength == 0. {
            return None;
        }
        let minutes = |unix: i64| {
            time::OffsetDateTime::from_unix_timestamp(unix)
                .ok()
                .map(|at| at.to_offset(now.offset()))
                .map(|at| at.hour() as f32 * 60. + at.minute() as f32)
        };
        let (rise, set) = self
            .sun_times(now)
            .and_then(|(rise, set)| Some((minutes(rise)?, minutes(set)?)))
            .unwrap_or((7. * 60., 19. * 60.));
        const NIGHT: u32 = 0x1b2a6b;
        const ROSE: u32 = 0xff8a8a;
        const AMBER: u32 = 0xffa646;
        const VIOLET: u32 = 0x7b5cc8;
        // (minute of day, colour, share of the full strength), in order through the day.
        let keys = [
            (rise - 50., NIGHT, 1.),
            (rise + 5., ROSE, 0.8),
            (rise + 80., ROSE, 0.),
            (set - 100., AMBER, 0.),
            (set - 25., AMBER, 0.9),
            (set + 20., VIOLET, 0.75),
            (set + 70., NIGHT, 1.),
        ];
        let now = now.hour() as f32 * 60. + now.minute() as f32 + now.second() as f32 / 60.;
        let (color, share) = match keys
            .windows(2)
            .find(|pair| (pair[0].0..pair[1].0).contains(&now))
        {
            Some(pair) => {
                let (from, to) = (pair[0], pair[1]);
                let t = (now - from.0) / (to.0 - from.0);
                // Fade colour across a change of hue; keep it while only the strength moves.
                (mix(from.1, to.1, t), from.2 + (to.2 - from.2) * t)
            }
            None => (NIGHT, 1.),
        };
        (share > 0.005).then_some((color, strength * share))
    }

    /// Today's sunrise and sunset for the weather location: the weather service's times
    /// when it sent them, otherwise calculated from the place's coordinates.
    pub fn sun_times(&self, now: time::OffsetDateTime) -> Option<(i64, i64)> {
        let place = self.preferences.weather_location.as_ref()?;
        self.weather
            .reading(place)
            .and_then(|reading| reading.sun)
            .or_else(|| {
                // The local calendar day, expressed as the UTC date the almanac expects.
                let date = now.date();
                crate::weather::solar_times(place.latitude, place.longitude, date)
            })
    }

    pub fn is_night(&self, now: time::OffsetDateTime) -> bool {
        let minutes = |at: time::OffsetDateTime| at.hour() as i32 * 60 + at.minute() as i32;
        let local = |unix: i64| {
            time::OffsetDateTime::from_unix_timestamp(unix)
                .ok()
                .map(|at| minutes(at.to_offset(now.offset())))
        };
        let sun = self
            .sun_times(now)
            .and_then(|(rise, set)| Some((local(rise)?, local(set)?)));
        let (rise, set) = sun.unwrap_or((7 * 60, 19 * 60));
        let now = minutes(now);
        now < rise || now >= set
    }

    /// Rain or snow to draw over Home's wallpaper: the option is on and the current weather
    /// (WMO code) is drizzle, rain, showers, a storm, or snow.
    pub fn precipitation(&self) -> Option<Precipitation> {
        if !self.preferences.weather_effects
            || self.page != Page::Home
            || !matches!(self.overlay, None | Some(Overlay::Quick))
        {
            return None;
        }
        let place = self.preferences.weather_location.as_ref()?;
        match self.weather.reading(place)?.code {
            51..=67 | 80..=82 | 95..=99 => Some(Precipitation::Rain),
            71..=77 | 85..=86 => Some(Precipitation::Snow),
            _ => None,
        }
    }

    fn night_wallpaper(&self, id: &str) -> bool {
        self.preferences
            .night_wallpapers
            .iter()
            .any(|item| item == id)
    }

    /// Wallpapers for this time of day, or all of them when day and night is off or a set
    /// is empty.
    fn wallpapers_for(&self, night: Option<bool>) -> Vec<String> {
        let usable = |id: &&str| self.deleting_wallpaper.as_deref() != Some(*id);
        // Mix in: alternate, the holiday after anything else and back again.
        let source = match &self.collection.mix {
            Some((holiday, _))
                if self.wallpaper_collection(&self.preferences.background) != holiday
                    && self.collection_size(holiday) > 0 =>
            {
                holiday.as_str()
            }
            _ => self.collection.id.as_str(),
        };
        let mut all: Vec<String> = self
            .collection_ids(source)
            .filter(usable)
            .map(str::to_owned)
            .collect();
        if all.is_empty() {
            all = self
                .all_background_ids()
                .filter(usable)
                .map(str::to_owned)
                .collect();
        }
        let Some(night) = night.filter(|_| self.preferences.day_night) else {
            return all;
        };
        let matching: Vec<String> = all
            .iter()
            .filter(|id| self.night_wallpaper(id) == night)
            .cloned()
            .collect();
        if matching.is_empty() { all } else { matching }
    }

    /// Day and night is on and the wallpaper belongs to the other half of the day.
    pub fn day_night_due(&self, now: time::OffsetDateTime) -> bool {
        let night = self.is_night(now);
        self.preferences.day_night
            && self.night_wallpaper(&self.preferences.background) != night
            && self
                .wallpapers_for(Some(night))
                .iter()
                .any(|id| self.night_wallpaper(id) == night)
    }

    /// Advances rotation, or swaps to the right half of day and night, with a crossfade.
    pub fn cycle_wallpaper_at(&mut self, now: time::OffsetDateTime) -> bool {
        let night = self.is_night(now);
        let ids = self.wallpapers_for(Some(night));
        self.cycle_through(ids, now.unix_timestamp().max(0) as u64)
    }

    #[cfg(test)]
    pub fn cycle_wallpaper(&mut self, now: u64) -> bool {
        let ids = self.wallpapers_for(None);
        self.cycle_through(ids, now)
    }

    fn cycle_through(&mut self, ids: Vec<String>, now: u64) -> bool {
        let next = crate::wallpaper_cycle::next(&ids, &self.preferences.background);
        self.preferences.wallpaper_changed = now;
        self.mix_due = false;
        let Some(id) = next else {
            return false;
        };
        self.preferences.splash_background = Some(self.preferences.splash_background().to_owned());
        if self.preferences.reduced_motion {
            self.previous_background = None;
        } else {
            self.previous_background = Some((self.preferences.background.clone(), Instant::now()));
            self.background_fade = CYCLE_FADE;
        }
        self.preferences.background = id;
        self.dirty = true;
        true
    }

    pub fn background_id(&self, index: usize) -> Option<&str> {
        self.background_ids().nth(index)
    }

    /// The collection a wallpaper belongs to: a choice made on the TV, else the one chosen
    /// when it was uploaded, else Photos (always for the built-in wallpapers).
    pub fn wallpaper_collection(&self, id: &str) -> &str {
        if let Some(collection) = self.preferences.wallpaper_collections.get(id) {
            return collection;
        }
        self.wallpapers
            .iter()
            .find(|item| item.id == id)
            .and_then(|item| item.collection.as_deref())
            .unwrap_or(crate::collections::PHOTOS)
    }

    /// The collection the Background gallery shows.
    pub fn gallery_collection(&self) -> &str {
        self.gallery_collection
            .as_deref()
            .unwrap_or(&self.collection.id)
    }

    fn collection_ids<'a>(&'a self, collection: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.all_background_ids()
            .filter(move |id| self.wallpaper_collection(id) == collection)
    }

    pub fn collection_size(&self, collection: &str) -> usize {
        self.collection_ids(collection).count()
    }

    /// Every collection to list: Photos, custom ones (known from wallpapers and schedules),
    /// then the holidays in calendar order, each with its number of wallpapers.
    pub fn collections(&self) -> Vec<(String, usize)> {
        let mut custom: Vec<String> = self
            .all_background_ids()
            .map(|id| self.wallpaper_collection(id))
            .chain(
                self.preferences
                    .collection_schedules
                    .keys()
                    .map(String::as_str),
            )
            .chain(
                self.preferences
                    .created_collections
                    .iter()
                    .map(String::as_str),
            )
            .filter(|id| crate::collections::is_custom(id))
            .map(str::to_owned)
            .collect();
        custom.sort_by_key(|id| id.to_lowercase());
        custom.dedup();
        std::iter::once(crate::collections::PHOTOS.to_string())
            .chain(custom)
            .chain(
                crate::collections::HOLIDAYS
                    .iter()
                    .filter(|_| {
                        self.preferences.holiday_mode != crate::collections::HolidayMode::Off
                    })
                    .map(|(id, _)| (*id).to_string()),
            )
            .map(|id| {
                let size = self.collection_size(&id);
                (id, size)
            })
            .collect()
    }

    /// Discover results arrived (or failed) for a page of the current search.
    pub fn suggestions(
        &mut self,
        page: u32,
        result: Result<Vec<crate::wallpapers::Suggestion>, String>,
    ) {
        if page != self.discover.page {
            return;
        }
        self.discover.loading = false;
        match result {
            Ok(found) => {
                self.discover.exhausted = found.len() < 24;
                let start = self.discover.photos.len();
                for suggestion in found {
                    if !self
                        .discover
                        .photos
                        .iter()
                        .any(|item| item.suggestion.photo.id == suggestion.photo.id)
                    {
                        self.discover.photos.push(SuggestionItem {
                            suggestion,
                            status: SuggestionStatus::Ready,
                        });
                    }
                }
                self.discover.error = None;
                if self.focus == Target::MoreSuggestions
                    && self.subpage == Some(Subpage::Discover)
                    && start < self.discover.photos.len()
                {
                    self.focus = Target::Suggestion(start);
                }
            }
            Err(error) => self.discover.error = Some(error),
        }
        self.dirty = true;
    }

    /// The collection (display name) already holding this Unsplash photo, if any.
    pub fn suggestion_owner(&self, photo: &str) -> Option<String> {
        let item = self
            .wallpapers
            .iter()
            .find(|item| item.unsplash.as_deref() == Some(photo))?;
        Some(crate::collections::name(
            self.wallpaper_collection(&item.id),
        ))
    }

    /// A Discover suggestion was saved as a wallpaper, or failed to be.
    pub fn suggestion_added(&mut self, id: &str, result: Result<(), String>) {
        if let Some(item) = self
            .discover
            .photos
            .iter_mut()
            .find(|item| item.suggestion.photo.id == id)
        {
            item.status = if result.is_ok() {
                SuggestionStatus::Added
            } else {
                SuggestionStatus::Failed
            };
        }
        if let Err(error) = result {
            self.notify(error);
        }
        self.dirty = true;
    }

    /// Custom collections, which can follow a schedule.
    pub fn custom_collections(&self) -> Vec<String> {
        self.collections()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| crate::collections::is_custom(id))
            .collect()
    }

    /// Works out which collection should be showing; true when that changed.
    pub fn refresh_collection(&mut self, now: time::OffsetDateTime) -> bool {
        let active = crate::collections::active(
            now,
            self.preferences.collection_pick.as_ref(),
            self.preferences.collection_schedules.iter(),
            |id| self.collection_size(id) > 0,
            self.preferences.holiday_mode,
        );
        if active == self.collection {
            return false;
        }
        // A holiday just started mixing in: show one of its wallpapers right away.
        if active.mix.is_some() && active.mix != self.collection.mix {
            self.mix_due = true;
        }
        self.collection = active;
        self.dirty = true;
        true
    }

    /// The wallpaper on screen isn't from the active collection, which has some to show.
    pub fn collection_due(&self) -> bool {
        let current = self.wallpaper_collection(&self.preferences.background);
        let mixed = self
            .collection
            .mix
            .as_ref()
            .is_some_and(|(holiday, _)| holiday == current);
        self.mix_due
            || (current != self.collection.id
                && !mixed
                && self.collection_size(&self.collection.id) > 0)
    }

    /// The gallery: wallpapers of the collection it shows.
    fn background_ids(&self) -> impl Iterator<Item = &str> {
        let collection = self.gallery_collection();
        self.all_background_ids()
            .filter(move |id| self.wallpaper_collection(id) == collection)
    }

    fn all_background_ids(&self) -> impl Iterator<Item = &str> {
        BACKGROUNDS
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| {
                *id == self.preferences.background
                    || !self
                        .preferences
                        .deleted_backgrounds
                        .iter()
                        .any(|deleted| deleted == id)
            })
            .chain(self.wallpapers.iter().map(|item| item.id.as_str()))
    }

    pub fn can_delete_background(&self, id: &str) -> bool {
        id != self.preferences.background && self.background_ids().any(|known| known == id)
    }

    fn background_target(&self, id: &str) -> Target {
        Target::Background(
            self.background_ids()
                .position(|known| known == id)
                .unwrap_or(0),
        )
    }

    pub fn wallpaper_deleted(&mut self, id: String, result: Result<(), String>) {
        if self.deleting_wallpaper.as_deref() != Some(&id) {
            return;
        }
        self.deleting_wallpaper = None;
        if let Err(error) = result {
            eprintln!("Luma background deletion: {error}");
            self.notify(self.preferences.language.t("deleteBackgroundError"));
            return;
        }
        self.finish_background_delete(&id);
    }

    fn finish_background_delete(&mut self, id: &str) {
        let index = self
            .background_ids()
            .position(|known| known == id)
            .unwrap_or(0);
        let focused = match self.focus {
            Target::Background(index) => self.background_id(index).map(str::to_owned),
            _ => None,
        };
        self.wallpapers.retain(|item| item.id != id);
        if BACKGROUNDS.iter().any(|(known, _)| *known == id) {
            self.preferences.deleted_backgrounds.push(id.to_owned());
        }
        if matches!(&self.overlay, Some(Overlay::BackgroundOptions { id: current, .. } | Overlay::BackgroundDelete { id: current, .. }) if current == id)
        {
            self.overlay = None;
            self.panel_started = None;
            self.focus = Target::Background(index.min(self.background_count().saturating_sub(1)));
            self.pointer = false;
        } else if let Some(focused) = focused {
            self.focus = self.background_target(&focused);
        }
        self.pressed = None;
        self.notify(self.preferences.language.t("backgroundDeleted"));
    }

    pub fn set_wallpapers(&mut self, wallpapers: Vec<crate::wallpapers::Wallpaper>) {
        // A newly uploaded ID can sort before existing entries. Keep the focused
        // image, rather than moving the pointer's target to a different photograph.
        let focused_id = if let Target::Background(index) = self.focus {
            self.background_id(index).map(str::to_owned)
        } else {
            None
        };
        self.wallpapers = wallpapers;
        if let Some(
            Overlay::BackgroundOptions { id, restore } | Overlay::BackgroundDelete { id, restore },
        ) = &self.overlay
            && !self.background_ids().any(|known| known == id)
        {
            let index = if let Target::Background(index) = restore {
                *index
            } else {
                0
            };
            self.focus = Target::Background(index.min(self.background_count().saturating_sub(1)));
            self.overlay = None;
            self.panel_started = None;
            self.pressed = None;
            self.pointer = false;
        }
        if let Some(id) = focused_id {
            let index = (0..self.background_count())
                .find(|&i| self.background_id(i) == Some(id.as_str()))
                .unwrap_or(0);
            self.focus = Target::Background(index);
        }
        self.dirty = true;
    }

    fn section_entry(&self) -> Target {
        match self.section {
            1 => Target::Background(
                (0..self.background_count())
                    .position(|index| {
                        self.background_id(index) == Some(self.preferences.background.as_str())
                    })
                    .unwrap_or(0),
            ),
            WIDGETS_SECTION => Target::Widget(4),
            SYSTEM_SECTION if self.available_update.is_some() => Target::InstallUpdate,
            SYSTEM_SECTION => Target::CheckUpdates,
            section => Target::Setting(
                section_rows(section, self.preferences.ui_sounds)
                    .first()
                    .copied()
                    .unwrap_or(0),
            ),
        }
    }

    fn close_subpage(&mut self) {
        if self.subpage == Some(Subpage::WeatherLocation) {
            self.weather.cancel_search();
        }
        self.panel_started = None;
        self.pointer = false;
        self.focus = match self.subpage.take() {
            Some(Subpage::AppNames) => Target::Setting(0),
            Some(Subpage::Greeting) => Target::Widget(0),
            Some(Subpage::Date) => Target::Widget(4),
            Some(Subpage::DateFormat) => {
                self.subpage = Some(Subpage::Date);
                Target::DateFormat
            }
            Some(Subpage::Weather) => Target::Widget(1),
            Some(Subpage::Shortcuts) => Target::Widget(2),
            Some(Subpage::Inputs) => Target::Widget(3),
            Some(Subpage::Clock) => Target::Setting(1),
            Some(Subpage::QuickPanel) => Target::Setting(QUICK_PANEL_ROW),
            Some(Subpage::ClockFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockFormat
            }
            Some(Subpage::TimeFormat) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockTimeFormat
            }
            Some(Subpage::ClockSize) => {
                self.subpage = Some(Subpage::Clock);
                Target::ClockSize
            }
            Some(Subpage::InputIcons(id)) => {
                self.subpage = Some(Subpage::Input(id));
                Target::InputIcon
            }
            Some(Subpage::Input(id)) => {
                self.subpage = Some(Subpage::Inputs);
                Target::InputMode(id)
            }
            Some(Subpage::WeatherLocation) => {
                self.subpage = Some(Subpage::Weather);
                Target::WeatherLocation
            }
            Some(Subpage::Temperature) => {
                self.subpage = Some(Subpage::Weather);
                Target::WeatherTemperature
            }
            Some(Subpage::Language) => Target::Setting(4),
            Some(Subpage::TextRendering) => Target::Setting(7),
            Some(Subpage::Collections | Subpage::MoveCollection(_)) => {
                self.subpage = None;
                Target::WallpaperCollection
            }
            Some(Subpage::Discover) => {
                self.subpage = None;
                Target::DiscoverOpen
            }
            None => Target::Sidebar(self.section),
        };
        self.scroll = 0.;
        self.dirty = true;
    }

    pub fn direction(&mut self, direction: Direction, controls: &[Control]) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if self.recents_open() {
            return self.recent_direction(direction);
        }
        if matches!(self.overlay, Some(Overlay::SystemKeyboard)) {
            // The TV keyboard owns its arrows until Done/Back dismisses it.
            return vec![];
        }
        if matches!(
            self.overlay,
            Some(Overlay::WallpaperUpload | Overlay::TileEditor)
        ) {
            return vec![];
        }
        if let Some(id) = self.moving.clone() {
            let mut ids = self.preferences.favorites(&self.inventory.apps);
            let slots: Vec<_> = ids
                .iter()
                .enumerate()
                .filter_map(|(index, id)| {
                    (!self.preferences.hidden.contains(id)
                        && self.inventory.apps.iter().any(|app| &app.id == id))
                    .then_some(index)
                })
                .collect();
            if let Some(index) = slots.iter().position(|slot| ids[*slot] == id) {
                let next = match direction {
                    Direction::Left => index.saturating_sub(1),
                    Direction::Right => (index + 1).min(slots.len() - 1),
                    _ => index,
                };
                if next != index {
                    ids.swap(slots[index], slots[next]);
                    self.preferences.favorites = Some(ids);
                    self.dirty = true;
                    return vec![Effect::Save];
                }
            }
            return vec![];
        }
        if self.overlay.is_none() {
            if self.page == Page::Home
                && let Target::App(id) = &self.focus
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    match direction {
                        Direction::Left | Direction::Right => {
                            let next = if matches!(direction, Direction::Left) {
                                index.saturating_sub(1)
                            } else {
                                (index + 1).min(apps.len() - 1)
                            };
                            let id = apps[next].id.clone();
                            self.focus(Target::App(id), controls);
                        }
                        Direction::Up => {
                            // Up reaches the gear (Luma settings) at the top right.
                            self.focus(Target::Nav(2), controls)
                        }
                        Direction::Down => {
                            let x = controls
                                .iter()
                                .find(|control| control.target == self.focus)
                                .map_or(960., |control| control.rect.center().0);
                            let top = controls
                                .iter()
                                .filter(|control| matches!(control.target, Target::Input(_)))
                                .map(|control| control.rect.y)
                                .fold(f32::INFINITY, f32::min);
                            let next = controls
                                .iter()
                                .filter(|control| {
                                    matches!(control.target, Target::Input(_))
                                        && (control.rect.y - top).abs() < 1.
                                })
                                .min_by(|a, b| {
                                    (a.rect.center().0 - x)
                                        .abs()
                                        .total_cmp(&(b.rect.center().0 - x).abs())
                                })
                                .map(|control| control.target.clone());
                            if let Some(next) = next {
                                self.focus(next, controls);
                            } else {
                                // Nothing below the favourites: Down opens the app drawer.
                                self.open_drawer();
                            }
                        }
                    }
                    return vec![];
                }
            }
            if self.focus == Target::InputsButton {
                match direction {
                    Direction::Left => {}
                    Direction::Right => self.focus(Target::TvSettings, controls),
                    Direction::Up => {}
                    Direction::Down => self.focus(self.first_content(), controls),
                }
                return vec![];
            }
            let setting_rows: Option<&[Target]> = match self.subpage {
                Some(Subpage::Greeting) => Some(&[
                    Target::GreetingToggle,
                    Target::GreetingName,
                    Target::GreetingNameClear,
                ]),
                Some(Subpage::AppNames) => Some(&[Target::AppNamesHome, Target::AppNamesApps]),
                Some(Subpage::Date) => Some(&[Target::DateToggle, Target::DateFormat]),
                Some(Subpage::Clock) => Some(&[
                    Target::ClockFormat,
                    Target::ClockTimeFormat,
                    Target::ClockSize,
                    Target::ClockPosition,
                    Target::ClockStyle,
                ]),
                Some(Subpage::QuickPanel) => Some(&QUICK_TOGGLES),
                _ => None,
            };
            if let Some(rows) = setting_rows
                && let Some(index) = rows.iter().position(|target| *target == self.focus)
            {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(2)
                        } else {
                            rows[index - 1].clone()
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Right
                        if matches!(
                            self.focus,
                            Target::GreetingName
                                | Target::ClockFormat
                                | Target::ClockTimeFormat
                                | Target::ClockSize
                                | Target::ClockPosition
                                | Target::ClockStyle
                                | Target::DateFormat
                        ) =>
                    {
                        return self.activate();
                    }
                    _ => {}
                }
                return vec![];
            }
            if let Target::Choice(index) = self.focus {
                let count = match self.subpage {
                    Some(
                        Subpage::ClockFormat
                        | Subpage::TimeFormat
                        | Subpage::Temperature
                        | Subpage::TextRendering,
                    ) => 2,
                    Some(Subpage::InputIcons(_)) => crate::model::INPUT_ICONS.len() + 1,
                    Some(Subpage::Language) => Language::ALL.len(),
                    // The last row makes a new collection.
                    Some(Subpage::Collections) => {
                        self.collections().len() + self.custom_collections().len() + 1
                    }
                    Some(Subpage::MoveCollection(_)) => self.collections().len(),
                    Some(Subpage::ClockSize) => crate::model::ClockSize::ALL.len(),
                    Some(Subpage::DateFormat) => crate::model::DateFormat::ALL.len(),
                    _ => 3,
                };
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Nav(2)
                        } else {
                            Target::Choice(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down
                        if index + 1 == count
                            && matches!(self.subpage, Some(Subpage::Input(_))) =>
                    {
                        self.focus(Target::InputRename, controls)
                    }
                    Direction::Down => self.focus(Target::Choice((index + 1) % count), controls),
                    Direction::Right => {}
                }
                return vec![];
            }
            if matches!(
                self.focus,
                Target::InputRename
                    | Target::InputNameReset
                    | Target::InputIcon
                    | Target::InputInMenu
            ) {
                let mut rows = vec![Target::InputRename];
                if controls.iter().any(|c| c.target == Target::InputNameReset) {
                    rows.push(Target::InputNameReset);
                }
                rows.push(Target::InputIcon);
                if controls.iter().any(|c| c.target == Target::InputInMenu) {
                    rows.push(Target::InputInMenu);
                }
                let index = rows
                    .iter()
                    .position(|target| *target == self.focus)
                    .unwrap_or(0);
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::Choice(2)
                        } else {
                            rows[index - 1].clone()
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < rows.len() => {
                        self.focus(rows[index + 1].clone(), controls)
                    }
                    Direction::Right if self.focus == Target::InputIcon => return self.activate(),
                    _ => {}
                }
                return vec![];
            }
            if let Target::Sidebar(index) = self.focus {
                match direction {
                    Direction::Right => {
                        // A pointer may have left focus on a section that isn't open yet.
                        if index != self.section {
                            self.section = index;
                            self.subpage = None;
                            self.scroll = 0.;
                        }
                        let entry = if self.section == 1 {
                            Target::UploadWallpaper
                        } else {
                            self.section_entry()
                        };
                        self.focus(entry, controls);
                    }
                    Direction::Up => self.focus(
                        match section_position(index) {
                            0 => Target::Nav(2),
                            position => Target::Sidebar(SECTION_ORDER[position - 1]),
                        },
                        controls,
                    ),
                    Direction::Down => self.focus(
                        Target::Sidebar(
                            SECTION_ORDER[(section_position(index) + 1) % SECTION_ORDER.len()],
                        ),
                        controls,
                    ),
                    Direction::Left => {}
                }
                return vec![];
            }
            if (matches!(
                self.focus,
                Target::TileEditor
                    | Target::Tile(_)
                    | Target::InputMode(_)
                    | Target::WeatherToggle
                    | Target::WeatherLocation
                    | Target::WeatherTemperature
                    | Target::WeatherForecast
                    | Target::Widget(_)
            ) || (matches!(self.focus, Target::Setting(3) | Target::InputsMenu)
                && self.subpage == Some(Subpage::Inputs)))
                && matches!(direction, Direction::Left)
            {
                self.close_subpage();
                return vec![];
            }
            if matches!(
                self.focus,
                Target::Setting(0 | 1 | 4 | 7)
                    | Target::Widget(_)
                    | Target::InputMode(_)
                    | Target::WeatherLocation
                    | Target::WeatherTemperature
            ) && matches!(direction, Direction::Right)
            {
                return self.activate();
            }
            if matches!(
                self.focus,
                Target::Setting(_) | Target::WeatherToggle | Target::WeatherForecast
            ) && matches!(direction, Direction::Right)
            {
                return vec![];
            }
            if let Target::InputMode(id) = &self.focus
                && matches!(direction, Direction::Up)
                && self
                    .inventory
                    .inputs
                    .first()
                    .is_some_and(|input| &input.id == id)
            {
                self.focus(Target::Nav(2), controls);
                return vec![];
            }
            if self.focus == Target::Setting(0) && matches!(direction, Direction::Up) {
                self.focus(Target::Nav(2), controls);
                return vec![];
            }
            if self.focus == Target::WeatherToggle && matches!(direction, Direction::Up) {
                self.focus(Target::Nav(2), controls);
                return vec![];
            }
            if matches!(
                self.focus,
                Target::WeatherQuery | Target::WeatherSearch | Target::WeatherClear
            ) {
                match direction {
                    Direction::Left => {
                        self.close_subpage();
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(
                            if self.preferences.weather_location.is_some()
                                && self.focus != Target::WeatherClear
                            {
                                Target::WeatherClear
                            } else {
                                Target::Nav(2)
                            },
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Down => {
                        let target = if self.focus == Target::WeatherClear {
                            Some(Target::WeatherQuery)
                        } else if !self.weather.results.is_empty() {
                            Some(Target::WeatherResult(0))
                        } else if self.focus == Target::WeatherQuery {
                            Some(Target::WeatherSearch)
                        } else {
                            None
                        };
                        if let Some(target) = target {
                            self.focus(target, controls);
                        }
                        return vec![];
                    }
                    _ => {}
                }
            }
            if let Target::WeatherResult(index) = self.focus {
                match direction {
                    Direction::Left => self.close_subpage(),
                    Direction::Up => self.focus(
                        if index == 0 {
                            Target::WeatherQuery
                        } else {
                            Target::WeatherResult(index - 1)
                        },
                        controls,
                    ),
                    Direction::Down if index + 1 < self.weather.results.len() => {
                        self.focus(Target::WeatherResult(index + 1), controls)
                    }
                    _ => {}
                }
                return vec![];
            }
            if self.focus == Target::Search {
                match direction {
                    Direction::Left => {
                        self.focus(
                            Target::Filter(if self.hidden_count() > 0 { 5 } else { 4 }),
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Up => return self.close_drawer(),
                    _ => {}
                }
            }
            // The drawer's top row (filters and search) goes back up to Home.
            if self.page == Page::Apps
                && matches!(direction, Direction::Up)
                && matches!(
                    self.focus,
                    Target::Filter(_) | Target::ClearFilter | Target::Search | Target::ClearSearch
                )
            {
                return self.close_drawer();
            }
            // The gear stands alone at the top right: Left and Right stay on it.
            if self.page == Page::Home
                && matches!(self.focus, Target::Nav(_))
                && matches!(direction, Direction::Left | Direction::Right)
            {
                return vec![];
            }
            // Home's top-right buttons come back down to the favourites.
            if self.page == Page::Home
                && matches!(direction, Direction::Down)
                && matches!(self.focus, Target::TvSettings | Target::InputsButton)
            {
                self.focus(self.first_content(), controls);
                return vec![];
            }
            if self.page == Page::Apps
                && let Target::App(id) = &self.focus
                && matches!(direction, Direction::Up | Direction::Down)
            {
                let apps = self.apps();
                if let Some(index) = apps.iter().position(|app| &app.id == id) {
                    // A preceding row may be above the visible scroll region.
                    // Its grid index still defines the route; reveal_focus then
                    // scrolls it into view, without letting the fixed nav steal Up.
                    let next = match direction {
                        Direction::Up if index < APPS_COLUMNS => match self.drawer_top() {
                            Some(target) => Some(target),
                            None => return self.close_drawer(),
                        },
                        Direction::Up => Some(Target::App(apps[index - APPS_COLUMNS].id.clone())),
                        Direction::Down
                            if (index / APPS_COLUMNS + 1) * APPS_COLUMNS < apps.len() =>
                        {
                            Some(Target::App(
                                apps[(index + APPS_COLUMNS).min(apps.len() - 1)].id.clone(),
                            ))
                        }
                        _ => None,
                    };
                    if let Some(next) = next {
                        self.focus(next, controls);
                    }
                    return vec![];
                }
            }
            if let Target::Background(index) = self.focus {
                match direction {
                    Direction::Right if index + 1 < self.background_count() => {
                        self.focus(Target::Background(index + 1), controls);
                        return vec![];
                    }
                    Direction::Right => return vec![],
                    Direction::Left if index % 3 != 0 => {
                        self.focus(Target::Background(index - 1), controls);
                        return vec![];
                    }
                    Direction::Up => {
                        self.focus(
                            if index == 2 {
                                Target::WallpaperCycle
                            } else if index == 1 {
                                Target::UploadWallpaper
                            } else if index == 0 {
                                Target::WallpaperCollection
                            } else {
                                Target::Background(index - 3)
                            },
                            controls,
                        );
                        return vec![];
                    }
                    Direction::Down => {
                        if (index / 3 + 1) * 3 < self.background_count() {
                            self.focus(
                                Target::Background((index + 3).min(self.background_count() - 1)),
                                controls,
                            );
                        }
                        return vec![];
                    }
                    _ => {}
                }
            }
            if self.subpage == Some(Subpage::Discover)
                && (matches!(self.focus, Target::Suggestion(_) | Target::MoreSuggestions)
                    || (matches!(self.focus, Target::Nav(_))
                        && matches!(direction, Direction::Down)))
            {
                let count = self.discover.photos.len();
                match (self.focus.clone(), direction) {
                    (Target::Suggestion(index), Direction::Left) if index % 4 == 0 => {
                        self.close_subpage()
                    }
                    (Target::Suggestion(index), Direction::Left) => {
                        self.focus(Target::Suggestion(index - 1), controls)
                    }
                    (Target::Suggestion(index), Direction::Right) => {
                        if index % 4 < 3 && index + 1 < count {
                            self.focus(Target::Suggestion(index + 1), controls)
                        }
                    }
                    (Target::Suggestion(index), Direction::Up) => self.focus(
                        if index < 4 {
                            Target::Nav(2)
                        } else {
                            Target::Suggestion(index - 4)
                        },
                        controls,
                    ),
                    (Target::Suggestion(index), Direction::Down) => self.focus(
                        if index + 4 < count {
                            Target::Suggestion(index + 4)
                        } else {
                            Target::MoreSuggestions
                        },
                        controls,
                    ),
                    (Target::MoreSuggestions, Direction::Up) => self.focus(
                        if count > 0 {
                            Target::Suggestion(count - 1)
                        } else {
                            Target::Nav(2)
                        },
                        controls,
                    ),
                    (Target::MoreSuggestions, Direction::Left) => self.close_subpage(),
                    (Target::Nav(_), _) => self.focus(
                        if count > 0 {
                            Target::Suggestion(0)
                        } else {
                            Target::MoreSuggestions
                        },
                        controls,
                    ),
                    _ => {}
                }
                return vec![];
            }
            if self.focus == Target::DiscoverOpen {
                match direction {
                    Direction::Down if self.removable_collection().is_some() => {
                        self.focus(Target::RemoveCollection, controls)
                    }
                    Direction::Down => self.focus(
                        Target::Background(1.min(self.background_count().saturating_sub(1))),
                        controls,
                    ),
                    Direction::Up => self.focus(Target::Nav(2), controls),
                    Direction::Left => self.focus(Target::WallpaperCollection, controls),
                    Direction::Right => self.focus(Target::UploadWallpaper, controls),
                }
                return vec![];
            }
            if self.focus == Target::RemoveCollection {
                match direction {
                    Direction::Up => self.focus(Target::WallpaperCollection, controls),
                    Direction::Left => self.focus(Target::Sidebar(1), controls),
                    _ => {}
                }
                return vec![];
            }
            if self.focus == Target::WallpaperCollection {
                match direction {
                    Direction::Down => {
                        if self.background_count() > 0 {
                            self.focus(Target::Background(0), controls)
                        } else if self.removable_collection().is_some() {
                            self.focus(Target::RemoveCollection, controls)
                        }
                    }
                    Direction::Up => self.focus(Target::Nav(2), controls),
                    Direction::Left => self.focus(Target::Sidebar(1), controls),
                    Direction::Right => self.focus(
                        if self.unsplash_ready {
                            Target::DiscoverOpen
                        } else {
                            Target::UploadWallpaper
                        },
                        controls,
                    ),
                }
                return vec![];
            }
            if self.focus == Target::UploadWallpaper {
                match direction {
                    Direction::Down => self.focus(
                        Target::Background(1.min(self.background_count().saturating_sub(1))),
                        controls,
                    ),
                    Direction::Up => self.focus(Target::Nav(2), controls),
                    Direction::Left => self.focus(
                        if self.unsplash_ready {
                            Target::DiscoverOpen
                        } else {
                            Target::WallpaperCollection
                        },
                        controls,
                    ),
                    Direction::Right => self.focus(Target::WallpaperGradient, controls),
                }
                return vec![];
            }
            if self.focus == Target::WallpaperGradient {
                match direction {
                    Direction::Right => self.focus(Target::WallpaperCycle, controls),
                    Direction::Left => self.focus(Target::UploadWallpaper, controls),
                    Direction::Up => self.focus(Target::Nav(2), controls),
                    Direction::Down => self.focus(
                        Target::Background(1.min(self.background_count().saturating_sub(1))),
                        controls,
                    ),
                }
                return vec![];
            }
            if self.focus == Target::WallpaperCycle {
                match direction {
                    Direction::Right => {}
                    Direction::Left => self.focus(Target::WallpaperGradient, controls),
                    Direction::Up => self.focus(Target::Nav(2), controls),
                    Direction::Down => self.focus(
                        Target::Background(2.min(self.background_count().saturating_sub(1))),
                        controls,
                    ),
                }
                return vec![];
            }
            if matches!(self.focus, Target::Nav(_)) && matches!(direction, Direction::Down) {
                if self.page == Page::Settings {
                    self.subpage = None;
                    self.section = 0;
                }
                self.focus(self.first_content(), controls);
                return vec![];
            }
            if self.page == Page::Settings
                && matches!(direction, Direction::Left)
                && !matches!(
                    self.focus,
                    Target::Nav(_) | Target::Sidebar(_) | Target::TvSettings
                )
            {
                if let Target::Background(index) = self.focus {
                    if index % 3 != 0 { /* left follows gallery order */
                    } else {
                        self.focus(Target::Sidebar(self.section), controls);
                        return vec![];
                    }
                } else {
                    self.focus(Target::Sidebar(self.section), controls);
                    return vec![];
                }
            }
        }
        let Some(current) = controls.iter().find(|c| c.target == self.focus) else {
            self.focus(self.first_content(), controls);
            return vec![];
        };
        let (x, y) = current.rect.center();
        let next = controls
            .iter()
            .filter(|candidate| candidate.target != self.focus)
            // The "More apps" hint is for the pointer: Down past the last row opens the drawer.
            .filter(|candidate| candidate.target != Target::MoreApps)
            .filter(|candidate| {
                self.page != Page::Home
                    || !matches!(candidate.target, Target::App(_))
                    || (candidate.rect.x >= HOME_STRIP_LEFT - 0.1
                        && candidate.rect.x + candidate.rect.w <= HOME_STRIP_RIGHT + 0.1)
            })
            .filter_map(|candidate| {
                // Header controls never steal an Up route from the active tab.
                if self.overlay.is_none()
                    && matches!(direction, Direction::Up)
                    && candidate.rect.y < 150.
                    && candidate.target != Target::Nav(2)
                {
                    return None;
                }
                let (cx, cy) = candidate.rect.center();
                let (primary, cross, overlap) = match direction {
                    Direction::Up => (
                        y - cy,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Down => (
                        cy - y,
                        (x - cx).abs(),
                        (current.rect.w + candidate.rect.w) / 2.,
                    ),
                    Direction::Left => (
                        x - cx,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                    Direction::Right => (
                        cx - x,
                        (y - cy).abs(),
                        (current.rect.h + candidate.rect.h) / 2.,
                    ),
                };
                (primary > 1.).then_some((
                    candidate.target.clone(),
                    primary + cross * 2. + if cross >= overlap { 10000. } else { 0. },
                ))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(target, _)| target);
        if let Some(next) = next {
            self.focus(next, controls);
        } else if self.page == Page::Home
            && self.overlay.is_none()
            && matches!(direction, Direction::Down)
            && matches!(
                self.focus,
                Target::App(_) | Target::Input(_) | Target::MoreApps
            )
        {
            self.open_drawer();
        }
        vec![]
    }

    fn quick(&mut self, key: crate::model::QuickKey) -> Vec<Effect> {
        use crate::model::QuickKey;
        match key {
            QuickKey::Ambient => {
                self.close_quick();
                if self.page != Page::Home {
                    self.open(0);
                }
                self.focus = self.first_content();
                self.ambient = Some(Instant::now());
                self.ambient_manual = true;
                vec![]
            }
            QuickKey::Inputs => {
                if let Some(first) = self.input_choices().into_iter().next() {
                    self.overlay = Some(Overlay::Inputs);
                    self.focus = Target::InputPick(first);
                    self.pressed = None;
                }
                vec![]
            }
            QuickKey::TvSettings => {
                self.close_quick();
                vec![Effect::Launch("com.palm.app.settings".into())]
            }
            QuickKey::Collections => {
                self.close_quick();
                self.open(2);
                self.section = BACKGROUND_SECTION;
                self.gallery_collection = None;
                self.focus = Target::WallpaperCollection;
                self.activate()
            }
        }
    }

    pub fn options(&mut self) {
        if self.overlay.is_some() || self.moving.is_some() {
            return;
        }
        if let Target::Background(index) = self.focus {
            if let Some(id) = self.background_id(index).map(str::to_owned) {
                self.overlay = Some(Overlay::BackgroundOptions {
                    id,
                    restore: self.focus.clone(),
                });
                self.focus = Target::UseBackground;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                self.dirty = true;
            }
        } else if let Target::Tile(id) = &self.focus {
            if self.preferences.custom_tile(id).is_none() {
                return;
            }
            self.overlay = Some(Overlay::Options {
                id: id.clone(),
                restore: self.focus.clone(),
            });
            self.focus = Target::TileTest(id.clone());
            self.panel_started = (!self.preferences.reduced_motion).then(Instant::now);
            self.dirty = true;
        } else if let Target::App(id) = &self.focus {
            self.overlay = Some(Overlay::Options {
                id: id.clone(),
                restore: self.focus.clone(),
            });
            self.focus = Target::OpenApp;
            if !self.preferences.reduced_motion {
                self.panel_started = Some(Instant::now());
            }
            self.dirty = true;
        }
    }

    pub fn uninstall_response(&mut self, id: String, result: Result<(), String>) -> Vec<Effect> {
        if self.uninstalling.as_ref() != Some(&id) {
            return vec![];
        }
        self.uninstalling = None;
        self.dirty = true;
        if let Err(error) = result {
            eprintln!("Luma uninstall: {error}");
            self.notify(self.preferences.language.t("deleteAppError"));
            if matches!(&self.overlay, Some(Overlay::Delete { id: current, .. } | Overlay::Options { id: current, .. }) if current == &id)
            {
                if let Some(Overlay::Delete { restore, .. } | Overlay::Options { restore, .. }) =
                    self.overlay.take()
                {
                    self.focus = restore;
                }
                self.panel_started = None;
                self.pressed = None;
                self.reconcile_focus();
            }
            // Even an uncertain/failed mutation may have changed TV inventory.
            // Refresh is read-only, and also clears a stale card after late removal.
            return vec![Effect::Refresh];
        }
        let old_ids: Vec<_> = self.apps().iter().map(|app| app.id.clone()).collect();
        let index = old_ids.iter().position(|item| item == &id).unwrap_or(0);
        let mut favorites = self.preferences.favorites(&self.inventory.apps);
        favorites.retain(|item| item != &id);
        self.preferences.favorites = Some(favorites);
        self.preferences.hidden.retain(|item| item != &id);
        self.inventory.apps.retain(|app| app.id != id);
        if self.filter == 5 && self.hidden_count() == 0 {
            self.filter = 0;
        }
        if matches!(&self.overlay, Some(Overlay::Delete { id: current, .. } | Overlay::Options { id: current, .. }) if current == &id)
        {
            self.overlay = None;
            self.panel_started = None;
            self.focus = Target::App(id.clone());
        }
        if self.focus == Target::App(id) {
            self.focus = self
                .apps()
                .get(index.min(self.apps().len().saturating_sub(1)))
                .map(|app| Target::App(app.id.clone()))
                .unwrap_or_else(|| self.first_content());
        }
        self.reconcile_focus();
        self.notify(self.preferences.language.t("appDeleted"));
        vec![Effect::Save, Effect::Refresh]
    }

    pub fn hold(&mut self) {
        if self.page == Page::Home && self.overlay.is_none() {
            if let Target::App(id) = &self.focus {
                self.moving = Some(id.clone());
                self.notice = None;
                self.dirty = true;
            }
        } else {
            self.options();
        }
    }

    pub fn text(&mut self, text: &str) {
        if self.text_input_active() {
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(120usize.saturating_sub(query.chars().count())),
                );
                self.weather.edit(query);
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::Name {
                let name = &mut self.greeting_name_draft;
                name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(40usize.saturating_sub(name.chars().count())),
                );
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::CollectionName {
                let length = self.collection_name.chars().count();
                self.collection_name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(32usize.saturating_sub(length)),
                );
                self.dirty = true;
                return;
            }
            if self.keyboard_field == TextField::InputName {
                let length = self.input_name.chars().count();
                self.input_name.extend(
                    text.chars()
                        .filter(|c| !c.is_control())
                        .take(INPUT_NAME_LIMIT.saturating_sub(length)),
                );
                self.dirty = true;
                return;
            }
            self.query.extend(
                text.chars()
                    .filter(|c| !c.is_control())
                    .take(120usize.saturating_sub(self.query.chars().count())),
            );
            self.scroll = 0.;
            self.dirty = true;
        }
    }

    pub fn delete(&mut self) {
        if self.text_input_active() {
            if self.keyboard_field == TextField::Location {
                let mut query = self.weather.query.clone();
                query.pop();
                self.weather.edit(query);
            } else if self.keyboard_field == TextField::Name {
                self.greeting_name_draft.pop();
            } else if self.keyboard_field == TextField::InputName {
                self.input_name.pop();
            } else if self.keyboard_field == TextField::CollectionName {
                self.collection_name.pop();
            } else {
                self.query.pop();
            }
            self.dirty = true;
        }
    }

    pub fn text_input_active(&self) -> bool {
        matches!(
            self.overlay,
            Some(Overlay::Keyboard | Overlay::SystemKeyboard)
        )
    }

    pub fn finish_text_input(&mut self) -> Vec<Effect> {
        if self.text_input_active() {
            self.overlay = None;
            self.panel_started = None;
            self.focus = self.keyboard_return();
            self.pointer = false;
            self.dirty = true;
            return self.commit_text_input();
        }
        vec![]
    }

    fn commit_text_input(&mut self) -> Vec<Effect> {
        if self.keyboard_field == TextField::Name {
            let name = std::mem::take(&mut self.greeting_name_draft);
            let name = name.trim();
            // Empty dismissal preserves the saved value; Clear name is explicit.
            if !name.is_empty() && self.preferences.greeting_name != name {
                self.preferences.greeting_name = name.into();
                return vec![Effect::Save];
            }
            return vec![];
        }
        if self.keyboard_field == TextField::CollectionName {
            return self.commit_collection_name();
        }
        self.commit_input_name()
    }

    /// A name typed for a new collection: make it (or find the existing one with that name)
    /// and open its gallery, where Discover searches Unsplash for the name.
    fn commit_collection_name(&mut self) -> Vec<Effect> {
        let typed = std::mem::take(&mut self.collection_name);
        let Some(id) = crate::collections::custom_id(&typed) else {
            return vec![];
        };
        let existing = self
            .collections()
            .into_iter()
            .map(|(id, _)| id)
            .find(|known| known.to_lowercase() == id.to_lowercase());
        let id = match existing {
            Some(known) => known,
            None => {
                self.preferences.created_collections.push(id.clone());
                id
            }
        };
        self.gallery_collection = Some(id);
        self.subpage = None;
        self.scroll = 0.;
        self.focus = if self.unsplash_ready {
            Target::DiscoverOpen
        } else {
            Target::UploadWallpaper
        };
        self.dirty = true;
        vec![Effect::Save]
    }

    /// An empty collection made on the TV, which its empty gallery offers to remove.
    pub fn removable_collection(&self) -> Option<&str> {
        let id = self.gallery_collection();
        (self.background_count() == 0
            && self
                .preferences
                .created_collections
                .iter()
                .any(|known| known == id))
        .then_some(id)
    }

    /// Closing the keyboard by any route keeps a typed input name. An empty
    /// field leaves the saved name unchanged; Clear name removes it explicitly.
    fn commit_input_name(&mut self) -> Vec<Effect> {
        let name = std::mem::take(&mut self.input_name);
        if self.keyboard_field != TextField::InputName || clean_input_name(&name).is_empty() {
            return vec![];
        }
        let Some(Subpage::Input(id)) = self.subpage.clone() else {
            return vec![];
        };
        if self.preferences.set_input_name(&id, &name) {
            vec![Effect::Save]
        } else {
            vec![]
        }
    }

    /// Applies the tile list edited on a phone or computer. New tiles join
    /// the end of the Home favorites and removed ones leave favorites and hidden apps.
    pub fn replace_tiles(&mut self, tiles: Vec<CustomTile>) -> Vec<Effect> {
        let removed: Vec<_> = self
            .preferences
            .custom_tiles
            .iter()
            .filter(|old| !tiles.iter().any(|tile| tile.id == old.id))
            .map(|old| old.id.clone())
            .collect();
        for id in &removed {
            self.preferences.remove_tile(id);
        }
        let added: Vec<_> = tiles
            .iter()
            .filter(|tile| self.preferences.custom_tile(&tile.id).is_none())
            .map(|tile| tile.id.clone())
            .collect();
        self.preferences.custom_tiles = tiles;
        self.preferences.sanitize();
        self.sync_custom_tiles();
        if let Target::Tile(id) | Target::TileRemove(id) | Target::TileTest(id) = &self.focus
            && self.preferences.custom_tile(id).is_none()
        {
            if matches!(
                &self.overlay,
                Some(Overlay::Options {
                    restore: Target::Tile(_),
                    ..
                })
            ) {
                self.overlay = None;
                self.panel_started = None;
            }
            self.focus = Target::TileEditor;
        }
        if !added.is_empty() {
            let mut favorites = self.preferences.favorites(&self.inventory.apps);
            for id in added {
                if self.preferences.custom_tile(&id).is_some() && !favorites.contains(&id) {
                    favorites.push(id);
                }
            }
            self.preferences.favorites = Some(favorites);
        }
        vec![Effect::Save]
    }

    fn send_tile(&mut self, tile: CustomTile) -> Vec<Effect> {
        self.notify(
            self.preferences
                .language
                .format("tileSending", &[("tile", &tile.title)]),
        );
        vec![Effect::FireWebhook(tile)]
    }

    pub fn delete_surrounding_text(&mut self, offset: i32, length: i32) {
        if !self.text_input_active() || length <= 0 {
            return;
        }
        // SDL-webOS forwards the native text model's UTF-8 byte offsets,
        // relative to the insertion cursor (at the end of our query).
        let mut query = self.editing_text().to_owned();
        let mut start =
            (query.len() as i64 + i64::from(offset)).clamp(0, query.len() as i64) as usize;
        let mut end = start.saturating_add(length as usize).min(query.len());
        while !query.is_char_boundary(start) {
            start -= 1;
        }
        while !query.is_char_boundary(end) {
            end += 1;
        }
        query.replace_range(start..end, "");
        if self.keyboard_field == TextField::Location {
            self.weather.edit(query);
        } else if self.keyboard_field == TextField::Name {
            self.greeting_name_draft = query;
        } else if self.keyboard_field == TextField::InputName {
            self.input_name = query;
        } else {
            self.query = query;
            self.scroll = 0.;
        }
        self.dirty = true;
    }

    pub fn editing_text(&self) -> &str {
        match self.keyboard_field {
            TextField::Location => &self.weather.query,
            TextField::Name => &self.greeting_name_draft,
            TextField::InputName => &self.input_name,
            TextField::CollectionName => &self.collection_name,
            TextField::Apps => &self.query,
        }
    }
    pub fn open_keyboard(&mut self, field: TextField) {
        self.keyboard_field = field;
        if field == TextField::Name {
            // SDL-webOS cannot pre-fill the system keyboard's text model.
            self.greeting_name_draft.clear();
        }
        if field == TextField::InputName {
            // The TV keyboard starts empty, so a new name replaces the old one.
            self.input_name.clear();
        }
        if field == TextField::CollectionName {
            self.collection_name.clear();
        }
        if cfg!(all(
            feature = "webos",
            target_os = "linux",
            target_arch = "arm"
        )) {
            self.overlay = Some(Overlay::SystemKeyboard);
            self.focus = match field {
                TextField::Location => Target::WeatherQuery,
                TextField::Name => Target::GreetingName,
                TextField::InputName => Target::InputRename,
                TextField::CollectionName => self.focus.clone(),
                TextField::Apps => Target::Search,
            };
            self.panel_started = None;
            self.dirty = true;
            return;
        }
        self.overlay = Some(Overlay::Keyboard);
        self.focus = Target::Key(0);
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn open_location(&mut self) {
        self.weather.edit(String::new());
        self.subpage = Some(Subpage::WeatherLocation);
        self.focus = Target::WeatherQuery;
        self.scroll = 0.;
        if !self.preferences.reduced_motion {
            self.panel_started = Some(Instant::now());
        }
        self.dirty = true;
    }
    fn keyboard_return(&self) -> Target {
        match self.keyboard_field {
            TextField::Location => Target::WeatherSearch,
            TextField::Name => Target::GreetingName,
            TextField::InputName => Target::InputRename,
            TextField::CollectionName => {
                Target::Choice(self.collections().len() + self.custom_collections().len())
            }
            TextField::Apps => Target::Search,
        }
    }

    pub fn activate(&mut self) -> Vec<Effect> {
        if self.recents_open() {
            use crate::recents::Command;
            return match self.focus.clone() {
                Target::Recent(id) => self.recent_request(Command::Launch(id)),
                Target::RecentRetry => self.recent_request(Command::List),
                _ => vec![],
            };
        }
        if self.moving.take().is_some() {
            // Order was saved at every move. A fresh OK release finishes the
            // mode and keeps the selected favorite, rather than launching it.
            self.dirty = true;
            return vec![];
        }
        self.dirty = true;
        if self.page != Page::Apps
            && let Target::App(id) = &self.focus
            && let Some(tile) = self.preferences.custom_tile(id).cloned()
        {
            return self.send_tile(tile);
        }
        match self.focus.clone() {
            Target::Nav(2) => return self.open_quick(),
            Target::Nav(index) => self.open(index),
            Target::QuickAll => {
                self.close_quick();
                // All settings opens on its first section, ready to move down the list.
                self.open(2);
                self.focus = self.first_content();
                self.ease_in_page();
            }
            Target::QuickToggle(key) => {
                self.preferences.toggle_quick(key);
                self.dirty = true;
                return vec![Effect::Save];
            }
            Target::Quick(key) => return self.quick(key),
            Target::Browse => self.open(1),
            Target::Retry => return vec![Effect::Reload],
            Target::DismissNotice => self.notice = None,
            Target::CloseDialog => return self.back(),
            Target::UploadWallpaper => {
                self.wallpaper_upload = None;
                self.overlay = Some(Overlay::WallpaperUpload);
                self.focus = Target::CloseDialog;
                self.pressed = None;
            }
            Target::TileEditor => {
                self.tile_editor = None;
                self.overlay = Some(Overlay::TileEditor);
                self.focus = Target::CloseDialog;
                self.pressed = None;
            }
            Target::TvSettings => return vec![Effect::Launch("com.palm.app.settings".into())],
            Target::MoreApps => self.open_drawer(),
            Target::ClockPosition => {
                self.preferences.clock_position = self.preferences.clock_position.next();
                self.dirty = true;
                return vec![Effect::Save];
            }
            Target::ClockStyle => {
                self.preferences.clock_style = self.preferences.clock_style.next();
                self.dirty = true;
                return vec![Effect::Save];
            }
            Target::InputsButton => {
                if let Some(first) = self.input_choices().into_iter().next() {
                    self.overlay = Some(Overlay::Inputs);
                    self.focus = Target::InputPick(first);
                    self.pressed = None;
                    if !self.preferences.reduced_motion {
                        self.panel_started = Some(Instant::now());
                    }
                }
            }
            Target::InputPick(id) if !self.launching => {
                self.overlay = None;
                self.panel_started = None;
                self.focus = Target::Nav(2);
                return vec![Effect::Launch(id)];
            }
            Target::InputsMenu => {
                self.preferences.inputs_menu = !self.preferences.inputs_menu;
                return vec![Effect::Save];
            }
            Target::App(_) if self.page == Page::Apps && !self.launching => self.options(),
            Target::App(id) | Target::Input(id) if !self.launching => {
                return vec![Effect::Launch(id)];
            }
            Target::OpenApp if !self.launching => {
                if let Some(Overlay::Options { id, .. }) = &self.overlay
                    && self.inventory.apps.iter().any(|app| &app.id == id)
                {
                    let id = id.clone();
                    self.back();
                    if let Some(tile) = self.preferences.custom_tile(&id).cloned() {
                        return self.send_tile(tile);
                    }
                    return vec![Effect::Launch(id)];
                }
            }
            Target::Filter(index) => {
                self.filter = if self.filter == index { 0 } else { index };
                self.scroll = 0.;
            }
            Target::ClearFilter => {
                self.filter = 0;
                self.focus = self.drawer_top().unwrap_or_else(|| self.first_content());
                self.scroll = 0.;
            }
            Target::ClearSearch => {
                self.query.clear();
                self.focus = Target::Search;
                self.scroll = 0.;
            }
            Target::Search => {
                self.open_keyboard(TextField::Apps);
            }
            Target::WeatherConfigure => {
                self.open(2);
                self.section = WIDGETS_SECTION;
                self.open_location();
                self.open_keyboard(TextField::Location);
            }
            Target::WeatherToggle => {
                self.preferences.show_weather = !self.preferences.show_weather;
                self.weather.invalidate_forecast();
                return vec![Effect::Save];
            }
            Target::WeatherForecast => {
                self.preferences.show_forecast = !self.preferences.show_forecast;
                return vec![Effect::Save];
            }
            Target::WeatherLocation => self.open_location(),
            Target::WeatherTemperature => {
                self.subpage = Some(Subpage::Temperature);
                self.focus = Target::Choice(usize::from(
                    self.preferences.temperature_unit == Unit::Fahrenheit,
                ));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::WeatherQuery => self.open_keyboard(TextField::Location),
            Target::GreetingToggle => {
                self.preferences.show_greeting = !self.preferences.show_greeting;
                return vec![Effect::Save];
            }
            Target::DateToggle => {
                self.preferences.show_date = !self.preferences.show_date;
                return vec![Effect::Save];
            }
            Target::DateFormat => {
                self.subpage = Some(Subpage::DateFormat);
                self.focus = Target::Choice(self.preferences.date_format.index());
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::GreetingName => self.open_keyboard(TextField::Name),
            Target::GreetingNameClear => {
                self.preferences.greeting_name.clear();
                self.focus = Target::GreetingName;
                return vec![Effect::Save];
            }
            Target::AppNamesHome => {
                self.preferences.show_home_names = !self.preferences.show_home_names;
                return vec![Effect::Save];
            }
            Target::AppNamesApps => {
                self.preferences.show_app_names = !self.preferences.show_app_names;
                return vec![Effect::Save];
            }
            Target::ClockFormat | Target::ClockTimeFormat => {
                let format = self.focus == Target::ClockFormat;
                self.subpage = Some(if format {
                    Subpage::ClockFormat
                } else {
                    Subpage::TimeFormat
                });
                self.focus = Target::Choice(usize::from(if format {
                    self.preferences.hour_24
                } else {
                    self.preferences.show_seconds
                }));
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::ClockSize => {
                self.subpage = Some(Subpage::ClockSize);
                self.focus = Target::Choice(self.preferences.clock_size.index());
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::InputIcon => {
                if let Some(Subpage::Input(id)) = self.subpage.clone() {
                    let selected = self
                        .preferences
                        .input_icon(&id)
                        .and_then(|saved| {
                            crate::model::INPUT_ICONS
                                .iter()
                                .position(|(icon, _)| *icon == saved)
                        })
                        .map_or(0, |index| index + 1);
                    self.subpage = Some(Subpage::InputIcons(id));
                    self.focus = Target::Choice(selected);
                    self.scroll = 0.;
                    if !self.preferences.reduced_motion {
                        self.panel_started = Some(Instant::now());
                    }
                }
            }
            Target::InputRename => self.open_keyboard(TextField::InputName),
            Target::Widget(index) => {
                let (subpage, focus) = match index {
                    0 => (Subpage::Greeting, Target::GreetingToggle),
                    1 => (Subpage::Weather, Target::WeatherToggle),
                    2 => (Subpage::Shortcuts, Target::TileEditor),
                    4 => (Subpage::Date, Target::DateToggle),
                    3 => (
                        Subpage::Inputs,
                        self.inventory
                            .inputs
                            .first()
                            .map(|input| Target::InputMode(input.id.clone()))
                            .unwrap_or(Target::Setting(3)),
                    ),
                    _ => return vec![],
                };
                self.section = WIDGETS_SECTION;
                self.subpage = Some(subpage);
                self.focus = focus;
                self.scroll = 0.;
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
            }
            Target::InstallUpdate => {
                if !self.update_busy()
                    && let Some(version) = &self.available_update
                {
                    let version = version.clone();
                    self.update = crate::update::InstallStatus::Downloading(0);
                    return vec![Effect::UpdateDownload(version)];
                }
            }
            Target::CheckUpdates => {
                if !self.update_busy() {
                    self.update = crate::update::InstallStatus::Idle;
                    self.update_status = crate::update::Status::Checking;
                    self.update_check_requested = true;
                    self.dirty = true;
                }
            }
            Target::Tile(_) => self.options(),
            Target::TileTest(id) => {
                if matches!(&self.overlay, Some(Overlay::Options { id: current, restore: Target::Tile(_), .. }) if current == &id)
                    && let Some(tile) = self.preferences.custom_tile(&id).cloned()
                {
                    return self.send_tile(tile);
                }
            }
            Target::TileRemove(id) => {
                if matches!(&self.overlay, Some(Overlay::Options { id: current, restore: Target::Tile(_), .. }) if current == &id)
                    && let Some(tile) = self.preferences.remove_tile(&id)
                {
                    self.overlay = None;
                    self.notify(
                        self.preferences
                            .language
                            .format("tileRemoved", &[("tile", &tile.title)]),
                    );
                    self.subpage = Some(Subpage::Shortcuts);
                    self.panel_started = None;
                    self.scroll = 0.;
                    self.sync_custom_tiles();
                    self.focus = Target::TileEditor;
                    return vec![Effect::Save];
                }
            }
            Target::InputInMenu => {
                if let Some(Subpage::Input(id)) = &self.subpage {
                    let hidden = &mut self.preferences.inputs_menu_hidden;
                    if let Some(index) = hidden.iter().position(|item| item == id) {
                        hidden.remove(index);
                    } else {
                        hidden.push(id.clone());
                    }
                    return vec![Effect::Save];
                }
            }
            Target::InputNameReset => {
                if let Some(Subpage::Input(id)) = &self.subpage
                    && self.preferences.input_names.remove(id).is_some()
                {
                    self.focus = Target::InputRename;
                    return vec![Effect::Save];
                }
            }
            Target::WeatherSearch => {
                return self
                    .weather
                    .search(self.preferences.language)
                    .map(|command| vec![Effect::Weather(command)])
                    .unwrap_or_default();
            }
            Target::WeatherClear => {
                self.preferences.weather_location = None;
                self.weather.remove_location();
                self.focus = Target::WeatherSearch;
                return vec![Effect::Save, Effect::Weather(weather::Command::ClearCache)];
            }
            Target::WeatherResult(index) => {
                if let Some(place) = self.weather.results.get(index).cloned() {
                    self.preferences.weather_location = Some(place);
                    self.weather.invalidate_forecast();
                    self.weather.cancel_search();
                    self.focus = Target::WeatherSearch;
                    return vec![Effect::Save];
                }
            }
            Target::Sidebar(index) => {
                if self.subpage == Some(Subpage::WeatherLocation) {
                    self.weather.cancel_search();
                }
                self.section = index;
                self.scroll = 0.;
                self.subpage = None;
                self.focus = self.section_entry();
            }
            Target::Setting(index) => {
                match index {
                    0 => {
                        self.subpage = Some(Subpage::AppNames);
                        self.focus = Target::AppNamesHome;
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    1 => {
                        self.subpage = Some(Subpage::Clock);
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        self.focus = Target::ClockFormat;
                        return vec![];
                    }
                    2 => self.preferences.reduced_motion = !self.preferences.reduced_motion,
                    BACK_RECENTS_ROW => {
                        self.preferences.back_recents = !self.preferences.back_recents
                    }
                    LANDING_ROW => {
                        self.preferences.landing_layout = !self.preferences.landing_layout
                    }
                    GLASS_CLOCK_ROW => self.preferences.glass_clock = !self.preferences.glass_clock,
                    TILE_LIFT_ROW => self.preferences.tile_lift = !self.preferences.tile_lift,
                    MORE_APPS_ROW => {
                        self.preferences.more_apps_hint = self.preferences.more_apps_hint.next()
                    }
                    PIXEL_SHIFT_ROW => self.preferences.pixel_shift = !self.preferences.pixel_shift,
                    DAY_NIGHT_ROW => self.preferences.day_night = !self.preferences.day_night,
                    WEATHER_EFFECTS_ROW => {
                        self.preferences.weather_effects = !self.preferences.weather_effects
                    }
                    UI_SOUNDS_ROW => self.preferences.ui_sounds = !self.preferences.ui_sounds,
                    HOLIDAY_REMINDERS_ROW => {
                        self.preferences.holiday_reminders = !self.preferences.holiday_reminders
                    }
                    POWER_ON_ROW => self.preferences.power_on = self.preferences.power_on.next(),
                    HOLIDAYS_ROW => {
                        self.preferences.holiday_mode = self.preferences.holiday_mode.next()
                    }
                    WALLPAPER_TINT_ROW => {
                        self.preferences.wallpaper_tint = self.preferences.wallpaper_tint.next()
                    }
                    UI_SOUND_LEVEL_ROW => {
                        self.preferences.ui_sound_level = self.preferences.ui_sound_level.next()
                    }
                    UI_SOUND_OUTPUT_ROW => {
                        // Default, then each output the TV offers, then back to Default.
                        let outputs = &self.ui_sound_outputs;
                        let next = match &self.preferences.ui_sound_output {
                            None => outputs.first().cloned(),
                            Some(current) => outputs
                                .iter()
                                .position(|output| output == current)
                                .and_then(|index| outputs.get(index + 1))
                                .cloned(),
                        };
                        self.preferences.ui_sound_output = next;
                        self.ui_sounds_detail = None;
                    }
                    APP_SORTING_ROW => {
                        self.preferences.app_sorting = self.preferences.app_sorting.next()
                    }
                    DRAWER_TOOLBAR_ROW => {
                        self.preferences.drawer_toolbar = !self.preferences.drawer_toolbar;
                        if !self.preferences.drawer_toolbar {
                            // Nothing on screen could clear these any more.
                            self.query.clear();
                            if self.filter != 5 {
                                self.filter = 0;
                            }
                        }
                    }
                    QUICK_PANEL_ROW => {
                        self.subpage = Some(Subpage::QuickPanel);
                        self.focus = Target::QuickToggle(crate::model::QuickKey::ALL[0]);
                        self.scroll = 0.;
                        return vec![];
                    }
                    AMBIENT_ROW => {
                        let current = AMBIENT_CHOICES
                            .iter()
                            .position(|&m| m == self.preferences.ambient_minutes)
                            .unwrap_or(0);
                        self.preferences.ambient_minutes =
                            AMBIENT_CHOICES[(current + 1) % AMBIENT_CHOICES.len()];
                    }
                    4 => {
                        self.subpage = Some(Subpage::Language);
                        self.focus = Target::Choice(self.preferences.language.index());
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    5 => return vec![Effect::Quit],
                    9 => {
                        self.reset_icon_cache = true;
                        return vec![];
                    }
                    7 => {
                        self.subpage = Some(Subpage::TextRendering);
                        self.focus = Target::Choice(self.preferences.text_rendering.index());
                        self.scroll = 0.;
                        if !self.preferences.reduced_motion {
                            self.panel_started = Some(Instant::now());
                        }
                        return vec![];
                    }
                    _ => return vec![Effect::Reload],
                }
                return vec![Effect::Save];
            }
            Target::Background(_) => {
                self.options();
            }
            Target::WallpaperCycle => {
                (
                    self.preferences.wallpaper_cycle,
                    self.preferences.cycle_interval,
                ) = crate::wallpaper_cycle::Interval::step(
                    self.preferences.wallpaper_cycle,
                    self.preferences.cycle_interval,
                );
                self.preferences.wallpaper_changed = unix_now();
                return vec![Effect::Save];
            }
            Target::WallpaperGradient => {
                use crate::model::GradientStrength::{Light, Medium, Strong};
                let preferences = &mut self.preferences;
                (
                    preferences.wallpaper_gradient,
                    preferences.gradient_strength,
                ) = match (
                    preferences.wallpaper_gradient,
                    preferences.gradient_strength,
                ) {
                    (false, _) => (true, Light),
                    (true, Light) => (true, Medium),
                    (true, Medium) => (true, Strong),
                    (true, Strong) => (false, Strong),
                };
                return vec![Effect::Save];
            }
            Target::UseBackground => {
                let Some(Overlay::BackgroundOptions { id, .. }) = self.overlay.clone() else {
                    return vec![];
                };
                if self.deleting_wallpaper.as_deref() == Some(&id) {
                    return vec![];
                }
                if !self.background_ids().any(|known| known == id) {
                    return self.back();
                }
                if !self.preferences.reduced_motion && self.preferences.background != id {
                    self.previous_background =
                        Some((self.preferences.background.clone(), Instant::now()));
                    self.background_fade = BACKGROUND_FADE;
                }
                self.preferences.splash_background = Some(if crate::wallpapers::valid_id(&id) {
                    self.preferences.splash_background().to_owned()
                } else {
                    id.clone()
                });
                self.preferences.background = id;
                self.preferences.wallpaper_changed = unix_now();
                self.back();
                return vec![Effect::Save];
            }
            Target::RemoveCollection => {
                if let Some(id) = self.removable_collection().map(str::to_owned) {
                    self.preferences
                        .created_collections
                        .retain(|known| *known != id);
                    self.preferences.collection_schedules.remove(&id);
                    if self
                        .preferences
                        .collection_pick
                        .as_ref()
                        .is_some_and(|pick| pick.id == id)
                    {
                        self.preferences.collection_pick = None;
                    }
                    self.gallery_collection = None;
                    self.focus = Target::WallpaperCollection;
                    self.dirty = true;
                    return vec![Effect::Save];
                }
            }
            Target::DiscoverOpen => {
                let collection = self.gallery_collection().to_owned();
                self.subpage = Some(Subpage::Discover);
                self.scroll = 0.;
                self.dirty = true;
                if self.discover.collection == collection && !self.discover.photos.is_empty() {
                    self.focus = Target::Suggestion(0);
                    return vec![];
                }
                let query = match collection.as_str() {
                    crate::collections::PHOTOS => "landscape".to_string(),
                    other => crate::collections::name(other),
                };
                self.discover = Discover {
                    collection,
                    query: query.clone(),
                    page: 1,
                    loading: true,
                    ..Discover::default()
                };
                self.focus = Target::MoreSuggestions;
                return vec![Effect::Suggest { query, page: 1 }];
            }
            Target::Suggestion(index) => {
                // Already saved here: say where instead of downloading it twice.
                if let Some(owner) = self
                    .discover
                    .photos
                    .get(index)
                    .and_then(|item| self.suggestion_owner(&item.suggestion.photo.id))
                {
                    self.notify(format!("Already in {owner}"));
                    return vec![];
                }
                if let Some(item) = self.discover.photos.get_mut(index)
                    && matches!(
                        item.status,
                        SuggestionStatus::Ready | SuggestionStatus::Failed
                    )
                {
                    item.status = SuggestionStatus::Adding;
                    self.dirty = true;
                    return vec![Effect::AddSuggestion {
                        id: item.suggestion.photo.id.clone(),
                        collection: self.discover.collection.clone(),
                    }];
                }
            }
            Target::MoreSuggestions => {
                if !self.discover.loading && !self.discover.exhausted {
                    self.discover.loading = true;
                    self.discover.page += 1;
                    self.dirty = true;
                    return vec![Effect::Suggest {
                        query: self.discover.query.clone(),
                        page: self.discover.page,
                    }];
                }
            }
            Target::WallpaperCollection => {
                let current = self.gallery_collection().to_owned();
                let index = self
                    .collections()
                    .iter()
                    .position(|(id, _)| *id == current)
                    .unwrap_or(0);
                self.subpage = Some(Subpage::Collections);
                self.focus = Target::Choice(index);
                self.scroll = 0.;
                self.dirty = true;
            }
            Target::MoveBackground => {
                if let Some(Overlay::BackgroundOptions { id, .. }) = self.overlay.clone() {
                    let current = self.wallpaper_collection(&id).to_owned();
                    let index = self
                        .collections()
                        .iter()
                        .position(|(collection, _)| *collection == current)
                        .unwrap_or(0);
                    self.overlay = None;
                    self.panel_started = None;
                    self.subpage = Some(Subpage::MoveCollection(id));
                    self.focus = Target::Choice(index);
                    self.scroll = 0.;
                    self.dirty = true;
                }
            }
            Target::NightBackground => {
                if let Some(Overlay::BackgroundOptions { id, .. }) = &self.overlay {
                    let night = &mut self.preferences.night_wallpapers;
                    if let Some(index) = night.iter().position(|item| item == id) {
                        night.remove(index);
                    } else {
                        night.push(id.clone());
                    }
                    self.dirty = true;
                    return vec![Effect::Save];
                }
            }
            Target::DeleteBackground => {
                if let Some(Overlay::BackgroundOptions { id, restore }) = self.overlay.clone()
                    && self.can_delete_background(&id)
                    && self.deleting_wallpaper.is_none()
                {
                    self.overlay = Some(Overlay::BackgroundDelete { id, restore });
                    self.focus = Target::CancelDelete;
                    self.pointer = false;
                    self.pressed = None;
                    self.panel_started = None;
                }
            }
            Target::InputMode(id) => {
                let mode = self
                    .preferences
                    .input_modes
                    .get(&id)
                    .copied()
                    .unwrap_or_default();
                self.subpage = Some(Subpage::Input(id));
                if !self.preferences.reduced_motion {
                    self.panel_started = Some(Instant::now());
                }
                self.focus = Target::Choice(match mode {
                    InputMode::On => 0,
                    InputMode::Off => 1,
                    InputMode::Auto => 2,
                });
            }
            Target::Choice(index) => {
                match self.subpage.clone() {
                    Some(Subpage::Collections) => {
                        let collections = self.collections();
                        if let Some((id, _)) = collections.get(index) {
                            self.preferences.collection_pick = Some(crate::collections::Pick {
                                id: id.clone(),
                                at: unix_now() as i64,
                            });
                            self.gallery_collection = Some(id.clone());
                            self.close_subpage();
                        } else if index == collections.len() + self.custom_collections().len() {
                            self.open_keyboard(TextField::CollectionName);
                            return vec![];
                        } else if let Some(id) =
                            self.custom_collections().get(index - collections.len())
                        {
                            let schedules = &mut self.preferences.collection_schedules;
                            match crate::collections::Schedule::next(schedules.get(id).copied()) {
                                Some(next) => schedules.insert(id.clone(), next),
                                None => schedules.remove(id),
                            };
                            self.dirty = true;
                        }
                        return vec![Effect::Save];
                    }
                    Some(Subpage::MoveCollection(wallpaper)) => {
                        let Some((collection, _)) = self.collections().into_iter().nth(index)
                        else {
                            return vec![];
                        };
                        let uploaded = self
                            .wallpapers
                            .iter()
                            .find(|item| item.id == wallpaper)
                            .and_then(|item| item.collection.clone())
                            .unwrap_or_else(|| crate::collections::PHOTOS.into());
                        if collection == uploaded {
                            self.preferences.wallpaper_collections.remove(&wallpaper);
                        } else {
                            self.preferences
                                .wallpaper_collections
                                .insert(wallpaper, collection);
                        }
                        self.close_subpage();
                        return vec![Effect::Save];
                    }
                    Some(Subpage::ClockFormat) => self.preferences.hour_24 = index == 1,
                    Some(Subpage::TimeFormat) => self.preferences.show_seconds = index == 1,
                    Some(Subpage::ClockSize) => {
                        self.preferences.clock_size = crate::model::ClockSize::ALL[index.min(3)];
                    }
                    Some(Subpage::DateFormat) => {
                        self.preferences.date_format = crate::model::DateFormat::ALL[index.min(3)];
                    }
                    Some(Subpage::InputIcons(id)) => {
                        let icon = if index == 0 {
                            ""
                        } else if let Some((icon, _)) = crate::model::INPUT_ICONS.get(index - 1) {
                            *icon
                        } else {
                            return vec![];
                        };
                        let changed = self.preferences.set_input_icon(&id, icon);
                        self.close_subpage();
                        return if changed { vec![Effect::Save] } else { vec![] };
                    }
                    Some(Subpage::Input(id)) => {
                        self.preferences.input_modes.insert(
                            id,
                            [InputMode::On, InputMode::Off, InputMode::Auto][index.min(2)],
                        );
                    }
                    Some(Subpage::Temperature) => {
                        self.preferences.temperature_unit = if index == 1 {
                            Unit::Fahrenheit
                        } else {
                            Unit::Celsius
                        }
                    }
                    Some(Subpage::Language) => {
                        self.preferences.language =
                            Language::ALL[index.min(Language::ALL.len() - 1)];
                        self.weather.cancel_search();
                    }
                    Some(Subpage::TextRendering) => {
                        let mode = TextRendering::ALL[index.min(TextRendering::ALL.len() - 1)];
                        if mode != self.preferences.text_rendering {
                            self.preferences.text_rendering = mode;
                            self.notify(self.preferences.language.t("textRenderingRestart"));
                        }
                    }
                    Some(
                        Subpage::WeatherLocation
                        | Subpage::QuickPanel
                        | Subpage::Greeting
                        | Subpage::Date
                        | Subpage::AppNames
                        | Subpage::Clock
                        | Subpage::Weather
                        | Subpage::Shortcuts
                        | Subpage::Inputs
                        | Subpage::Discover,
                    ) => {
                        return vec![];
                    }
                    None => return vec![],
                }
                self.close_subpage();
                return vec![Effect::Save];
            }
            Target::Option(index) => {
                if index == 2 {
                    if let Some(Overlay::Options { id, restore }) = &self.overlay
                        && crate::platform::can_uninstall(id)
                        && self.uninstalling.is_none()
                        && self.inventory.apps.iter().any(|app| &app.id == id)
                    {
                        self.overlay = Some(Overlay::Delete {
                            id: id.clone(),
                            restore: restore.clone(),
                        });
                        self.focus = Target::CancelDelete;
                        self.pointer = false;
                        self.pressed = None;
                        self.panel_started = None;
                    }
                    return vec![];
                }
                let Some(Overlay::Options { id, restore }) = self.overlay.take() else {
                    return vec![];
                };
                self.panel_started = None;
                let old_ids: Vec<_> = self.apps().iter().map(|a| a.id.clone()).collect();
                let old_index = old_ids.iter().position(|item| item == &id).unwrap_or(0);
                let mut favorites = self.preferences.favorites(&self.inventory.apps);
                let title = self
                    .inventory
                    .apps
                    .iter()
                    .find(|app| app.id == id)
                    .map_or(id.clone(), |app| app.title.clone());
                if index == 0 {
                    if favorites.contains(&id) {
                        favorites.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteRemoved", &[("app", &title)]),
                        );
                    } else {
                        favorites.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("favoriteAdded", &[("app", &title)]),
                        );
                    }
                    self.preferences.hidden.retain(|item| item != &id);
                    self.preferences.favorites = Some(favorites);
                } else if index == 1 {
                    if self.preferences.hidden.contains(&id) {
                        self.preferences.hidden.retain(|item| item != &id);
                        self.notify(
                            self.preferences
                                .language
                                .format("appRestored", &[("app", &title)]),
                        );
                    } else {
                        self.preferences.hidden.push(id.clone());
                        self.notify(
                            self.preferences
                                .language
                                .format("appHidden", &[("app", &title)]),
                        );
                    }
                }
                if self.filter == 5 && self.hidden_count() == 0 {
                    self.filter = 0;
                }
                self.focus = restore;
                self.pointer = false;
                if let Target::App(id) = &self.focus {
                    let apps = self.apps();
                    if !apps.iter().any(|app| &app.id == id) {
                        self.focus = apps
                            .get(old_index.min(apps.len().saturating_sub(1)))
                            .map(|app| Target::App(app.id.clone()))
                            .unwrap_or_else(|| self.first_content());
                    }
                }
                return if index < 2 {
                    vec![Effect::Save]
                } else {
                    vec![]
                };
            }
            Target::CancelDelete => return self.back(),
            Target::ConfirmDelete => {
                if let Some(Overlay::BackgroundDelete { id, .. }) = self.overlay.clone() {
                    if self.can_delete_background(&id) && self.deleting_wallpaper.is_none() {
                        if crate::wallpapers::valid_id(&id) {
                            self.deleting_wallpaper = Some(id.clone());
                            self.focus = Target::CancelDelete;
                            return vec![Effect::DeleteWallpaper(id)];
                        }
                        // Packaged assets remain intact; persist removal from the gallery.
                        self.finish_background_delete(&id);
                        return vec![Effect::Save];
                    }
                    return vec![];
                }
                if let Some(Overlay::Delete { id, .. }) = &self.overlay
                    && self.uninstalling.is_none()
                    && crate::platform::can_uninstall(id)
                    && self.inventory.apps.iter().any(|app| &app.id == id)
                {
                    self.uninstalling = Some(id.clone());
                    self.focus = Target::CancelDelete;
                    return vec![Effect::Uninstall(id.clone())];
                }
            }
            Target::Key(index) => match KEYS[index] {
                "Space" => self.text(" "),
                "Delete" => self.delete(),
                "Clear" => match self.keyboard_field {
                    TextField::Location => self.weather.edit(String::new()),
                    TextField::Name => self.greeting_name_draft.clear(),
                    TextField::InputName => self.input_name.clear(),
                    TextField::CollectionName => self.collection_name.clear(),
                    TextField::Apps => self.query.clear(),
                },
                "Done" => {
                    self.overlay = None;
                    self.panel_started = None;
                    self.focus = self.keyboard_return();
                    return self.commit_text_input();
                }
                key => self.text(key),
            },
            _ => {}
        }
        vec![]
    }

    pub fn back(&mut self) -> Vec<Effect> {
        self.pointer = false;
        self.dirty = true;
        if self.moving.take().is_some() {
            return vec![];
        }
        if let Some(overlay) = self.overlay.take() {
            self.panel_started = None;
            if let Overlay::BackgroundDelete { id, restore } = overlay {
                self.overlay = Some(Overlay::BackgroundOptions { id, restore });
                self.focus = Target::DeleteBackground;
                self.pressed = None;
                return vec![];
            }
            if let Overlay::BackgroundOptions { id, .. } = overlay {
                self.focus = self.background_target(&id);
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::WallpaperUpload) {
                self.wallpaper_upload = None;
                self.focus = Target::UploadWallpaper;
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::TileEditor) {
                self.tile_editor = None;
                self.focus = Target::TileEditor;
                self.pressed = None;
                return vec![];
            }
            if let Overlay::Delete { id, restore } = overlay {
                self.overlay = Some(Overlay::Options { id, restore });
                self.focus = Target::Option(2);
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::Recents { .. }) {
                self.recents.closing_focus = Some(self.focus.clone());
                self.recents.updated = Instant::now();
                self.pressed = None;
                if self.preferences.reduced_motion {
                    self.recents.visibility = 0.;
                    self.recents.closing_focus = None;
                }
            }
            if matches!(overlay, Overlay::Inputs) {
                // Back from the Inputs list returns to the quick panel it came from.
                self.overlay = Some(Overlay::Quick);
                self.focus = Target::Quick(crate::model::QuickKey::Inputs);
                self.pressed = None;
                return vec![];
            }
            if matches!(overlay, Overlay::Quick) {
                self.close_quick();
                return vec![];
            }
            if let Overlay::Options { restore, .. } | Overlay::Recents { restore } = overlay {
                self.focus = restore;
                self.recents.pending = None;
                self.reconcile_focus();
                return vec![];
            }
            self.focus = self.keyboard_return();
            return self.commit_text_input();
        }
        if self.page == Page::Settings && self.subpage.is_some() {
            self.close_subpage();
            return vec![];
        }
        if self.page == Page::Settings && !matches!(self.focus, Target::Nav(_)) {
            if matches!(self.focus, Target::Sidebar(_)) {
                self.focus = Target::Nav(2);
            } else {
                self.focus = Target::Sidebar(self.section);
            }
            self.scroll = 0.;
            return vec![];
        }
        if self.page == Page::Apps
            && matches!(self.focus, Target::Search | Target::ClearSearch)
            && !self.query.is_empty()
        {
            self.query.clear();
            self.focus = Target::Search;
            self.scroll = 0.;
            return vec![];
        }
        if self.page == Page::Apps {
            self.close_drawer()
        } else if self.page != Page::Home {
            self.return_home()
        } else if self.preferences.back_recents {
            self.open_recents()
        } else {
            // With the Recent apps shortcut off, Back on Home returns to the first favourite,
            // scrolling the strip back to its beginning.
            self.home_scroll = 0.;
            self.focus = self.first_content();
            vec![]
        }
    }

    /// The Back-button shortcut to Recent apps (Back on Home, or held anywhere), unless the owner
    /// turned it off in Settings → General.
    pub fn back_recents(&mut self) -> Vec<Effect> {
        if self.preferences.back_recents {
            self.open_recents()
        } else {
            vec![]
        }
    }
}

/// Smooth start and finish for a 0–1 progress.
pub(crate) fn ease(t: f32) -> f32 {
    t * t * (3. - 2. * t)
}

/// Linear blend of two 0xRRGGBB colours.
fn mix(from: u32, to: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let a = ((from >> shift) & 0xff) as f32;
        let b = ((to >> shift) & 0xff) as f32;
        ((a + (b - a) * t.clamp(0., 1.)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// The TV's Discover page: Unsplash suggestions for one collection.
#[derive(Clone, Debug, Default)]
pub struct Discover {
    pub collection: String,
    pub query: String,
    pub page: u32,
    pub photos: Vec<SuggestionItem>,
    pub loading: bool,
    /// The last page came back short, so there is nothing more to load.
    pub exhausted: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SuggestionItem {
    pub suggestion: crate::wallpapers::Suggestion,
    pub status: SuggestionStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuggestionStatus {
    Ready,
    Adding,
    Added,
    Failed,
}
