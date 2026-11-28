use crate::{
    model::{App, Input, InputMode, Preferences, demo_inventory},
    platform::{normalize_apps, normalize_inputs},
    state::{Direction, Effect, Overlay, Page, State, Target},
    storage, view,
};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf};

#[path = "recents-tests.rs"]
mod recent_tests;

#[path = "wallpapers-ui-tests.rs"]
mod wallpaper_tests;

#[path = "input-refresh-tests.rs"]
mod input_refresh_tests;

#[path = "input-icon-tests.rs"]
mod input_icon_tests;

fn ready() -> State {
    let mut state = State::new(Preferences::default());
    // Most fixtures start from the tab bar, as if a key had been pressed before apps loaded.
    state.note_input();
    state.set_inventory(demo_inventory(), None);
    state
}

#[test]
fn home_relaunch_returns_from_apps_and_settings() {
    for page in [1, 2] {
        let mut state = ready();
        state.open(page);
        state.query = "YouTube".into();
        state.filter = 1;
        state.scroll = 400.;
        state.subpage = Some(crate::state::Subpage::Clock);
        state.pointer = true;
        state.pressed = Some(Target::Nav(page));
        state.dirty = false;

        assert!(state.return_home().is_empty());
        assert_eq!(state.page, Page::Home);
        assert_eq!(state.focus, Target::App("netflix".into()));
        assert!(state.query.is_empty());
        assert_eq!(state.filter, 0);
        assert_eq!(state.scroll, 0.);
        assert!(state.subpage.is_none());
        assert!(!state.pointer);
        assert!(state.pressed.is_none());
        assert!(state.dirty);
        assert!(state.return_home().is_empty());
        assert!(state.overlay.is_none(), "Home again must not open Recents");
    }
}

#[test]
fn home_relaunch_dismisses_overlays_without_cancelling_mutations() {
    for overlay in [
        Overlay::Options {
            id: "test.app".into(),
            restore: Target::Nav(1),
        },
        Overlay::Delete {
            id: "test.app".into(),
            restore: Target::Nav(1),
        },
        Overlay::WallpaperUpload,
        Overlay::BackgroundOptions {
            id: "dunes".into(),
            restore: Target::Sidebar(1),
        },
        Overlay::BackgroundDelete {
            id: "dunes".into(),
            restore: Target::Sidebar(1),
        },
    ] {
        let mut state = ready();
        state.open(2);
        state.overlay = Some(overlay);
        state.panel_started = Some(std::time::Instant::now());
        state.wallpaper_upload = Some(Err("test".into()));
        state.uninstalling = Some("test.app".into());
        state.deleting_wallpaper = Some("dunes".into());
        state.moving = Some("test.app".into());

        assert!(state.return_home().is_empty());
        assert_eq!(state.page, Page::Home);
        assert!(state.overlay.is_none());
        assert!(state.panel_started.is_none());
        assert!(state.wallpaper_upload.is_none());
        assert!(state.moving.is_none());
        assert_eq!(state.uninstalling.as_deref(), Some("test.app"));
        assert_eq!(state.deleting_wallpaper.as_deref(), Some("dunes"));
    }
}

#[test]
fn home_relaunch_commits_keyboard_edits_before_clearing_the_subpage() {
    for system in [false, true] {
        let mut state = ready();
        state.open(2);
        state.subpage = Some(crate::state::Subpage::Input("HDMI_1".into()));
        state.open_keyboard(crate::state::TextField::InputName);
        if system {
            state.overlay = Some(Overlay::SystemKeyboard);
        }
        state.text("Console");

        assert_eq!(state.return_home(), vec![Effect::Save]);
        assert_eq!(
            state
                .preferences
                .input_names
                .get("HDMI_1")
                .map(String::as_str),
            Some("Console")
        );
        assert_eq!(state.page, Page::Home);
        assert_eq!(state.focus, Target::App("netflix".into()));
        assert!(!state.text_input_active());
        assert!(state.subpage.is_none());
    }
}

#[test]
fn home_relaunch_dismisses_recents_and_ignores_its_pending_reply() {
    let mut state = ready();
    state.open(1);
    state.open_recents();
    state.recents.visibility = 1.;
    state.recents.closing_focus = Some(Target::RecentEmpty);
    assert!(state.recents.pending.is_some());
    let generation = state.recents.generation;
    let command = state.recents.pending.clone().unwrap();

    assert!(state.return_home().is_empty());
    assert_eq!(state.page, Page::Home);
    assert!(state.overlay.is_none());
    assert!(state.recents.pending.is_none());
    assert_eq!(state.recents.visibility, 0.);
    assert!(state.recents.closing_focus.is_none());
    assert!(
        state
            .recent_response(generation, command, Ok(Some(vec![])))
            .is_empty()
    );
    assert_eq!(state.focus, Target::App("netflix".into()));
    assert!(state.overlay.is_none());
}

#[test]
fn home_relaunch_focuses_first_visible_favorite_in_saved_order() {
    let mut state = ready();
    state.preferences.favorites = Some(vec![
        "missing.app".into(),
        "netflix".into(),
        "youtube.leanback.v4".into(),
    ]);
    state.preferences.hidden.push("netflix".into());
    state.home_scroll = crate::state::HOME_TILE_PITCH;
    state.focus = Target::Input("HDMI_1".into());

    state.return_home();
    assert_eq!(state.focus, Target::App("youtube.leanback.v4".into()));
    assert_eq!(state.home_scroll, 0.);
    assert_eq!(
        state.activate(),
        vec![Effect::Launch("youtube.leanback.v4".into())]
    );
}

#[test]
fn home_relaunch_resets_scrolled_favorites_and_keeps_remote_routes() {
    let mut state = ready();
    state.preferences.favorites = Some(
        state
            .inventory
            .apps
            .iter()
            .take(9)
            .map(|app| app.id.clone())
            .collect(),
    );
    let favorites = state.preferences.favorites.clone().unwrap();
    state.home_scroll = crate::state::HOME_TILE_PITCH * 2.;

    state.return_home();
    assert_eq!(state.home_scroll, 0.);
    assert_eq!(state.focus, Target::App(favorites[0].clone()));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::App(favorites[1].clone()));
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::App(favorites[0].clone()));
    state.direction(Direction::Down, &controls(&state));
    assert!(matches!(state.focus, Target::Input(_)));

    state.return_home();
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Nav(2), "Up reaches the gear");
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(
        state.page,
        Page::Home,
        "No tabs: Left stays on Home's buttons"
    );
    assert_eq!(
        state.focus,
        Target::Nav(2),
        "the gear is the only top button"
    );
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::App(favorites[0].clone()));
}

#[test]
fn home_relaunch_without_favorites_uses_existing_empty_and_error_actions() {
    let mut state = ready();
    state.preferences.favorites = Some(vec![]);
    state.return_home();
    assert_eq!(state.focus, Target::Browse);

    state.error = Some("Inventory unavailable".into());
    state.return_home();
    assert_eq!(state.focus, Target::Retry);
}

#[test]
fn back_to_home_focuses_first_favorite() {
    for page in [1, 2] {
        let mut state = ready();
        state.open(page);
        state.home_scroll = crate::state::HOME_TILE_PITCH;
        assert!(state.back().is_empty());
        assert_eq!(state.page, Page::Home);
        assert_eq!(state.focus, Target::App("netflix".into()));
        assert_eq!(state.home_scroll, 0.);
    }
}

fn scene(state: &State) -> view::Scene {
    let assets = view::Assets {
        root: PathBuf::new(),
        version: "test".into(),
        metrics: BTreeMap::new(),
        artwork: crate::app_artwork::Service::new(false),
    };
    let navigation = view::navigation(&assets, state.preferences.language);
    view::scene_with_pill(
        state,
        &assets,
        navigation.items[state.page_index()].x,
        navigation.items[state.page_index()].w,
        time::OffsetDateTime::UNIX_EPOCH,
    )
}

fn controls(state: &State) -> Vec<crate::state::Control> {
    scene(state).controls
}

#[test]
fn favorites_edge_fragments_share_draw_hit_and_wheel_viewports() {
    let mut state = ready();
    let favorites: Vec<_> = state
        .inventory
        .apps
        .iter()
        .take(9)
        .map(|app| app.id.clone())
        .collect();
    state.preferences.favorites = Some(favorites.clone());
    assert_eq!(
        state.hit_test(&controls(&state), 1860., 600.),
        Some(Target::App(favorites[6].clone()))
    );
    assert!(state.wheel_scroll(&controls(&state), Some((1860., 600.)), 0, -1));
    assert!((state.home_scroll - crate::state::HOME_TILE_PITCH).abs() < 0.01);
    assert_eq!(state.scroll, 0.);
    assert_eq!(
        state.hit_test(&controls(&state), 40., 600.),
        Some(Target::App(favorites[0].clone()))
    );
    let scroll = state.home_scroll;
    state.pointer_focus(Target::App(favorites[0].clone()));
    assert_eq!(state.home_scroll, scroll);
    assert!(state.hit_test(&controls(&state), 4., 600.).is_none());
    assert!(state.hit_test(&controls(&state), 1915., 600.).is_none());
    assert!(state.hit_test(&controls(&state), 40., 1060.).is_none());
    let scene = scene(&state);
    let viewport = state.favorites_viewport();
    assert!(scene.draw.iter().any(|draw| matches!(draw, crate::renderer::Draw::Clip(Some(rect)) if rect.x == viewport.x && rect.w == viewport.w)));
    assert!(state.wheel_scroll(&scene.controls, Some((40., 600.)), 0, 1));
    assert_eq!(state.home_scroll, 0.);
    assert_eq!(state.scroll, 0.);
}

#[test]
fn widening_favorites_does_not_widen_apps_or_unclip_offscreen_rows() {
    let mut state = ready();
    state.open(1);
    let control = crate::state::Control {
        target: Target::App("sample".into()),
        rect: crate::state::Rect::new(0., 500., 1920., 600.),
        scrollable: true,
    };
    let controls = [control];
    assert!(state.hit_test(&controls, 40., 600.).is_none());
    assert!(state.hit_test(&controls, 1860., 600.).is_none());
    assert_eq!(
        state.hit_test(&controls, 100., 1000.),
        Some(Target::App("sample".into()))
    );
    assert!(state.hit_test(&controls, 100., 1080.).is_none());
    assert_eq!(
        state.hit_test(&controls, 100., 600.),
        Some(Target::App("sample".into()))
    );
    assert!(!state.wheel_scroll(&controls, Some((1860., 600.)), 0, -1));
    assert_eq!(state.home_scroll, 0.);
    let viewport = state.content_viewport();
    let scroll_limit = controls[0].rect.y + controls[0].rect.h - (viewport.y + viewport.h - 12.);
    assert_eq!(state.scroll, 90_f32.min(scroll_limit));
}

#[test]
fn remote_routes_from_header_and_back_through_settings() {
    let mut state = ready();
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::App("netflix".into()));
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Nav(2));
    state.open(2);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(0));
    state.activate();
    assert_eq!(state.focus, Target::Setting(0));
    assert!(state.back().is_empty());
    assert_eq!(state.focus, Target::Sidebar(0));
    state.back();
    assert_eq!(state.focus, Target::Nav(2));
    state.back();
    assert_eq!(state.page, Page::Home);
    assert_eq!(
        state.back(),
        vec![Effect::Recent {
            generation: 1,
            command: crate::recents::Command::List
        }]
    );
    assert!(state.recents_open());
}

#[test]
fn hiding_restores_nearest_app_and_hidden_filter_can_restore_it() {
    let mut state = ready();
    state.open(1);
    state.focus = Target::App("netflix".into());
    state.options();
    state.focus = Target::Option(1);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.hidden.contains(&"netflix".into()));
    assert_eq!(state.focus, Target::App("youtube.leanback.v4".into()));
    state.filter = 5;
    assert_eq!(state.apps().len(), 1);
    state.focus = Target::App("netflix".into());
    state.options();
    state.focus = Target::Option(1);
    state.activate();
    assert!(state.preferences.hidden.is_empty());
    assert_eq!(state.focus, Target::App("netflix".into()));
    assert_eq!(state.filter, 0);
}

#[test]
fn reorder_persists_order_and_keeps_selected_identity() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.hold();
    assert_eq!(
        state.direction(Direction::Right, &controls(&state)),
        vec![Effect::Save]
    );
    assert_eq!(
        &state.preferences.favorites(&state.inventory.apps)[..2],
        &["youtube.leanback.v4", "netflix"]
    );
    assert_eq!(state.focus, Target::App("netflix".into()));
    assert!(state.back().is_empty());
    assert!(state.moving.is_none());
}

#[test]
fn keyboard_is_modal_and_back_preserves_then_clears_search() {
    let mut state = ready();
    state.open(1);
    state.focus = Target::Search;
    state.activate();
    assert!(
        controls(&state)
            .iter()
            .all(|c| matches!(c.target, Target::Key(_) | Target::CloseDialog))
    );
    state.text("netflix");
    state.delete();
    state.text("x");
    state.back();
    assert_eq!(state.query, "netflix");
    assert_eq!(state.focus, Target::Search);
    assert_eq!(state.apps().len(), 1);
    state.back();
    assert!(state.query.is_empty());
    assert_eq!(state.focus, Target::Search);
    assert_eq!(state.page, Page::Apps);
    // With the search cleared, Back closes the app drawer.
    state.back();
    assert_eq!(state.page, Page::Home);
}

#[test]
fn partial_refresh_preserves_valid_inputs_and_never_invents_tv_apps() {
    let mut state = ready();
    state.set_inventory(
        crate::model::Inventory::default(),
        Some("Inputs unavailable".into()),
    );
    assert!(state.inventory.apps.is_empty());
    assert_eq!(state.inventory.inputs.len(), 3);
    assert_eq!(state.first_content(), Target::Browse);
}

#[test]
fn inventory_filters_system_apps_duplicates_and_unsafe_identifiers() {
    let apps = normalize_apps(&json!([
        {"id":"netflix","title":" Netflix\n"}, {"id":"netflix","title":"Duplicate"},
        {"id":"bad;command","title":"Bad"}, {"id":"com.webos.app.hidden","title":"Internal"},
        {"id":"com.webos.app.livetv","visible":false,"title":"Live TV"},
        {"id":"com.luma.launcher.native","visible":true},
        {"id":"com.webos.app.hdmi1","visible":true},
        {"id":"hidden","visible":"false"}
    ]))
    .unwrap();
    assert_eq!(apps.len(), 2);
    assert_eq!(
        apps.iter().find(|a| a.id == "netflix").unwrap().title,
        "Netflix"
    );
    assert!(normalize_apps(&json!({"apps":[]})).is_err());
    let inputs = normalize_inputs(&json!([
        {"appId":"com.webos.app.hdmi1","id":"HDMI_1","port":1,"connected":true},
        {"appId":"com.webos.app.hdmi2","id":"HDMI_4"},
        {"appId":"com.webos.app.hdmi3"}, {"appId":"com.webos.app.hdmi3"},
        {"appId":"com.webos.app.hdmi4","label":"Console"}
    ]))
    .unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[1].connected, None);
}

#[test]
fn inventory_visibility_cases_match_the_launcher_filters() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../tests/fixtures/app-visibility.json")).unwrap();
    let inventory: Vec<_> = cases.iter().map(|case| case["app"].clone()).collect();
    let actual: std::collections::BTreeSet<_> = normalize_apps(&json!(inventory))
        .unwrap()
        .into_iter()
        .map(|app| app.id)
        .collect();
    let expected: std::collections::BTreeSet<_> = cases
        .iter()
        .filter(|case| case["include"] == true)
        .map(|case| case["app"]["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn asset_loading_and_unknown_app_cards_do_not_require_collected_artwork() {
    let root = std::env::temp_dir().join(format!("luma-assets-no-dumps-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("version.txt"), "test").unwrap();
    std::fs::write(root.join("font-metrics.json"), "{}").unwrap();
    for webos in [false, true] {
        let assets = view::Assets::load(root.clone(), webos).unwrap();
        let mut state = State::new(Preferences::default());
        state.set_inventory(
            crate::model::Inventory {
                apps: vec![App::new("com.example.regional", "Regional App")],
                inputs: vec![],
            },
            None,
        );
        state.open(1);
        let scene = view::scene_with_pill(
            &state,
            &assets,
            800.,
            100.,
            time::OffsetDateTime::UNIX_EPOCH,
        );
        assert!(scene.draw.iter().any(|draw| matches!(draw,
            crate::renderer::Draw::TintedImage(path, ..) if path.ends_with("ui-icons/generic.lvec"))));
        assert!(scene.draw.iter().any(|draw| matches!(draw,
            crate::renderer::Draw::Text(value, ..) if value == "Regional App")));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn auto_input_visibility_requires_a_positive_connection_report() {
    let mut preferences = Preferences::default();
    let mut input = Input {
        id: "com.webos.app.hdmi1".into(),
        title: "Console".into(),
        port: "HDMI 1".into(),
        connected: None,
        signal: None,
    };
    assert!(!preferences.input_visible(&input));
    input.connected = Some(true);
    assert!(preferences.input_visible(&input));
    preferences
        .input_modes
        .insert(input.id.clone(), InputMode::Off);
    assert!(!preferences.input_visible(&input));
    preferences
        .input_modes
        .insert(input.id.clone(), InputMode::On);
    input.connected = Some(false);
    assert!(preferences.input_visible(&input));
}

#[test]
fn preferences_round_trip_and_corrupt_data_is_not_silently_overwritten() {
    let directory = std::env::temp_dir().join(format!("luma-native-test-{}", std::process::id()));
    let path = directory.join("preferences.json");
    let mut preferences = Preferences {
        favorites: Some(vec![]),
        background: "forest".into(),
        ..Preferences::default()
    };
    preferences
        .input_modes
        .insert("com.webos.app.hdmi1".into(), InputMode::Off);
    storage::save(&path, &preferences).unwrap();
    let loaded = storage::load(&path).unwrap();
    assert_eq!(loaded.favorites, Some(vec![]));
    assert_eq!(loaded.background, "forest");
    assert_eq!(loaded.input_modes["com.webos.app.hdmi1"], InputMode::Off);
    std::fs::write(&path, "broken").unwrap();
    assert!(storage::load(&path).is_err());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn gallery_and_grid_navigation_scroll_to_keep_focus_visible() {
    let mut state = ready();
    state.set_wallpapers(vec![crate::wallpapers::Wallpaper {
        collection: None,
        credit: None,
        id: "custom-00000000000000000000000000000001".into(),
        directory: PathBuf::from("/preview/wallpapers/custom-00000000000000000000000000000001"),
        unsplash: None,
        has_original: true,
    }]);
    state.open(2);
    state.section = 1;
    state.focus = Target::Background(0);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Background(3));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Background(6));
    assert_eq!(state.scroll, 0., "the first three gallery rows fit");
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Background(9));
    assert!(state.scroll > 0., "the fourth gallery row must scroll");
    let control = controls(&state)
        .into_iter()
        .find(|c| c.target == state.focus)
        .unwrap();
    let viewport = state.backgrounds_viewport();
    assert!(control.rect.y >= viewport.y);
    assert!(control.rect.y + control.rect.h <= viewport.y + viewport.h);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(1));
    state.open(1);
    let last = state.apps().last().unwrap().id.clone();
    state.focus(Target::App(last), &controls(&state));
    assert!(
        state.scroll > 0.,
        "library focus includes its wrapped caption"
    );
}

#[test]
fn cancelling_options_preserves_identity_and_does_not_save() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    state.options();
    assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
    state.focus = Target::CloseDialog;
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::App("netflix".into()));
    assert!(state.overlay.is_none());
}

#[test]
fn pointer_focus_does_not_scroll_the_clicked_control() {
    let mut state = ready();
    state.open(1);
    state.scroll = 150.;
    state.pointer_focus(Target::App("netflix".into()));
    assert_eq!(state.scroll, 150.);
    state.focus(Target::Filter(1), &controls(&state));
    assert_eq!(state.scroll, 0.);
}

#[test]
fn gallery_right_crosses_rows_in_gallery_order() {
    let mut state = ready();
    state.open(2);
    state.section = 1;
    state.focus = Target::Background(2);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::Background(3));
}

#[test]
fn reordering_skips_hidden_and_unavailable_favorite_slots() {
    let mut state = ready();
    state.preferences.favorites = Some(vec![
        "netflix".into(),
        "missing".into(),
        "youtube.leanback.v4".into(),
        "amazon".into(),
    ]);
    state.preferences.hidden.push("youtube.leanback.v4".into());
    state.focus = Target::App("netflix".into());
    state.hold();
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(
        state.preferences.favorites.unwrap(),
        vec!["amazon", "missing", "youtube.leanback.v4", "netflix"]
    );
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn down_past_homes_last_row_opens_the_app_drawer_and_back_returns_to_the_favourite() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.focus = Target::App("youtube.leanback.v4".into());
    state.direction(Direction::Down, &controls(&state));
    assert!(
        matches!(state.focus, Target::Input(_)),
        "Inputs still come first"
    );
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(
        state.page,
        Page::Apps,
        "Down past the last row opens the drawer"
    );
    assert!(matches!(state.focus, Target::App(_)));
    // Up from the drawer's top row (its filters) goes back to Home.
    state.focus = Target::Filter(1);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.page, Page::Home);
    // Opened from a favourite, Back lands on that favourite again.
    state.focus = Target::App("youtube.leanback.v4".into());
    state.open_drawer();
    assert_eq!(state.page, Page::Apps);
    state.back();
    assert_eq!(state.page, Page::Home);
    assert_eq!(state.focus, Target::App("youtube.leanback.v4".into()));
    // With every input hidden from Home, Down from a favourite opens the drawer directly.
    for input in state.inventory.inputs.clone() {
        state
            .preferences
            .input_modes
            .insert(input.id, crate::model::InputMode::Off);
    }
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.page, Page::Apps);
    state.back();
    assert_eq!(state.focus, Target::App("youtube.leanback.v4".into()));
    // The gear opens the quick panel; All settings opens Settings on its first section.
    state.focus = Target::Nav(2);
    state.activate();
    assert!(matches!(state.overlay, Some(Overlay::Quick)));
    state.focus = Target::QuickAll;
    state.activate();
    assert!(state.overlay.is_none());
    assert_eq!(state.page, Page::Settings);
    assert_eq!(state.focus, Target::Sidebar(0));
}

#[test]
fn greeting_subpage_opens_the_keyboard_and_back_restores_its_parent() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(0);
    assert!(
        state
            .direction(Direction::Right, &controls(&state))
            .is_empty()
    );
    assert_eq!(state.subpage, Some(crate::state::Subpage::Greeting));
    assert_eq!(state.focus, Target::GreetingToggle);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::GreetingName);
    assert!(!state.text_input_active());
    assert!(state.activate().is_empty());
    assert!(state.text_input_active());
    state.text("Ana");
    assert_eq!(state.editing_text(), "Ana");
    assert!(state.preferences.greeting_name.is_empty());
    assert_eq!(state.finish_text_input(), vec![Effect::Save]);
    assert_eq!(state.focus, Target::GreetingName);
    assert!(state.finish_text_input().is_empty());
    assert_eq!(
        crate::i18n::Language::En.greeting(9, &state.preferences.greeting_name),
        "Good morning, Ana."
    );
    assert_eq!(state.subpage, Some(crate::state::Subpage::Greeting));
    assert!(state.back().is_empty());
    assert!(state.subpage.is_none());
    assert_eq!(state.focus, Target::Widget(0));
}

#[test]
fn greeting_name_replacement_commits_on_every_keyboard_completion_route() {
    for route in 0..3 {
        let mut state = ready();
        state.preferences.greeting_name = "Ana".into();
        state.open(2);
        state.focus = Target::Widget(0);
        state.activate();
        state.direction(Direction::Down, &controls(&state));
        state.activate();
        assert!(state.editing_text().is_empty());
        state.text("  BobX ");
        state.delete_surrounding_text(-3, 3);
        assert_eq!(state.editing_text(), "  Bo");
        state.text("b  ");
        assert_eq!(state.preferences.greeting_name, "Ana");
        let effects = match route {
            0 => {
                state.overlay = Some(Overlay::SystemKeyboard);
                state.finish_text_input()
            }
            1 => state.back(),
            _ => {
                state.focus = Target::Key(crate::state::KEYS.len() - 1);
                state.activate()
            }
        };
        assert_eq!(effects, vec![Effect::Save]);
        assert_eq!(state.preferences.greeting_name, "Bob");
        assert_eq!(state.focus, Target::GreetingName);
        assert!(state.finish_text_input().is_empty());
        state.activate();
        state.text("   ");
        assert!(state.back().is_empty());
        assert_eq!(state.preferences.greeting_name, "Bob");
    }
}

#[test]
fn remote_can_clear_a_saved_greeting_name_without_moving_the_controls() {
    let mut state = ready();
    state.preferences.greeting_name = "Ana".into();
    state.open(2);
    state.focus = Target::Widget(0);
    state.activate();
    let before: Vec<_> = controls(&state)
        .iter()
        .map(|c| (c.target.clone(), c.rect.y))
        .collect();
    state.direction(Direction::Down, &controls(&state));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::GreetingNameClear);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.greeting_name.is_empty());
    assert_eq!(state.focus, Target::GreetingName);
    let after: Vec<_> = controls(&state)
        .iter()
        .map(|c| (c.target.clone(), c.rect.y))
        .collect();
    assert_eq!(before, after);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Widget(0));
    assert!(state.subpage.is_none());
}

#[test]
fn app_name_settings_use_independent_defaults_including_with_legacy_preferences() {
    let old: Preferences = serde_json::from_value(json!({"show_names": true})).unwrap();
    assert!(!old.show_home_names);
    assert!(old.show_app_names);
    let mut state = ready();
    assert!(!state.preferences.show_home_names);
    assert!(state.preferences.show_app_names);
    state.open(2);
    state.focus = Target::Setting(0);
    assert!(
        state
            .direction(Direction::Right, &controls(&state))
            .is_empty()
    );
    assert_eq!(state.subpage, Some(crate::state::Subpage::AppNames));
    assert_eq!(state.focus, Target::AppNamesHome);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.show_home_names);
    assert!(state.preferences.show_app_names);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::AppNamesApps);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.show_home_names);
    assert!(!state.preferences.show_app_names);
    assert_eq!(state.subpage, Some(crate::state::Subpage::AppNames));
    let restored: Preferences =
        serde_json::from_value(serde_json::to_value(&state.preferences).unwrap()).unwrap();
    assert!(restored.show_home_names);
    assert!(!restored.show_app_names);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Setting(0));
    assert!(state.subpage.is_none());
}

#[test]
fn home_and_apps_render_only_their_own_caption_preference() {
    let mut state = ready();
    for (home, apps) in [(false, true), (true, false), (true, true), (false, false)] {
        state.preferences.show_home_names = home;
        state.preferences.show_app_names = apps;
        for (page, expected) in [(0, home), (1, apps)] {
            state.open(page);
            let scene = scene(&state);
            let caption = scene.draw.iter().any(
                |draw| matches!(draw, crate::renderer::Draw::Text(value, ..) if value == "Netflix"),
            );
            assert_eq!(caption, expected, "Home={home}, Apps={apps}, page={page}");
            let card = scene
                .controls
                .iter()
                .find(|c| c.target == Target::App("netflix".into()))
                .unwrap();
            assert_eq!(
                card.rect.h > 154.4,
                expected,
                "Hidden captions must not reserve space"
            );
        }
    }
}

#[test]
fn input_name_actions_only_highlight_the_focused_row_and_keep_the_saved_mode() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state
        .preferences
        .set_input_name("com.webos.app.hdmi1", "Console");
    state.open(2);
    state.focus = Target::Widget(3);
    state.activate();
    state.focus = Target::InputMode("com.webos.app.hdmi1".into());
    state.activate();
    for target in [Target::InputRename, Target::InputNameReset] {
        state.pointer_focus(target.clone());
        let scene = scene(&state);
        let highlighted: Vec<_> = scene
            .draw
            .iter()
            .filter_map(|draw| match draw {
                crate::renderer::Draw::RoundedGradient(rect, ..)
                    if rect.x == 640. && rect.w == 1120. =>
                {
                    Some(rect.y)
                }
                _ => None,
            })
            .collect();
        let focused = scene.controls.iter().find(|c| c.target == target).unwrap();
        assert_eq!(highlighted, vec![focused.rect.y]);
        assert!(scene.draw.iter().any(|draw| matches!(draw, crate::renderer::Draw::TintedImage(path, ..) if path.ends_with("check.lvec"))));
        assert_eq!(
            state
                .preferences
                .input_modes
                .get("com.webos.app.hdmi1")
                .copied()
                .unwrap_or_default(),
            InputMode::Auto
        );
        assert!(scene.draw.iter().any(
            |draw| matches!(draw, crate::renderer::Draw::Text(value, ..) if value == "Clear name")
        ));
    }
}

#[test]
fn clock_choices_only_highlight_focus_while_preserving_saved_checkmarks() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.preferences.show_seconds = true;
    state.open(2);
    state.focus = Target::Setting(1);
    state.activate();
    state.activate();
    for index in 0..2 {
        state.pointer_focus(Target::Choice(index));
        let scene = scene(&state);
        let highlighted: Vec<_> = scene
            .draw
            .iter()
            .filter_map(|draw| match draw {
                crate::renderer::Draw::RoundedGradient(rect, ..)
                    if rect.x == 640. && rect.w == 1120. =>
                {
                    Some(rect.y)
                }
                _ => None,
            })
            .collect();
        let focused = scene
            .controls
            .iter()
            .find(|c| c.target == Target::Choice(index))
            .unwrap();
        assert_eq!(highlighted, vec![focused.rect.y]);
        let checks = scene.draw.iter().filter(|draw| matches!(draw, crate::renderer::Draw::TintedImage(path, ..) if path.ends_with("check.lvec"))).count();
        assert_eq!(checks, 1);
        assert!(!state.preferences.hour_24);
        assert!(state.preferences.show_seconds);
    }
}

#[test]
fn clock_format_subpage_saves_and_restores_its_parent_row() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Setting(1);
    assert!(
        state
            .direction(Direction::Right, &controls(&state))
            .is_empty()
    );
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert_eq!(state.focus, Target::ClockFormat);
    assert!(
        state
            .direction(Direction::Right, &controls(&state))
            .is_empty()
    );
    assert_eq!(state.subpage, Some(crate::state::Subpage::ClockFormat));
    assert_eq!(state.focus, Target::Choice(0));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.hour_24);
    assert!(!state.preferences.show_seconds);
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert_eq!(state.focus, Target::ClockFormat);
    state.direction(Direction::Left, &controls(&state));
    assert!(state.subpage.is_none());
    assert_eq!(state.focus, Target::Setting(1));
    state.activate();
    state.activate();
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert_eq!(state.focus, Target::ClockFormat);
    state.back();
    assert_eq!(state.focus, Target::Setting(1));
    assert!(state.subpage.is_none());
}

#[test]
fn time_format_selects_seconds_without_changing_the_hour_format() {
    let mut state = ready();
    state.preferences.hour_24 = true;
    state.open(2);
    state.focus = Target::Setting(1);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert!(!state.preferences.show_seconds);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::ClockTimeFormat);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.subpage, Some(crate::state::Subpage::TimeFormat));
    assert_eq!(state.focus, Target::Choice(0));
    assert!(
        !controls(&state)
            .iter()
            .any(|c| c.target == Target::Choice(2))
    );
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Choice(1));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.show_seconds);
    assert!(state.preferences.hour_24);
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert_eq!(state.focus, Target::ClockTimeFormat);
    state.activate();
    assert_eq!(state.focus, Target::Choice(1));
    state.back();
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert_eq!(state.focus, Target::ClockTimeFormat);
    state.activate();
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.show_seconds);
    assert!(state.preferences.hour_24);
    state.back();
    assert!(state.subpage.is_none());
    assert_eq!(state.focus, Target::Setting(1));
}

#[test]
fn system_version_is_inside_the_update_status_card_without_a_separate_row() {
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    state.panel_started = None;
    let system = scene(&state);
    assert!(system.draw.iter().any(|draw| matches!(
        draw,
        crate::renderer::Draw::Shape(rect, _, _, color)
            if rect.x == 640. && rect.y == 164.8 && rect.w == 1120. && rect.h == 220. && color[3] == 0.72
    )));
    assert!(system.draw.iter().any(|draw| matches!(
        draw,
        crate::renderer::Draw::Text(label, _, style, color, _)
            if label == "Current version: test · Based on Luma" && style.size == 20.8 && style.weight == 450 && color[3] == 0.73
    )));
    assert_eq!(state.hit_test(&system.controls, 900., 200.), None);
    state.focus = Target::Sidebar(crate::state::SYSTEM_SECTION);
    state.direction(Direction::Right, &system.controls);
    assert_eq!(state.focus, Target::CheckUpdates);
}

#[test]
fn system_category_owns_refresh_and_quit_and_restores_sidebar_focus() {
    let mut state = ready();
    state.open(2);
    assert!(
        !controls(&state)
            .iter()
            .any(|c| matches!(c.target, Target::Setting(3 | 5 | 9)))
    );
    assert!(
        controls(&state)
            .iter()
            .any(|c| c.target == Target::Sidebar(crate::state::SYSTEM_SECTION))
    );
    state.focus = Target::Sidebar(crate::state::CONTROLS_SECTION);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(crate::state::SYSTEM_SECTION));
    assert_eq!(state.section, crate::state::SYSTEM_SECTION);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::CheckUpdates);
    for row in [4, 2, 7, 3] {
        state.direction(Direction::Down, &controls(&state));
        assert_eq!(state.focus, Target::Setting(row));
    }
    assert_eq!(state.activate(), vec![Effect::Reload]);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Setting(9));
    let preferences = serde_json::to_value(&state.preferences).unwrap();
    assert!(state.activate().is_empty());
    assert!(state.reset_icon_cache);
    assert_eq!(
        serde_json::to_value(&state.preferences).unwrap(),
        preferences
    );
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Setting(5));
    assert_eq!(state.activate(), vec![Effect::Quit]);
    state.back();
    assert_eq!(state.focus, Target::Sidebar(crate::state::SYSTEM_SECTION));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(0));
    assert_eq!(state.section, 0);
}

#[test]
fn system_maintenance_actions_share_an_icon_and_group_below_text_rendering() {
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    state.panel_started = None;
    let system = scene(&state);
    let rect = |index| {
        system
            .controls
            .iter()
            .find(|control| control.target == Target::Setting(index))
            .unwrap()
            .rect
    };
    let text = rect(7);
    let refresh = rect(3);
    let reset = rect(9);
    let quit = rect(5);
    let gap = |above: crate::state::Rect, below: crate::state::Rect| below.y - above.y - above.h;
    assert!(gap(text, refresh) > gap(refresh, reset));
    assert!(gap(reset, quit) > gap(refresh, reset));
    assert!((gap(refresh, reset) - 12.).abs() < 0.01);
    for action in [refresh, reset] {
        assert!(system.draw.iter().any(|draw| matches!(draw,
            crate::renderer::Draw::TintedImage(path, icon, ..)
                if path.ends_with("ui-icons/refresh.lvec")
                    && icon.y >= action.y && icon.y + icon.h <= action.y + action.h
        )));
    }
    state.focus = Target::Setting(9);
    state.direction(Direction::Up, &system.controls);
    assert_eq!(state.focus, Target::Setting(3));
    state.direction(Direction::Up, &system.controls);
    assert_eq!(state.focus, Target::Setting(7));
}

#[test]
fn text_rendering_preferences_default_to_msaa_and_recover_invalid_values() {
    use crate::text_rendering::TextRendering;
    for value in [
        json!({}),
        json!({"text_rendering": "unknown"}),
        json!({"text_rendering": null}),
        json!({"text_rendering": 4}),
    ] {
        let mut value = value;
        value["greeting_name"] = json!("Ana");
        let preferences: Preferences = serde_json::from_value(value).unwrap();
        assert_eq!(preferences.text_rendering, TextRendering::Msaa);
        assert_eq!(preferences.greeting_name, "Ana");
    }
}

#[test]
fn text_rendering_choices_persist_and_back_restores_the_system_row() {
    use crate::{state::Subpage, text_rendering::TextRendering};
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    state.focus = Target::Setting(7);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.subpage, Some(Subpage::TextRendering));
    assert_eq!(state.focus, Target::Choice(0));
    assert!(scene(&state).draw.iter().any(|draw| matches!(
        draw,
        crate::renderer::Draw::Text(label, ..) if label == "Restart Lume to apply changes."
    )));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Choice(1));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.text_rendering, TextRendering::Analytic);
    assert_eq!(state.focus, Target::Setting(7));
    assert!(state.subpage.is_none());
    let saved = serde_json::to_value(&state.preferences).unwrap();
    assert_eq!(saved["text_rendering"], "analytic");
    let mut restored = State::new(serde_json::from_value(saved).unwrap());
    restored.open(2);
    restored.section = crate::state::SYSTEM_SECTION;
    restored.focus = Target::Setting(7);
    restored.activate();
    assert_eq!(restored.focus, Target::Choice(1));
    restored.direction(Direction::Down, &controls(&restored));
    assert_eq!(restored.focus, Target::Choice(0));
    restored.back();
    assert_eq!(restored.focus, Target::Setting(7));
    assert_eq!(restored.preferences.text_rendering, TextRendering::Analytic);
    restored.activate();
    restored.direction(Direction::Up, &controls(&restored));
    assert_eq!(restored.activate(), vec![Effect::Save]);
    assert_eq!(restored.preferences.text_rendering, TextRendering::Msaa);
}

#[test]
fn input_visibility_has_explicit_choices_and_returns_to_the_same_input() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(3);
    state.activate();
    let id = "com.webos.app.hdmi1".to_string();
    state.focus = Target::InputMode(id.clone());
    state.activate();
    assert_eq!(state.focus, Target::Choice(2));
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.input_modes[&id], InputMode::Off);
    assert_eq!(state.focus, Target::InputMode(id.clone()));
    assert!(!state.preferences.input_visible(&state.inventory.inputs[0]));
    state.activate();
    state.back();
    assert_eq!(state.focus, Target::InputMode(id));
    assert_eq!(state.subpage, Some(crate::state::Subpage::Inputs));
    state.back();
    assert_eq!(state.focus, Target::Widget(3));
}

#[test]
fn background_sidebar_right_focuses_upload_and_ok_restores_the_applied_background() {
    let mut state = ready();
    state.open(2);
    state.preferences.background = "forest".into();
    state.focus = Target::Sidebar(1);
    state.section = 1;
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    assert_eq!(state.preferences.background, "forest");
    assert!(!state.preferences.wallpaper_cycle);
    assert!(state.overlay.is_none());
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperGradient);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperCycle);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperGradient);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::WallpaperCollection);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(1));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::UploadWallpaper);
    state.back();
    state.activate();
    assert_eq!(state.focus, Target::Background(6));
}

#[test]
fn categories_and_search_have_clear_actions_and_conditional_hidden_control() {
    let mut state = ready();
    state.open(1);
    assert!(
        !controls(&state)
            .iter()
            .any(|c| c.target == Target::Filter(5))
    );
    state.focus = Target::Search;
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Filter(4));
    state.preferences.hidden.push("netflix".into());
    state.focus = Target::Search;
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Filter(5));
    state.activate();
    assert_eq!(state.apps().len(), 1);
    state.focus = Target::ClearFilter;
    state.activate();
    assert_eq!(state.filter, 0);
    assert_eq!(state.focus, Target::Filter(1));
    state.query = "YouTube".into();
    state.focus = Target::ClearSearch;
    state.activate();
    assert!(state.query.is_empty());
    assert_eq!(state.focus, Target::Search);
}

#[test]
fn a_notice_does_not_escape_modal_focus_and_closing_restores_the_app() {
    let mut state = ready();
    state.open(1);
    state.notify("Saved");
    state.focus = Target::App("netflix".into());
    state.options();
    assert!(
        controls(&state)
            .iter()
            .all(|c| matches!(c.target, Target::OpenApp | Target::Option(_)))
    );
    state.focus = Target::Option(1);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Option(2));
    state.back();
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn delete_confirmation_defaults_to_cancel_and_back_restores_side_popup() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.open(1);
    state.focus = Target::App("netflix".into());
    let card = controls(&state)
        .into_iter()
        .find(|c| c.target == state.focus)
        .unwrap()
        .rect;
    let parent_draw_count = scene(&state).draw.len();
    state.options();
    let popup_scene = scene(&state);
    let popup = popup_scene.modal.unwrap();
    assert!(
        popup_scene
            .controls
            .iter()
            .all(|control| matches!(control.target, Target::OpenApp | Target::Option(_)))
    );
    assert!(!popup_scene.draw[parent_draw_count..].iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Text(value, rect, ..) if popup.contains(rect.x, rect.y)
            && ["Netflix", "Streaming", "Select", "Close", "OK", "Back"].contains(&value.as_str()))));
    let action_icons: Vec<_> = popup_scene.draw[parent_draw_count..]
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Image(_, rect, ..)
            | crate::renderer::Draw::TintedImage(_, rect, ..)
                if popup.contains(rect.x, rect.y) =>
            {
                Some(rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(action_icons.len(), 4);
    assert!(
        action_icons
            .iter()
            .all(|rect| rect.x > popup.x + popup.w - 80.)
    );
    assert!(popup.x >= card.x + card.w || popup.x + popup.w <= card.x);
    state.focus = Target::Option(2);
    let focused_scene = scene(&state);
    let delete_rect = focused_scene
        .controls
        .iter()
        .find(|c| c.target == Target::Option(2))
        .unwrap()
        .rect;
    assert!(focused_scene.draw.iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Shape(rect, _, _, color) if rect.x == delete_rect.x && rect.y == delete_rect.y
            && *color == [214. / 255., 59. / 255., 53. / 255., 1.])));
    assert!(focused_scene.draw.iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Text(label, rect, _, color, _) if label == "Delete app" && delete_rect.contains(rect.x, rect.y)
            && *color == crate::renderer::WHITE)));
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::Delete { .. })));
    assert_eq!(state.focus, Target::CancelDelete);
    let confirmation = scene(&state).modal.unwrap();
    assert_eq!(confirmation.w, 640.);
    assert!(confirmation.h < confirmation.w);
    assert!(
        controls(&state)
            .iter()
            .all(|c| matches!(c.target, Target::ConfirmDelete | Target::CancelDelete))
    );
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
    assert_eq!(state.focus, Target::Option(2));
    state.activate();
    state.back();
    assert_eq!(state.focus, Target::Option(2));
    state.back();
    assert_eq!(state.focus, Target::App("netflix".into()));
}

#[test]
fn opening_side_popup_preserves_card_artwork_geometry_with_or_without_captions() {
    for page in [0, 1] {
        for names in [false, true] {
            let mut state = ready();
            state.preferences.show_app_names = names;
            state.preferences.show_home_names = names;
            state.open(page);
            state.focus = Target::App("netflix".into());
            let before = scene(&state);
            let logo_rect = |draws: &[crate::renderer::Draw]| {
                draws
                    .iter()
                    .filter_map(|draw| match draw {
                        crate::renderer::Draw::Image(path, rect, ..)
                        | crate::renderer::Draw::TintedImage(path, rect, ..)
                            if path.contains("netflix") =>
                        {
                            Some([rect.x, rect.y, rect.w, rect.h])
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            let original = logo_rect(&before.draw);
            assert!(!original.is_empty());
            state.options();
            let lifted = logo_rect(&scene(&state).draw);
            assert_eq!(lifted.len(), original.len() * 2);
            for (actual, expected) in lifted.iter().zip(original.iter().cycle()) {
                for (a, b) in actual.iter().zip(expected) {
                    assert!(
                        (a - b).abs() < 0.001,
                        "Artwork bounds changed: {actual:?} vs {expected:?}"
                    );
                }
            }
            let hearts = |draws: &[crate::renderer::Draw]| {
                draws
                    .iter()
                    .filter_map(|draw| match draw {
                        crate::renderer::Draw::Image(path, rect, ..)
                        | crate::renderer::Draw::TintedImage(path, rect, ..)
                            if path.contains("heart-filled") && rect.x < 350. =>
                        {
                            Some([rect.x, rect.y, rect.w, rect.h])
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            let original_hearts = hearts(&before.draw);
            assert_eq!(original_hearts.len(), usize::from(page == 1));
            let popup_hearts = hearts(&scene(&state).draw);
            assert_eq!(popup_hearts.len(), original_hearts.len() + 1);
            if page == 1 {
                assert_eq!(popup_hearts, vec![original_hearts[0], original_hearts[0]]);
            }
        }
    }
}

#[test]
fn protected_apps_cannot_be_deleted_through_ui_or_worker() {
    for id in [
        crate::platform::APP_ID,
        crate::platform::HOME_ID,
        "com.webos.app.livetv",
        "com.webos.app.browser",
        "com.webos.app.settings",
        "com.webos.service.applicationmanager",
        "com.palm.app.settings",
        "com.palm.app.input",
        "com.lge.app.store",
        "com.lge.service.example",
        "com.luma.launcher",
        "com.luma.native",
        "com.luma.launcher.native",
        "org.webosbrew.hbchannel",
        "org.webosbrew.hbchannel.service",
        "custom.lamp",
        "invalid';command",
    ] {
        assert!(crate::platform::uninstall_request(id).is_err());
        let mut state = ready();
        state.inventory.apps.push(App::new(id, "Renamed app"));
        state.open(1);
        state.focus = Target::App(id.into());
        state.options();
        assert!(
            !controls(&state)
                .iter()
                .any(|c| c.target == Target::Option(2))
        );
        state.focus = Target::Option(2);
        assert!(state.activate().is_empty());
        assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
        state.overlay = Some(Overlay::Delete {
            id: id.into(),
            restore: Target::App(id.into()),
        });
        state.focus = Target::ConfirmDelete;
        assert!(state.activate().is_empty());
    }
    assert_eq!(
        crate::platform::uninstall_request("netflix").unwrap(),
        (
            "luna://com.webos.appInstallService/remove",
            json!({"id": "netflix", "subscribe": true})
        )
    );
    for id in [
        "com.example.app",
        "com.webosbrew.app",
        "com.palmtree.app",
        "com.lgeneric.app",
    ] {
        assert!(
            crate::platform::can_uninstall(id),
            "Unexpectedly protected: {id}"
        );
    }
}

#[test]
fn deletion_is_single_flight_and_only_success_cleans_up_preferences() {
    let mut state = ready();
    state.open(1);
    state.preferences.favorites = Some(vec!["netflix".into(), "youtube.leanback.v4".into()]);
    state.preferences.hidden.push("netflix".into());
    state.focus = Target::App("netflix".into());
    state.options();
    state.focus = Target::Option(2);
    state.activate();
    state.focus = Target::ConfirmDelete;
    let cancel_before = controls(&state)
        .into_iter()
        .find(|c| c.target == Target::CancelDelete)
        .unwrap()
        .rect;
    assert_eq!(state.activate(), vec![Effect::Uninstall("netflix".into())]);
    assert_eq!(state.focus, Target::CancelDelete);
    let cancel_after = controls(&state)
        .into_iter()
        .find(|c| c.target == Target::CancelDelete)
        .unwrap()
        .rect;
    assert_eq!(
        [
            cancel_before.x,
            cancel_before.y,
            cancel_before.w,
            cancel_before.h
        ],
        [
            cancel_after.x,
            cancel_after.y,
            cancel_after.w,
            cancel_after.h
        ]
    );
    assert!(
        !controls(&state)
            .iter()
            .any(|c| c.target == Target::ConfirmDelete)
    );
    state.focus = Target::ConfirmDelete;
    assert!(state.activate().is_empty());
    assert!(state.uninstall_response("other".into(), Ok(())).is_empty());
    assert_eq!(state.uninstalling.as_deref(), Some("netflix"));
    assert_eq!(
        state.uninstall_response("netflix".into(), Err("denied".into())),
        vec![Effect::Refresh]
    );
    assert!(state.inventory.apps.iter().any(|a| a.id == "netflix"));
    assert!(state.preferences.hidden.contains(&"netflix".into()));
    assert_eq!(state.preferences.favorites.as_ref().unwrap().len(), 2);
    assert_eq!(state.focus, state.first_content());
    assert!(state.overlay.is_none());
    state.focus = Target::App("netflix".into());
    state.options();
    state.focus = Target::Option(2);
    state.activate();
    state.focus = Target::ConfirmDelete;
    state.activate();
    assert_eq!(
        state.uninstall_response("netflix".into(), Ok(())),
        vec![Effect::Save, Effect::Refresh]
    );
    assert!(!state.inventory.apps.iter().any(|a| a.id == "netflix"));
    assert!(state.preferences.hidden.is_empty());
    assert_eq!(
        state.preferences.favorites,
        Some(vec!["youtube.leanback.v4".into()])
    );
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::App("youtube.leanback.v4".into()));
}

#[test]
fn deletion_completion_does_not_steal_focus_after_dismissal() {
    let mut state = ready();
    state.uninstalling = Some("netflix".into());
    state.open(2);
    state.uninstall_response("netflix".into(), Ok(()));
    assert_eq!(state.page, Page::Settings);
    assert_eq!(state.focus, Target::Nav(2));
    assert!(state.overlay.is_none());
}

#[test]
fn app_activation_opens_options_with_open_focused_and_next_ok_launches() {
    for page in [0, 1] {
        let mut state = ready();
        state.open(page);
        state.focus = Target::App("netflix".into());
        if page == 0 {
            assert_eq!(state.activate(), vec![Effect::Launch("netflix".into())]);
            assert!(state.overlay.is_none());
            continue;
        }
        assert!(state.activate().is_empty());
        assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
        assert_eq!(state.focus, Target::OpenApp);
        assert_eq!(state.activate(), vec![Effect::Launch("netflix".into())]);
        assert!(state.overlay.is_none());
        assert_eq!(state.focus, Target::App("netflix".into()));
    }
}

#[test]
fn uncertain_deletion_uses_complete_inventory_instead_of_visible_app_filtering() {
    use crate::platform::inventory_confirms_removal;
    assert_eq!(inventory_confirms_removal(&json!([]), "netflix"), Ok(true));
    assert_eq!(
        inventory_confirms_removal(&json!([{"id": "youtube.leanback.v4"}]), "netflix"),
        Ok(true)
    );
    assert_eq!(
        inventory_confirms_removal(&json!([{"id": "netflix", "visible": false}]), "netflix"),
        Ok(false)
    );
    assert!(inventory_confirms_removal(&json!({}), "netflix").is_err());
    assert!(inventory_confirms_removal(&json!([{}]), "netflix").is_err());
}

#[test]
fn failed_deletion_refreshes_inventory_and_closes_stale_confirmation() {
    let mut state = ready();
    state.open(1);
    state.focus = Target::App("netflix".into());
    state.options();
    state.focus = Target::Option(2);
    state.activate();
    state.focus = Target::ConfirmDelete;
    state.activate();
    assert_eq!(
        state.uninstall_response("netflix".into(), Err("timeout".into())),
        vec![Effect::Refresh]
    );
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::App("netflix".into()));
    let mut inventory = demo_inventory();
    inventory.apps.retain(|app| app.id != "netflix");
    state.set_inventory(inventory, None);
    assert!(!state.apps().iter().any(|app| app.id == "netflix"));
    assert_ne!(state.focus, Target::App("netflix".into()));
}

#[test]
fn remote_toolbar_focus_normalizes_scroll_but_hover_does_not() {
    let mut state = ready();
    state.open(1);
    state.scroll = 100.;
    state.pointer_focus(Target::Filter(1));
    assert_eq!(state.scroll, 100.);
    state.focus(Target::Filter(2), &controls(&state));
    assert_eq!(state.scroll, 0.);
}

#[test]
fn long_captions_expand_rows_without_overlapping_the_next_row() {
    let mut state = ready();
    state.open(1);
    state.inventory.apps.iter_mut().find(|app| app.id == "netflix").unwrap().title = "A very long installed application title that needs more than two lines to remain fully readable".into();
    let targets = controls(&state);
    let title = targets
        .iter()
        .find(|c| c.target == Target::App("netflix".into()))
        .unwrap();
    let next_id = state.apps()[6].id.clone();
    let next = targets
        .iter()
        .find(|c| c.target == Target::App(next_id.clone()))
        .unwrap();
    assert!(title.rect.h > 240.);
    assert!(next.rect.y > title.rect.y + title.rect.h);
}

#[test]
fn reduced_motion_disables_panel_wallpaper_and_notification_animation() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.open(2);
    state.focus = Target::Setting(1);
    state.activate();
    assert!(!state.animating());
    state.back();
    state.focus = Target::Background(6);
    state.activate();
    state.notify("Saved");
    assert!(!state.animating());
}

#[test]
fn pinning_a_hidden_app_restores_it_and_leaves_hidden_when_empty() {
    let mut state = ready();
    state.open(1);
    state.filter = 5;
    state.preferences.hidden.push("org.jellyfin.webos".into());
    state.focus = Target::App("org.jellyfin.webos".into());
    state.options();
    assert_eq!(state.focus, Target::OpenApp);
    state.focus = Target::Option(0);
    state.activate();
    assert!(state.preferences.hidden.is_empty());
    assert_eq!(state.filter, 0);
    assert!(
        state
            .preferences
            .favorites(&state.inventory.apps)
            .contains(&"org.jellyfin.webos".into())
    );
    assert_eq!(state.focus, Target::App("org.jellyfin.webos".into()));
}

#[test]
fn weather_location_keyboard_back_and_sidebar_routes_restore_focus() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Sidebar(crate::state::WIDGETS_SECTION);
    state.section = crate::state::WIDGETS_SECTION;
    state.activate();
    assert_eq!(state.focus, Target::Widget(4));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Widget(0));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Widget(1));
    state.activate();
    assert_eq!(state.focus, Target::WeatherToggle);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::WeatherLocation);
    state.activate();
    assert_eq!(state.focus, Target::WeatherQuery);
    state.activate();
    state.text("Vienna");
    state.back();
    assert_eq!(state.focus, Target::WeatherSearch);
    assert_eq!(state.weather.query, "Vienna");
    assert_eq!(state.subpage, Some(crate::state::Subpage::WeatherLocation));
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::WeatherLocation);
    assert_eq!(state.subpage, Some(crate::state::Subpage::Weather));
    state.back();
    assert_eq!(state.focus, Target::Widget(1));
    assert!(state.subpage.is_none());
    state.back();
    assert_eq!(state.focus, Target::Sidebar(crate::state::WIDGETS_SECTION));
}

#[test]
fn weather_units_save_without_replacing_celsius_cache_and_hide_keeps_city() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(1);
    state.activate();
    let place = crate::weather::Location {
        name: "Vienna".into(),
        region: "Vienna".into(),
        country: "Austria".into(),
        latitude: 48.2085,
        longitude: 16.3721,
    };
    state.preferences.weather_location = Some(place.clone());
    state.weather.cache = Some(crate::weather::Cache {
        key: place.key(),
        updated_at: 1000,
        reading: crate::weather::Reading {
            temperature: 14.2,
            code: 2,
            is_day: true,
            daily: None,
            sun: None,
        },
    });
    state.focus = Target::WeatherTemperature;
    state.activate();
    assert_eq!(state.focus, Target::Choice(0));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(
        state.preferences.temperature_unit,
        crate::weather::Unit::Fahrenheit
    );
    assert_eq!(
        state.weather.cache.as_ref().unwrap().reading.temperature,
        14.2
    );
    assert_eq!(state.focus, Target::WeatherTemperature);
    state.focus = Target::WeatherToggle;
    state.activate();
    assert!(!state.preferences.show_weather);
    assert_eq!(state.preferences.weather_location, Some(place));
}

#[test]
fn home_weather_setup_returns_through_the_widgets_weather_parent() {
    use crate::state::{Subpage, WIDGETS_SECTION};
    let mut state = ready();
    state.focus = Target::WeatherConfigure;
    state.activate();
    assert_eq!(state.page, Page::Settings);
    assert_eq!(state.section, WIDGETS_SECTION);
    assert_eq!(state.subpage, Some(Subpage::WeatherLocation));
    assert!(state.text_input_active());
    state.back();
    state.back();
    assert_eq!(state.subpage, Some(Subpage::Weather));
    assert_eq!(state.focus, Target::WeatherLocation);
    state.back();
    assert!(state.subpage.is_none());
    assert_eq!(state.focus, Target::Widget(1));
}

#[test]
fn removing_weather_location_clears_cache_and_cancels_pending_results() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(1);
    state.activate();
    state.focus = Target::WeatherLocation;
    state.activate();
    state.focus = Target::WeatherClear;
    let effects = state.activate();
    assert_eq!(
        effects,
        vec![
            Effect::Save,
            Effect::Weather(crate::weather::Command::ClearCache)
        ]
    );
    assert!(state.weather.cache.is_none());
    assert!(state.preferences.weather_location.is_none());
    assert_eq!(state.focus, Target::WeatherSearch);
}

#[test]
fn language_selection_restores_its_parent_and_is_saved_with_preferences() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Setting(4);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.subpage, Some(crate::state::Subpage::Language));
    assert_eq!(
        state.focus,
        Target::Choice(crate::i18n::Language::En.index())
    );
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.language, crate::i18n::Language::De);
    assert_eq!(state.focus, Target::Setting(4));
    state.activate();
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Setting(4));
}

#[test]
fn remote_can_select_and_restore_every_language() {
    for (index, language) in crate::i18n::Language::ALL.into_iter().enumerate() {
        let mut state = ready();
        state.open(2);
        state.focus = Target::Setting(4);
        state.activate();
        for _ in 0..crate::i18n::Language::En.index() {
            state.direction(Direction::Up, &controls(&state));
        }
        for _ in 0..index {
            state.direction(Direction::Down, &controls(&state));
        }
        assert_eq!(state.focus, Target::Choice(index));
        let focused = controls(&state)
            .into_iter()
            .find(|control| control.target == state.focus)
            .unwrap();
        let viewport = state.content_viewport();
        assert!(focused.rect.y >= viewport.y);
        assert!(focused.rect.y + focused.rect.h <= viewport.y + viewport.h);
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert_eq!(state.preferences.language, language);
        assert_eq!(state.focus, Target::Setting(4));

        let saved = serde_json::to_value(&state.preferences).unwrap();
        let mut restored = State::new(serde_json::from_value(saved).unwrap());
        restored.open(2);
        restored.focus = Target::Setting(4);
        restored.activate();
        assert_eq!(restored.preferences.language, language);
        assert_eq!(restored.focus, Target::Choice(index));
        restored.direction(Direction::Down, &controls(&restored));
        assert_eq!(
            restored.focus,
            Target::Choice((index + 1) % crate::i18n::Language::ALL.len())
        );
    }
}

#[test]
fn settings_order_language_sort_and_shortcut_empty_card() {
    use crate::{i18n::Language, renderer::Draw, state::Subpage};
    assert!(
        Language::ALL
            .windows(2)
            .all(|pair| pair[0].name().to_lowercase() < pair[1].name().to_lowercase())
    );
    for (index, language) in Language::ALL.into_iter().enumerate() {
        assert_eq!(language.index(), index);
    }
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    let general = controls(&state);
    let language = general
        .iter()
        .find(|control| control.target == Target::Setting(4))
        .unwrap();
    let motion = general
        .iter()
        .find(|control| control.target == Target::Setting(2))
        .unwrap();
    assert!(language.rect.y < motion.rect.y);
    state.subpage = Some(Subpage::Shortcuts);
    let empty = scene(&state);
    assert!(empty.draw.iter().any(|draw| matches!(draw,
        Draw::Shape(rect, radius, stroke, color) if rect.x == 640. && rect.y == 248.8 && rect.h == 220.
            && *radius == 20. && *stroke == 0. && color[3] == 0.72
    )));
    assert!(empty.draw.iter().any(|draw| matches!(draw,
        Draw::Text(label, _, style, ..) if label == state.preferences.language.t("noTiles") && style.size == 24.
    )));
    assert!(empty.draw.iter().any(|draw| matches!(draw,
        Draw::Text(label, ..) if label == "Press Configure to add your first shortcut."
    )));
    assert!(
        empty
            .controls
            .iter()
            .any(|control| control.target == Target::TileEditor)
    );
}

#[test]
fn unsupported_language_preferences_recover_without_losing_other_choices() {
    let preferences: Preferences = serde_json::from_value(
        json!({"language":"unknown","favorites":["netflix"],"show_home_names":true}),
    )
    .unwrap();
    assert_eq!(preferences.language, crate::i18n::Language::En);
    assert!(preferences.show_home_names);
    assert_eq!(preferences.favorites, Some(vec!["netflix".into()]));
}

#[test]
fn unrelated_app_ids_and_titles_keep_their_installed_artwork() {
    for (id, title) in [
        ("com.tvweb.dashboard", "Glasshouse"),
        ("com.example.ardent", "Notes"),
        ("com.example.3ard", "Notes"),
        ("com.example.ard3", "Notes"),
        ("com.example.plexus", "Notes"),
        ("com.example.media", "Standard dashboard"),
        ("com.example.media", "netflixer"),
        ("com.example.media", "preNetflix"),
        ("amazon.alexa.view", "Amazon Alexa"),
        ("com.example.spotify", "Spotify tools"),
        ("com.example.media", "Plex"),
        ("com.example.browser", "Web Browser"),
        ("com.example.livetv", "Live TV"),
    ] {
        let app = App::new(id, title);
        assert_eq!(
            (app.brand.as_str(), app.category.as_str()),
            ("generic", "Utilities"),
            "An unrelated application must not be rebranded: {id} / {title}"
        );
    }
}

#[test]
fn curated_branding_does_not_use_title_words_or_partial_id_matches() {
    for (id, title) in [
        ("com.example.ard_more", "Notes"),
        ("com.example.more-ard", "Notes"),
        ("com.example.ARD", "Notes"),
        ("com.example.media", "ARD"),
        ("com.example.media", "(ARD)"),
        ("com.example.media", "ARD미디어테크"),
        ("com.example.media", "아에르데ARD"),
        ("com.example.media", "아에르데ARD미디어테크"),
        ("com.example.dashboard", "ARD"),
        ("com.example.media", "standard ardent ARD"),
    ] {
        let app = App::new(id, title);
        assert_eq!(
            (app.brand.as_str(), app.category.as_str()),
            ("generic", "Utilities"),
            "Only an exact app ID may select curated artwork: {id} / {title}"
        );
    }
}

#[test]
fn known_app_ids_preserve_the_curated_brands_and_categories() {
    let apps = demo_inventory().apps;
    let expected = [
        ("com.webos.app.livetv", "live-tv", "Streaming"),
        ("netflix", "netflix", "Streaming"),
        ("youtube.leanback.v4", "youtube", "Streaming"),
        ("com.apple.appletv", "apple-tv", "Streaming"),
        ("com.disney.disneyplus-prod", "disney-plus", "Streaming"),
        ("amazon", "prime-video", "Streaming"),
        ("plex", "plex", "Streaming"),
        ("spotify-beehive", "spotify", "Music"),
        ("org.jellyfin.webos", "jellyfin", "Streaming"),
        ("org.xbmc.kodi", "kodi", "Streaming"),
        ("com.limelight.webos", "moonlight", "Gaming"),
        ("org.webosbrew.hbchannel", "homebrew", "Utilities"),
        ("com.webos.app.browser", "browser", "Utilities"),
        ("com.arte.tv", "arte", "Streaming"),
        ("de.zdf.app.zdfm3", "zdf", "Streaming"),
        ("ard.mediathek", "das-erste", "Streaming"),
    ];
    for (id, brand, category) in expected {
        let app = apps.iter().find(|app| app.id == id).unwrap();
        assert_eq!(
            (app.brand.as_str(), app.category.as_str()),
            (brand, category),
            "Known app {id} must retain its curated brand and category"
        );
    }
}

#[test]
fn hdmi_plug_in_reports_the_cable_state_when_connected_only_means_present() {
    let inputs = normalize_inputs(&json!([
        {"id":"HDMI_1","label":"HDMI 1","port":1,"connected":true,"appId":"com.webos.app.hdmi1","hdmiPlugIn":false},
        {"id":"HDMI_2","label":"AVR-X1800H","port":2,"connected":true,"appId":"com.webos.app.hdmi2","hdmiPlugIn":true},
        {"id":"HDMI_3","label":"HDMI 3","port":3,"connected":true,"appId":"com.webos.app.hdmi3","hdmiPlugIn":false},
        {"id":"HDMI_4","label":"HDMI 4","port":4,"connected":true,"appId":"com.webos.app.hdmi4","hdmiPlugIn":false}
    ]))
    .unwrap();
    let connected: Vec<_> = inputs
        .iter()
        .map(|input| (input.id.as_str(), input.connected))
        .collect();
    assert_eq!(
        connected,
        [
            ("com.webos.app.hdmi1", Some(false)),
            ("com.webos.app.hdmi2", Some(true)),
            ("com.webos.app.hdmi3", Some(false)),
            ("com.webos.app.hdmi4", Some(false)),
        ]
    );
    assert_eq!(inputs[1].title, "AVR-X1800H");
    let preferences = Preferences::default();
    let visible: Vec<_> = inputs
        .iter()
        .filter(|input| preferences.input_visible(input))
        .map(|input| input.id.as_str())
        .collect();
    assert_eq!(visible, ["com.webos.app.hdmi2"]);
}

#[test]
fn inputs_without_hdmi_plug_in_keep_the_connected_report() {
    let inputs = normalize_inputs(&json!([
        {"appId":"com.webos.app.hdmi1","id":"HDMI_1","port":1,"connected":true},
        {"appId":"com.webos.app.hdmi2","id":"HDMI_2","port":2,"connected":false},
        {"appId":"com.webos.app.hdmi3","connected":true,"hdmiPlugIn":"yes"},
        {"appId":"com.webos.app.hdmi4"}
    ]))
    .unwrap();
    let connected: Vec<_> = inputs.iter().map(|input| input.connected).collect();
    assert_eq!(connected, [Some(true), Some(false), Some(true), None]);
}

#[test]
fn input_names_are_sanitized_and_old_preferences_load_without_them() {
    let old: Preferences = serde_json::from_value(json!({"background": "forest"})).unwrap();
    assert!(old.input_names.is_empty());
    let mut preferences: Preferences = serde_json::from_value(json!({
        "input_names": {
            "com.webos.app.hdmi1": "  Play\nStation\t5  ",
            "com.webos.app.hdmi2": "   ",
            "bad;id": "Ignored",
            "com.webos.app.hdmi3": "A".repeat(60),
        }
    }))
    .unwrap();
    preferences.sanitize();
    assert_eq!(
        preferences.input_names,
        BTreeMap::from([
            (
                "com.webos.app.hdmi1".to_string(),
                "PlayStation5".to_string()
            ),
            (
                "com.webos.app.hdmi3".to_string(),
                "A".repeat(crate::model::INPUT_NAME_LIMIT)
            ),
        ])
    );
    assert!(preferences.set_input_name("com.webos.app.hdmi2", " DENON "));
    assert!(!preferences.set_input_name("com.webos.app.hdmi2", "DENON"));
    assert_eq!(preferences.input_names["com.webos.app.hdmi2"], "DENON");
    assert!(preferences.set_input_name("com.webos.app.hdmi2", ""));
    assert!(!preferences.input_names.contains_key("com.webos.app.hdmi2"));
    assert!(!preferences.set_input_name("bad;id", "Name"));
}

#[test]
fn custom_input_names_keep_the_label_port_and_status_as_detail() {
    let mut preferences = Preferences::default();
    let input = Input {
        id: "com.webos.app.hdmi2".into(),
        title: "AVR-X1800H".into(),
        port: "HDMI 2".into(),
        connected: Some(true),
        signal: None,
    };
    let language = preferences.language;
    let status = |preferences: &Preferences| {
        view::input_status(&input, preferences.input_name(&input).is_some(), language)
    };
    assert_eq!(preferences.input_title(&input), "HDMI 2");
    assert_eq!(status(&preferences), "AVR-X1800H · Connected");
    preferences.set_input_name(&input.id, "DENON");
    assert_eq!(preferences.input_title(&input), "DENON");
    assert_eq!(status(&preferences), "AVR-X1800H · HDMI 2 · Connected");
    let plain = Input {
        title: "HDMI 2".into(),
        ..input.clone()
    };
    assert_eq!(
        view::input_status(&plain, true, language),
        "HDMI 2 · Connected"
    );
    assert_eq!(view::input_status(&plain, false, language), "Connected");
}

#[test]
fn remote_renames_and_resets_an_input_from_its_settings_page() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(3);
    state.activate();
    let id = "com.webos.app.hdmi2".to_string();
    state.focus = Target::InputMode(id.clone());
    state.activate();
    assert_eq!(state.focus, Target::Choice(2));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::InputRename);
    assert!(
        !controls(&state)
            .iter()
            .any(|c| c.target == Target::InputNameReset)
    );
    state.activate();
    assert!(matches!(state.overlay, Some(Overlay::Keyboard)));
    state.text("DENONX");
    state.delete();
    assert_eq!(state.back(), vec![Effect::Save]);
    assert_eq!(state.focus, Target::InputRename);
    assert_eq!(state.preferences.input_names[&id], "DENON");
    assert_eq!(
        state.preferences.input_title(&state.inventory.inputs[1]),
        "DENON"
    );
    // Dismissing an empty keyboard keeps the saved name.
    state.activate();
    assert!(state.finish_text_input().is_empty());
    assert_eq!(state.preferences.input_names[&id], "DENON");
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::InputNameReset);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.input_names.is_empty());
    assert_eq!(state.focus, Target::InputRename);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Choice(2));
    state.focus = Target::InputRename;
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::InputMode(id));
    assert_eq!(state.subpage, Some(crate::state::Subpage::Inputs));
}

#[test]
fn keyboard_done_saves_a_typed_input_name() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(3);
    state.activate();
    state.focus = Target::InputMode("com.webos.app.hdmi1".into());
    state.activate();
    state.focus = Target::InputRename;
    state.activate();
    state.text("PS5");
    state.focus = Target::Key(
        crate::state::KEYS
            .iter()
            .position(|k| *k == "Done")
            .unwrap(),
    );
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.input_names["com.webos.app.hdmi1"], "PS5");
    assert!(state.overlay.is_none());
}

#[test]
fn home_preload_launches_start_hidden() {
    for key in ["nid", "appId"] {
        for id in [crate::platform::HOME_ID, crate::platform::FULL_HOME_ID] {
            for mode in [
                "full",
                "semi-full",
                "partial",
                "minimal",
                "criu",
                "future-mode",
            ] {
                let mut options = crate::Options::default();
                options.apply_launch_metadata(&json!({
                    "@system_native_app": true,
                    (key): id,
                    "preload": mode,
                }));
                assert!(options.webos_home);
                assert_eq!(options.preload.as_deref(), Some(mode));
                assert!(options.start_hidden(), "{key}={id}, preload={mode}");
            }
        }
    }
}

#[test]
fn ordinary_home_launches_do_not_start_hidden() {
    for metadata in [
        json!({"nid": crate::platform::HOME_ID}),
        json!({"nid": crate::platform::HOME_ID, "preload": ""}),
        json!({"nid": crate::platform::HOME_ID, "preload": true}),
        json!({"nid": crate::platform::HOME_ID, "preload": null}),
        json!({"nid": crate::platform::HOME_ID, "preload": 1}),
    ] {
        let mut options = crate::Options::default();
        options.apply_launch_metadata(&metadata);
        assert!(options.webos_home);
        assert_eq!(options.preload, None);
        assert!(!options.start_hidden(), "{metadata}");
    }
}

#[test]
fn normal_app_preload_does_not_change_startup_visibility() {
    for metadata in [
        json!({"nid": crate::platform::APP_ID, "preload": "full"}),
        json!({"appId": crate::platform::APP_ID, "preload": "full"}),
        json!({"preload": "full"}),
        json!({}),
    ] {
        let mut options = crate::Options::default();
        options.apply_launch_metadata(&metadata);
        assert!(!options.webos_home);
        assert_eq!(options.preload, None);
        assert!(!options.start_hidden(), "{metadata}");
    }
}

#[test]
fn home_lifecycle_replies_map_for_both_registration_versions() {
    use crate::platform::Lifecycle;
    // Lifecycle v2 (registerApp, in place of LG's Home executable) names it in "event".
    assert_eq!(
        Lifecycle::from_reply(&json!({"event": "relaunch", "reason": "hotKey"})),
        Some(Lifecycle::Relaunch)
    );
    assert_eq!(
        Lifecycle::from_reply(&json!({"event": "close"})),
        Some(Lifecycle::Close)
    );
    // Lifecycle v1 (registerNativeApp for a staged Home manifest on webOS 10/11) in "message".
    assert_eq!(
        Lifecycle::from_reply(&json!({
            "parameters": {"launchType": "homeKey", "activateType": "home"},
            "returnValue": true,
            "message": "relaunch"
        })),
        Some(Lifecycle::Relaunch)
    );
    assert_eq!(
        Lifecycle::from_reply(&json!({"returnValue": true, "message": "close"})),
        Some(Lifecycle::Close)
    );
    // Acknowledgements and keep-alive need no action.
    for reply in [
        json!({"returnValue": true, "message": "registered"}),
        json!({"event": "registered"}),
        json!({"event": "pause", "reason": "keepAlive"}),
        json!({"returnValue": false, "errorText": "refused"}),
    ] {
        assert_eq!(Lifecycle::from_reply(&reply), None, "{reply}");
    }
}

#[test]
fn home_lifecycle_fallback_retries_legacy_and_other_versions_but_not_current_failures() {
    use crate::platform::{disable_home_lifecycle, home_lifecycle_disabled};
    let directory = std::env::temp_dir().join(format!(
        "luma-lifecycle-fallback-test-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let marker = directory.join("no-lifecycle");
    assert!(!home_lifecycle_disabled(&marker, "3.6.2"));

    // A legacy empty marker no longer disables registration indefinitely.
    std::fs::write(&marker, b"").unwrap();
    assert!(!home_lifecycle_disabled(&marker, "3.6.2"));

    // A failed retry records this release. Repeated launches retain the safety fallback.
    disable_home_lifecycle(&marker, "3.6.2").unwrap();
    assert!(home_lifecycle_disabled(&marker, "3.6.2"));
    assert!(home_lifecycle_disabled(&marker, "3.6.2"));

    // Upgrades (and switching releases) retry; merely reading does not erase the marker.
    assert!(!home_lifecycle_disabled(&marker, "3.6.3"));
    assert!(!home_lifecycle_disabled(&marker, "3.6.1"));
    assert!(home_lifecycle_disabled(&marker, "3.6.2"));
    disable_home_lifecycle(&marker, "3.6.3").unwrap();
    assert!(home_lifecycle_disabled(&marker, "3.6.3"));

    // Malformed, unknown and unreadable markers fail safe instead of retrying endlessly.
    for contents in ["broken", "{}", r#"{"version":null}"#, r#"{"version":""}"#] {
        std::fs::write(&marker, contents).unwrap();
        assert!(home_lifecycle_disabled(&marker, "3.6.3"), "{contents}");
    }
    assert!(home_lifecycle_disabled(&directory, "3.6.3"));
    assert!(disable_home_lifecycle(&directory, "3.6.3").is_err());
    std::fs::remove_file(marker).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn foreground_check_needs_focus_after_a_relaunch_from_the_background() {
    use crate::platform::ForegroundCheck;
    use std::time::{Duration, Instant};
    let start = Instant::now();
    let late = start + ForegroundCheck::TIMEOUT;

    // Relaunch while hidden, then only Shown (SDL emits it for SDL_ShowWindow itself): fails.
    let mut check = ForegroundCheck::default();
    check.relaunched(start, false);
    assert!(!check.failed(start + Duration::from_secs(1)));
    assert!(check.failed(late));
    // Reported once.
    assert!(!check.failed(late + Duration::from_secs(1)));

    // FocusGained or a remote key in time: no failure.
    let mut check = ForegroundCheck::default();
    check.relaunched(start, false);
    check.confirmed();
    assert!(!check.failed(late));

    // Already focused and visible (Home pressed on Home): nothing to check.
    let mut check = ForegroundCheck::default();
    check.relaunched(start, true);
    assert!(!check.failed(late));

    // A later relaunch from the background is checked again.
    check.relaunched(late, false);
    assert!(check.failed(late + ForegroundCheck::TIMEOUT));
}

#[test]
fn prewarm_waits_while_recents_replaces_the_page() {
    let mut state = ready();
    let mut jobs = vec![Page::Settings, Page::Apps];
    state.open_recents();
    state.recents.visibility = 1.;
    // As after the frame that showed it.
    state.dirty = false;
    // The Apps scene would be Recents only, so preparing it now would be wasted.
    let apps_controls = |state: &mut State| {
        let current = std::mem::replace(&mut state.page, Page::Apps);
        let found = controls(state)
            .iter()
            .any(|control| matches!(control.target, Target::App(_)));
        state.page = current;
        found
    };
    assert!(!apps_controls(&mut state));
    assert_eq!(crate::next_prewarm(&mut jobs, &state, true), None);
    assert_eq!(jobs, [Page::Settings, Page::Apps]);
    // Once Recents is gone the retained jobs run, in order.
    state.overlay = None;
    state.recents.visibility = 0.;
    state.dirty = false;
    assert!(apps_controls(&mut state));
    assert_eq!(crate::next_prewarm(&mut jobs, &state, false), None);
    assert_eq!(
        crate::next_prewarm(&mut jobs, &state, true),
        Some(Page::Apps)
    );
    assert_eq!(
        crate::next_prewarm(&mut jobs, &state, true),
        Some(Page::Settings)
    );
    assert_eq!(crate::next_prewarm(&mut jobs, &state, true), None);
}

#[test]
fn failed_first_inventory_keeps_loading_until_retries_are_used_up() {
    let mut state = State::new(Preferences::default());
    state.loading = true;
    for attempt in 1..=crate::state::INVENTORY_RETRIES {
        assert_eq!(
            state.inventory_failed("The TV service timed out".into()),
            Some(crate::state::INVENTORY_RETRY_DELAY)
        );
        assert_eq!(state.inventory_failures, attempt);
        assert!(state.loading && state.error.is_none());
    }
    assert_eq!(
        state.inventory_failed("The TV service timed out".into()),
        None
    );
    assert!(!state.loading);
    assert_eq!(state.error.as_deref(), Some("The TV service timed out"));
    assert_eq!(state.first_content(), Target::Retry);
    // Retry by hand starts a fresh round of automatic retries.
    assert_eq!(state.inventory_failures, 0);
}

#[test]
fn inventory_success_resets_failures_and_later_failures_show_at_once() {
    let mut state = State::new(Preferences::default());
    state.loading = true;
    assert!(state.inventory_failed("timeout".into()).is_some());
    state.set_inventory(demo_inventory(), None);
    assert_eq!(state.inventory_failures, 0);
    state.loading = true;
    // Apps are on screen already: a failed refresh is reported, not retried behind them.
    assert_eq!(state.inventory_failed("timeout".into()), None);
    assert!(!state.loading && state.error.is_some());
    assert!(!state.inventory.apps.is_empty());
}

#[test]
fn tile_urls_need_a_host_the_request_client_accepts() {
    use crate::model::valid_tile_url;
    for url in [
        "http://homeassistant.local:8123/api/webhook/lamp",
        "http://ha/api/webhook/lamp",
        "http://my_nas.lan/hook",
        "https://example.com",
        "HTTPS://example.com/",
        "http://192.168.188.10:8123/api/webhook/lamp?on=1",
        "http://[::1]:8123/api/webhook/lamp",
        "http://[fd00::10]/hook",
        "http://user:secret@ha.local/hook",
        "http://ha.local:65535/",
    ] {
        assert!(valid_tile_url(url), "{url} should be valid");
    }
    for url in [
        "http://",
        "http:///",
        "http:////api",
        "http://:8123/foo",
        "http://user@/foo",
        "http://ha.local:/foo",
        "http://ha.local:0/foo",
        "http://ha.local:65536/foo",
        "http://ha.local:80a/foo",
        "http://ha.local:+80/foo",
        "http://ha.local:80:81/foo",
        "http://ha..local/foo",
        "http://-ha.local/foo",
        "http://ha%2elocal/foo",
        "http://999.1.1.1/foo",
        "http://192.168.1/foo",
        "http://[::1/foo",
        "http://[not-ip]/foo",
        "http://::1/foo",
        "http://ha.local/a b",
        "ftp://ha.local/foo",
    ] {
        assert!(!valid_tile_url(url), "{url} should be invalid");
    }
}

#[test]
fn custom_tiles_are_sanitized_and_get_stable_unique_ids() {
    use crate::model::{TileMethod, new_tile_id, valid_app_id};
    let mut preferences: Preferences = serde_json::from_value(json!({
        "custom_tiles": [
            {"id": "custom.lamp", "title": "  Lamp ", "url": " http://ha.local/api/webhook/a "},
            {"id": "custom.lamp", "title": "Duplicate", "url": "http://ha.local/b"},
            {"id": "custom.get", "title": "Get", "url": "HTTPS://ha.local/c", "method": "GET", "icon": "netflix"},
            {"id": "netflix", "title": "Not custom", "url": "http://ha.local/d"},
            {"id": "custom.ftp", "title": "FTP", "url": "ftp://ha.local/e"},
            {"id": "custom.space", "title": "Space", "url": "http://ha local/f"},
            {"id": "custom.bare", "title": "Bare", "url": "http://"},
            {"id": "custom.blank", "title": "  ", "url": "http://ha.local/g"}
        ]
    }))
    .unwrap();
    preferences.sanitize();
    let tiles: Vec<_> = preferences
        .custom_tiles
        .iter()
        .map(|tile| {
            (
                tile.id.as_str(),
                tile.title.as_str(),
                tile.url.as_str(),
                tile.method,
            )
        })
        .collect();
    assert_eq!(
        tiles,
        [
            (
                "custom.lamp",
                "Lamp",
                "http://ha.local/api/webhook/a",
                TileMethod::Post
            ),
            ("custom.get", "Get", "HTTPS://ha.local/c", TileMethod::Get),
        ]
    );
    // Only interface icons from the list are kept.
    assert!(preferences.custom_tiles[1].icon.is_empty());
    assert_eq!(preferences.custom_tiles[1].icon(), "weather-moon");
    assert_eq!(
        new_tile_id(&preferences.custom_tiles, "Lamp"),
        "custom.lamp-2"
    );
    assert_eq!(
        new_tile_id(&preferences.custom_tiles, "Licht Küche!"),
        "custom.licht-k-che"
    );
    assert_eq!(new_tile_id(&preferences.custom_tiles, "灯"), "custom.tile");
    assert!(valid_app_id(&new_tile_id(
        &preferences.custom_tiles,
        &"x".repeat(200)
    )));
}

#[test]
fn shortcut_settings_offer_qr_editing_and_a_test_remove_popup() {
    use crate::state::{Subpage, WIDGETS_SECTION};
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.open(2);
    state.focus = Target::Widget(2);
    state.activate();
    assert_eq!(state.section, WIDGETS_SECTION);
    assert_eq!(state.subpage, Some(Subpage::Shortcuts));
    assert_eq!(
        controls(&state)
            .into_iter()
            .filter(|c| c.scrollable)
            .map(|c| c.target)
            .collect::<Vec<_>>(),
        vec![Target::TileEditor]
    );
    let tile = crate::model::CustomTile {
        id: "custom.lamp".into(),
        title: "Lamp".into(),
        url: "http://127.0.0.1:9/hook".into(),
        ..Default::default()
    };
    assert_eq!(state.replace_tiles(vec![tile.clone()]), vec![Effect::Save]);
    assert_eq!(state.subpage, Some(Subpage::Shortcuts));
    assert!(!state.text_input_active());
    assert_eq!(
        controls(&state)
            .into_iter()
            .filter(|c| c.scrollable)
            .map(|c| c.target)
            .collect::<Vec<_>>(),
        vec![Target::Tile(tile.id.clone()), Target::TileEditor]
    );
    assert_eq!(
        state.hit_test(&controls(&state), 800., 280.),
        Some(Target::Tile(tile.id.clone()))
    );
    let page = scene(&state);
    assert!(page.draw.iter().any(|draw| matches!(draw, crate::renderer::Draw::Text(label, _, _, color, _) if label == "Lamp" && color[3] == 0.73)));
    assert!(page.draw.iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Shape(rect, radius, _, color)
            if rect.x == 640. && rect.h == 90.7 && *radius == 20. && color[3] == 0.72
    )));
    assert!(
        !page
            .controls
            .iter()
            .any(|control| matches!(control.target, Target::TileRemove(_)))
    );
    assert!(!page.draw.iter().any(|draw| matches!(draw, crate::renderer::Draw::TintedImage(path, rect, ..) if path.ends_with("right.lvec") && rect.x >= 640.)));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Tile(tile.id.clone()));
    state.activate();
    assert_eq!(state.focus, Target::TileTest(tile.id.clone()));
    let popup = scene(&state);
    assert!(popup.modal.is_some());
    assert_eq!(
        popup
            .controls
            .iter()
            .map(|control| control.target.clone())
            .collect::<Vec<_>>(),
        vec![
            Target::TileTest(tile.id.clone()),
            Target::TileRemove(tile.id.clone())
        ]
    );
    assert!(
        matches!(state.activate().as_slice(), [Effect::FireWebhook(sent)] if sent.id == tile.id)
    );
    state.back();
    assert_eq!(state.focus, Target::Tile(tile.id.clone()));
    state.activate();
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::TileRemove(tile.id.clone()));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.custom_tiles.is_empty());
    assert!(
        !state
            .preferences
            .favorites(&state.inventory.apps)
            .contains(&tile.id)
    );
    assert!(!state.inventory.apps.iter().any(|app| app.id == tile.id));
    assert_eq!(state.subpage, Some(Subpage::Shortcuts));
    assert_eq!(state.focus, Target::TileEditor);
    state.back();
    assert_eq!(state.subpage, None);
    assert_eq!(state.focus, Target::Widget(2));
}

#[test]
fn widgets_group_existing_settings_and_restore_each_parent_without_changing_preferences() {
    use crate::state::{SECTION_ORDER, Subpage, WIDGETS_SECTION};
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.preferences.greeting_name = "Ana".into();
    state.preferences.show_weather = false;
    state.preferences.weather_location = Some(crate::weather::Location {
        name: "Vienna".into(),
        region: "Vienna".into(),
        country: "Austria".into(),
        latitude: 48.2085,
        longitude: 16.3721,
    });
    state.replace_tiles(vec![crate::model::CustomTile {
        id: "custom.lamp".into(),
        title: "Lamp".into(),
        url: "http://127.0.0.1:9/hook".into(),
        icon: "power".into(),
        ..Default::default()
    }]);
    let saved = serde_json::to_value(&state.preferences).unwrap();
    state.open(2);
    assert_eq!(SECTION_ORDER.len(), 7);
    assert!(
        !controls(&state)
            .iter()
            .any(|control| control.target == Target::Setting(6))
    );
    for _ in 0..5 {
        state.direction(Direction::Down, &controls(&state));
    }
    assert_eq!(state.focus, Target::Sidebar(WIDGETS_SECTION));
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::Widget(4));
    let widget_order: Vec<_> = controls(&state)
        .into_iter()
        .filter_map(|control| match control.target {
            Target::Widget(index) => Some(index),
            _ => None,
        })
        .collect();
    assert_eq!(widget_order, [4, 0, 1, 2, 3]);
    for (index, subpage) in [
        Subpage::Greeting,
        Subpage::Weather,
        Subpage::Shortcuts,
        Subpage::Inputs,
    ]
    .into_iter()
    .enumerate()
    {
        state.focus = Target::Widget(index);
        state.direction(Direction::Right, &controls(&state));
        assert_eq!(state.subpage, Some(subpage));
        state.back();
        assert!(state.subpage.is_none());
        assert_eq!(state.focus, Target::Widget(index));
    }
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Sidebar(WIDGETS_SECTION));
    assert_eq!(serde_json::to_value(&state.preferences).unwrap(), saved);
    state.inventory.inputs.clear();
    state.focus = Target::Widget(3);
    state.activate();
    assert_eq!(state.subpage, Some(Subpage::Inputs));
    assert_eq!(state.focus, Target::Setting(3));
    assert_eq!(state.activate(), vec![Effect::Reload]);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Widget(3));
}

#[test]
fn companion_removal_updates_the_shortcut_list_on_close() {
    use crate::state::Subpage;
    let mut state = ready();
    let tile = crate::model::CustomTile {
        id: "custom.lamp".into(),
        title: "Lamp".into(),
        url: "http://127.0.0.1:9/hook".into(),
        ..Default::default()
    };
    state.replace_tiles(vec![tile.clone()]);
    state.open(2);
    state.focus = Target::Widget(2);
    state.activate();
    state.activate();
    assert!(matches!(state.overlay, Some(Overlay::TileEditor)));
    state.replace_tiles(vec![]);
    assert_eq!(state.subpage, Some(Subpage::Shortcuts));
    assert_eq!(state.focus, Target::CloseDialog);
    state.back();
    assert_eq!(state.focus, Target::TileEditor);
    assert!(
        !state
            .preferences
            .favorites(&state.inventory.apps)
            .contains(&tile.id)
    );
    state.back();
    assert_eq!(state.focus, Target::Widget(2));
}

#[test]
fn widgets_are_localized_without_legacy_sidebar_entries() {
    use crate::state::WIDGETS_SECTION;
    for language in crate::i18n::Language::ALL {
        let mut state = ready();
        state.preferences.language = language;
        state.open(2);
        state.pointer_focus(Target::Sidebar(WIDGETS_SECTION));
        // Hover only highlights the section; the click opens it.
        assert_ne!(state.section, WIDGETS_SECTION);
        state.activate();
        let page = scene(&state);
        assert_eq!(
            page.controls
                .iter()
                .filter(|c| matches!(c.target, Target::Sidebar(_)))
                .count(),
            7
        );
        assert_eq!(
            page.controls
                .iter()
                .filter(|c| matches!(c.target, Target::Widget(_)))
                .count(),
            5
        );
        for key in [
            "widgets",
            "greeting",
            "weather",
            "shortcuts",
            "inputs",
            "date",
        ] {
            assert!(page.draw.iter().any(|draw| matches!(draw, crate::renderer::Draw::Text(label, ..) if label == language.t(key))), "{key}/{language:?}");
        }
        state.pointer_focus(Target::Sidebar(0));
        state.activate();
        assert!(!scene(&state).draw.iter().any(|draw| matches!(draw, crate::renderer::Draw::Text(label, ..) if label == language.t("greeting"))));
    }
}

#[test]
fn shortcut_list_scroll_and_pointer_focus_preserve_control_positions() {
    let mut state = ready();
    state.replace_tiles(
        (0..20)
            .map(|index| crate::model::CustomTile {
                id: format!("custom.tile-{index}"),
                title: format!("Tile {index}"),
                url: "http://127.0.0.1:9/hook".into(),
                ..Default::default()
            })
            .collect(),
    );
    state.open(2);
    state.focus = Target::Widget(2);
    state.activate();
    for _ in 0..20 {
        state.direction(Direction::Down, &controls(&state));
    }
    assert_eq!(state.focus, Target::Tile("custom.tile-19".into()));
    assert!(state.scroll > 0.);
    let before: Vec<_> = controls(&state)
        .iter()
        .map(|c| (c.target.clone(), c.rect.y))
        .collect();
    let scroll = state.scroll;
    let target = controls(&state)
        .into_iter()
        .find(|c| matches!(c.target, Target::Tile(_)) && c.rect.y > state.content_viewport().y)
        .unwrap();
    state.pointer_focus(target.target);
    assert_eq!(state.scroll, scroll);
    assert_eq!(
        before,
        controls(&state)
            .iter()
            .map(|c| (c.target.clone(), c.rect.y))
            .collect::<Vec<_>>()
    );
}

#[test]
fn settings_lists_keep_bottom_padding_for_remote_and_wheel_scrolling() {
    use crate::state::{SETTINGS_LIST_BOTTOM_PADDING, Subpage};
    for backgrounds in [false, true] {
        let mut state = ready();
        state.preferences.reduced_motion = true;
        state.open(2);
        if backgrounds {
            state.set_wallpapers(
                (1..=8)
                    .map(|index| crate::wallpapers::Wallpaper {
                        collection: None,
                        credit: None,
                        id: format!("custom-{index:032x}"),
                        directory: std::path::PathBuf::from(".build/padding-fixture"),
                        unsplash: None,
                        has_original: true,
                    })
                    .collect(),
            );
        }
        state.section = if backgrounds { 1 } else { 0 };
        state.subpage = if backgrounds {
            None
        } else {
            Some(Subpage::Language)
        };
        let last = if backgrounds {
            Target::Background(state.background_count() - 1)
        } else {
            Target::Choice(crate::i18n::Language::ALL.len() - 1)
        };
        state.focus = last.clone();
        state.reveal_focus(&controls(&state));
        let bottom =
            state.content_viewport().y + state.content_viewport().h - SETTINGS_LIST_BOTTOM_PADDING;
        let rect = controls(&state)
            .into_iter()
            .find(|c| c.target == last)
            .unwrap()
            .rect;
        assert!(rect.y + rect.h <= bottom + 0.01);
        state.wheel_scroll(&controls(&state), Some((1000., 600.)), 0, -1000);
        let rect = controls(&state)
            .into_iter()
            .find(|c| c.target == last)
            .unwrap()
            .rect;
        assert!((rect.y + rect.h - bottom).abs() < 0.01);
    }
}

#[test]
fn language_list_scrolls_under_fixed_ui_without_clipping_or_fades() {
    use crate::{renderer::Draw, state::Subpage};
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.open(2);
    state.subpage = Some(Subpage::Language);
    state.scroll = 180.;
    let rendered = scene(&state);
    assert!(!rendered.draw.iter().any(|draw| matches!(
        draw,
        Draw::Clip(Some(_)) | Draw::EdgeFade(Some(_)) | Draw::EdgeFadeX(Some(_))
    )));
    let first = rendered
        .controls
        .iter()
        .find(|control| control.target == Target::Choice(0))
        .unwrap();
    assert!(first.rect.y < state.content_viewport().y);
    let last_language = rendered
        .draw
        .iter()
        .rposition(|draw| {
            matches!(draw,
                Draw::Text(label, ..) if label == crate::i18n::Language::ALL.last().unwrap().name()
            )
        })
        .unwrap();
    let label = state.preferences.language.t("settingsHome");
    let fixed = rendered
        .draw
        .iter()
        .position(|draw| {
            matches!(draw,
                Draw::Text(text, ..) if text == label
            )
        })
        .unwrap();
    assert!(fixed > last_language);
    let nav = rendered
        .controls
        .iter()
        .find(|control| control.target == Target::Nav(2))
        .unwrap();
    let (x, y) = nav.rect.center();
    assert_eq!(
        state.hit_test(&rendered.controls, x, y),
        Some(Target::Nav(2))
    );
}

#[test]
fn apps_scroll_behind_navigation_without_a_top_cutoff_or_fade() {
    use crate::renderer::Draw;
    let mut state = ready();
    state.open(1);
    state.scroll = 400.;
    let rendered = scene(&state);
    assert!(!rendered.draw.iter().any(|draw| matches!(
        draw,
        Draw::Clip(Some(_)) | Draw::EdgeFade(Some(_)) | Draw::EdgeFadeX(Some(_))
    )));
    let card = rendered
        .draw
        .iter()
        .position(|draw| {
            matches!(draw,
                Draw::Shadow(rect, ..) if rect.y < 0. && rect.y + rect.h > 0.
            )
        })
        .expect("cards remain drawn until they leave the screen, not the content viewport");
    // The header clock is drawn over the scrolled cards; the drawer has no top buttons.
    let clock = rendered
        .draw
        .iter()
        .rposition(|draw| matches!(draw, Draw::Text(_, rect, ..) if rect.y < 120. && rect.x == 84.))
        .unwrap();
    assert!(card < clock);
    assert!(
        !rendered
            .controls
            .iter()
            .any(|control| matches!(control.target, Target::Nav(_) | Target::TvSettings))
    );
}

#[test]
fn apps_toolbar_stays_fixed_above_scrolling_cards_and_keeps_pointer_priority() {
    for with_clear_controls in [false, true] {
        let mut state = ready();
        state.open(1);
        if with_clear_controls {
            state.filter = 1;
            state.query = "n".into();
        }
        let before = controls(&state);
        state.scroll = 200.;
        let rendered = scene(&state);
        for control in before.iter().filter(|control| {
            matches!(
                control.target,
                Target::Filter(_) | Target::ClearFilter | Target::Search | Target::ClearSearch
            )
        }) {
            let fixed = rendered
                .controls
                .iter()
                .find(|item| item.target == control.target)
                .unwrap();
            assert_eq!(fixed.rect.y, control.rect.y);
            assert!(!fixed.scrollable);
            let (x, y) = fixed.rect.center();
            assert_eq!(
                state.hit_test(&rendered.controls, x, y),
                Some(fixed.target.clone())
            );
        }
        assert_eq!(state.hit_test(&rendered.controls, 1400., 230.), None);
        let first = before
            .iter()
            .find(|control| matches!(control.target, Target::App(_)))
            .unwrap();
        let scrolled = rendered
            .controls
            .iter()
            .find(|control| control.target == first.target)
            .unwrap();
        assert_eq!(first.rect.y - scrolled.rect.y, state.scroll);
        state.focus = first.target.clone();
        state.reveal_focus(&rendered.controls);
        let revealed = controls(&state)
            .into_iter()
            .find(|control| control.target == first.target)
            .unwrap();
        assert!(revealed.rect.y >= state.apps_viewport().y);
    }
}

#[test]
fn app_categories_and_search_use_matching_pills_and_larger_heavier_labels() {
    use crate::renderer::Draw;
    let mut state = ready();
    state.open(1);
    let rendered = scene(&state);
    for control in rendered
        .controls
        .iter()
        .filter(|control| matches!(control.target, Target::Filter(_) | Target::Search))
    {
        assert_eq!(control.rect.h, 56.);
        assert_eq!(control.rect.y, 164.8);
        assert!(rendered.draw.iter().any(|draw| matches!(draw,
            Draw::Shape(rect, radius, _, color) if rect.x == control.rect.x && rect.y == control.rect.y
                && rect.w == control.rect.w && (*radius == 48. || *radius == 28.) && color[3] >= 0.72
        )));
        assert!(rendered.draw.iter().any(|draw| matches!(draw,
            Draw::Text(_, rect, style, ..) if rect.x >= control.rect.x && rect.x < control.rect.x + control.rect.w
                && rect.y > control.rect.y && rect.y < control.rect.y + control.rect.h
                && style.size == 20.8 && style.weight == 550
        )));
    }
}

#[test]
fn category_focus_active_and_clear_button_share_the_pill_style() {
    use crate::renderer::Draw;
    let mut state = ready();
    state.open(1);
    state.filter = 1;
    state.focus = Target::ClearFilter;
    let rendered = scene(&state);
    let clear = rendered
        .controls
        .iter()
        .find(|control| control.target == Target::ClearFilter)
        .unwrap();
    assert_eq!((clear.rect.w, clear.rect.h), (56., 56.));
    assert!(rendered.draw.iter().any(|draw| matches!(draw,
        Draw::RoundedGradient(rect, radius, ..) if rect.x == clear.rect.x && rect.y == clear.rect.y && *radius == 28.
    )));
    let category = rendered
        .controls
        .iter()
        .find(|control| control.target == Target::Filter(1))
        .unwrap();
    assert!(rendered.draw.iter().any(|draw| matches!(draw,
        Draw::Shape(rect, radius, stroke, color) if rect.x == category.rect.x && rect.y == category.rect.y
            && *radius == 28. && *stroke == 1.5 && color[3] == 0.35
    )));
    state.focus = Target::Filter(1);
    let focused = scene(&state);
    assert!(focused.draw.iter().any(|draw| matches!(draw,
        Draw::RoundedGradient(rect, radius, ..) if rect.x == category.rect.x && rect.y == category.rect.y && *radius == 28.
    )));
}

#[test]
fn clock_date_and_home_input_descriptions_use_larger_text() {
    use crate::renderer::Draw;
    for language in crate::i18n::Language::ALL {
        let mut state = ready();
        state.preferences.language = language;
        let rendered = scene(&state);
        let now = time::OffsetDateTime::UNIX_EPOCH;
        for (label, size) in [
            (
                language.clock(
                    now,
                    state.preferences.hour_24,
                    state.preferences.show_seconds,
                ),
                24.,
            ),
            (language.date(now), 18.4),
        ] {
            assert!(rendered.draw.iter().any(|draw| matches!(draw,
                Draw::Text(text, rect, style, ..) if text == &label && style.size == size && rect.h >= size
            )));
        }
        let input = &state.inventory.inputs[0];
        let description = view::input_status(
            input,
            state.preferences.input_name(input).is_some(),
            language,
        );
        assert!(rendered.draw.iter().any(|draw| matches!(draw,
            Draw::Text(text, rect, style, ..) if text == &description && style.size == 16.8 && rect.h == 25.2
        )));
    }
}

#[test]
fn the_gear_sits_alone_at_the_top_right_and_opens_the_quick_panel() {
    use crate::model::QuickKey;
    let mut state = ready();
    let rendered = scene(&state);
    let gear = rendered
        .controls
        .iter()
        .find(|control| control.target == Target::Nav(2))
        .unwrap();
    assert!((gear.rect.y + gear.rect.h / 2. - (42.5 + 68.6 / 2.)).abs() < 0.01);
    assert_eq!((gear.rect.w, gear.rect.h), (60., 60.));
    assert_eq!(gear.rect.x + gear.rect.w, 1836.);
    let (x, y) = gear.rect.center();
    assert_eq!(
        state.hit_test(&rendered.controls, x, y),
        Some(Target::Nav(2))
    );
    // TV settings moved into the panel; opening it changes nothing on the TV.
    assert!(
        !rendered
            .controls
            .iter()
            .any(|c| c.target == Target::TvSettings)
    );
    state.focus = Target::Nav(2);
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::Quick(QuickKey::Ambient));
    let panel: Vec<_> = scene(&state)
        .controls
        .into_iter()
        .map(|c| c.target)
        .collect();
    assert!(panel.contains(&Target::Quick(QuickKey::TvSettings)));
    assert!(panel.contains(&Target::QuickAll));
    assert!(
        !panel.contains(&Target::Quick(QuickKey::Inputs)),
        "Inputs follows its own setting, off by default"
    );
    // Spatial moves inside the panel; Back closes it onto the gear.
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.focus, Target::Quick(QuickKey::Collections));
    state.focus = Target::Quick(QuickKey::TvSettings);
    assert_eq!(
        state.activate(),
        vec![Effect::Launch("com.palm.app.settings".into())]
    );
    assert!(state.overlay.is_none());
    state.open_quick();
    state.back();
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::Nav(2));
}

#[test]
fn quick_panel_starts_ambient_and_its_buttons_switch_off() {
    use crate::model::QuickKey;
    let mut state = ready();
    // Ambient starts even with its idle timer off, and any key wakes it as usual.
    state.preferences.ambient_minutes = 0;
    state.open_quick();
    state.focus = Target::Quick(QuickKey::Ambient);
    assert!(state.activate().is_empty());
    assert!(state.overlay.is_none());
    state.ambient_tick(true);
    assert!(state.ambient.is_some());
    assert!(state.note_input());
    // Each button can be switched off in Settings → General → Quick settings.
    state.preferences.toggle_quick(QuickKey::Ambient);
    assert!(!state.quick_keys().contains(&QuickKey::Ambient));
    state.preferences.toggle_quick(QuickKey::Ambient);
    assert!(state.quick_keys().contains(&QuickKey::Ambient));
}

#[test]
fn greeting_visibility_toggle_persists_without_clearing_name_or_hiding_date() {
    use crate::{renderer::Draw, state::Subpage};
    let legacy: Preferences = serde_json::from_str(r#"{"greeting_name":"Ana"}"#).unwrap();
    assert!(legacy.show_greeting);
    let mut state = ready();
    state.preferences = legacy;
    state.open(2);
    state.focus = Target::Widget(0);
    state.activate();
    assert_eq!(state.subpage, Some(Subpage::Greeting));
    assert_eq!(state.focus, Target::GreetingToggle);
    let rendered = scene(&state);
    let toggle = rendered
        .controls
        .iter()
        .find(|control| control.target == Target::GreetingToggle)
        .unwrap();
    assert_eq!(toggle.rect.y, 164.8);
    assert!(rendered.draw.iter().any(|draw| matches!(draw,
        Draw::Text(label, ..) if label == state.preferences.language.t("showWidget")
    )));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.show_greeting);
    assert_eq!(state.preferences.greeting_name, "Ana");
    let saved = serde_json::to_string(&state.preferences).unwrap();
    let restored: Preferences = serde_json::from_str(&saved).unwrap();
    assert!(!restored.show_greeting);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::GreetingName);
    state.focus = Target::GreetingToggle;
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::Widget(0));
    state.open(0);
    let hidden = scene(&state);
    let language = state.preferences.language;
    let now = time::OffsetDateTime::UNIX_EPOCH;
    let greeting = language.greeting(now.hour(), "Ana");
    assert!(
        !hidden
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Text(label, ..) if label == &greeting))
    );
    assert!(
        hidden
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Text(label, ..) if label == &language.date(now)))
    );
    state.open(2);
    state.focus = Target::Widget(0);
    state.activate();
    state.focus = Target::GreetingToggle;
    state.activate();
    assert!(state.preferences.show_greeting);
    state.open(0);
    assert!(
        scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, Draw::Text(label, ..) if label == &greeting))
    );
}

#[test]
fn date_widget_settings_persist_and_restore_parent_focus() {
    use crate::{model::DateFormat, state::Subpage};
    let legacy: Preferences = serde_json::from_str("{}").unwrap();
    assert!(legacy.show_date);
    assert_eq!(legacy.date_format, DateFormat::Localized);
    let mut state = ready();
    state.open(2);
    state.focus = Target::Widget(4);
    state.activate();
    assert_eq!(state.subpage, Some(Subpage::Date));
    assert_eq!(state.focus, Target::DateToggle);
    let before = controls(&state);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.show_date);
    let after = controls(&state);
    for (a, b) in before.iter().zip(&after) {
        assert_eq!(a.target, b.target);
        assert_eq!((a.rect.x, a.rect.y), (b.rect.x, b.rect.y));
    }
    state.direction(Direction::Down, &after);
    assert_eq!(state.focus, Target::DateFormat);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.subpage, Some(Subpage::DateFormat));
    assert_eq!(state.focus, Target::Choice(0));
    for index in 1..4 {
        state.direction(Direction::Down, &controls(&state));
        assert_eq!(state.focus, Target::Choice(index));
    }
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::Choice(0));
    for _ in 0..3 {
        state.direction(Direction::Down, &controls(&state));
    }
    assert_eq!(state.focus, Target::Choice(3));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.date_format, DateFormat::YearMonthDay);
    assert_eq!(state.subpage, Some(Subpage::Date));
    assert_eq!(state.focus, Target::DateFormat);
    state.activate();
    assert_eq!(state.focus, Target::Choice(3));
    state.back();
    assert_eq!(state.focus, Target::DateFormat);
    state.back();
    assert_eq!(state.focus, Target::Widget(4));
    let saved: Preferences =
        serde_json::from_str(&serde_json::to_string(&state.preferences).unwrap()).unwrap();
    assert!(!saved.show_date);
    assert_eq!(saved.date_format, DateFormat::YearMonthDay);
    state.open(0);
    let date = saved
        .date_format
        .format(saved.language, time::OffsetDateTime::UNIX_EPOCH);
    assert!(
        !scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, crate::renderer::Draw::Text(label, ..) if label == &date))
    );
}

#[test]
fn date_formats_preserve_localized_default_and_pad_numeric_dates() {
    use crate::{i18n::Language, model::DateFormat};
    let now = time::Date::from_calendar_date(2026, time::Month::February, 3)
        .unwrap()
        .midnight()
        .assume_utc();
    for language in Language::ALL {
        assert_eq!(
            DateFormat::Localized.format(language, now),
            language.date(now)
        );
        assert_eq!(DateFormat::DayMonthYear.format(language, now), "03.02.2026");
        assert_eq!(DateFormat::MonthDayYear.format(language, now), "02/03/2026");
        assert_eq!(DateFormat::YearMonthDay.format(language, now), "2026-02-03");
        for key in ["date", "dateFormat", "dateLocalized"] {
            assert_ne!(language.t(key), key);
        }
    }
}

#[test]
fn update_card_shows_both_versions_and_explicitly_starts_installation() {
    use crate::{i18n::Language, renderer::Draw};
    for language in Language::ALL {
        let mut state = ready();
        state.preferences.language = language;
        state.open(2);
        state.section = crate::state::SYSTEM_SECTION;
        assert!(
            !controls(&state)
                .iter()
                .any(|control| control.target == Target::InstallUpdate)
        );
        state.available_update = Some("9.0.0".into());
        let rendered = scene(&state);
        for label in [
            language.t("updateAvailable").to_owned(),
            language.format("installUpdate", &[("version", "9.0.0")]),
            language.format(
                "updateVersions",
                &[("current", "test"), ("latest", "9.0.0")],
            ),
        ] {
            assert!(
                rendered
                    .draw
                    .iter()
                    .any(|draw| matches!(draw, Draw::Text(text, ..) if text == &label))
            );
        }
        state.focus = Target::Sidebar(crate::state::SYSTEM_SECTION);
        state.direction(Direction::Right, &rendered.controls);
        assert_eq!(state.focus, Target::InstallUpdate);
        assert_eq!(
            state.activate(),
            vec![Effect::UpdateDownload("9.0.0".into())]
        );
        let button = rendered
            .controls
            .iter()
            .find(|control| control.target == Target::InstallUpdate)
            .unwrap();
        let (x, y) = button.rect.center();
        assert_eq!(
            state.hit_test(&rendered.controls, x, y),
            Some(Target::InstallUpdate)
        );
        state.direction(Direction::Down, &rendered.controls);
        assert_eq!(state.focus, Target::Setting(4)); // Language, then Reduced motion,
        state.direction(Direction::Down, &rendered.controls);
        assert_eq!(state.focus, Target::Setting(2));
        state.direction(Direction::Down, &rendered.controls);
        assert_eq!(state.focus, Target::Setting(7)); // then Text rendering, before maintenance actions.
        state.direction(Direction::Down, &rendered.controls);
        assert_eq!(state.focus, Target::Setting(3)); // Refresh apps & inputs.
        state.focus = Target::InstallUpdate;
        state.direction(Direction::Left, &rendered.controls);
        assert_eq!(state.focus, Target::Sidebar(crate::state::SYSTEM_SECTION));
    }
}

#[test]
fn update_status_card_only_claims_latest_after_success_and_allows_single_flight_retry() {
    use crate::{renderer::Draw, update::Status};
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    state.focus = Target::CheckUpdates;
    assert_eq!(state.update_status, Status::Unchecked);
    assert!(!scene(&state).draw.iter().any(|draw| matches!(draw,
        Draw::Text(label, ..) if label == state.preferences.language.t("latestVersionInstalled")
    )));
    assert!(state.activate().is_empty());
    assert_eq!(state.update_status, Status::Checking);
    assert!(std::mem::take(&mut state.update_check_requested));
    state.activate();
    assert!(!state.update_check_requested);
    state.update_response(Err("offline".into()));
    assert_eq!(state.update_status, Status::Failed);
    assert!(!scene(&state).draw.iter().any(|draw| matches!(draw,
        Draw::Text(label, ..) if label == state.preferences.language.t("latestVersionInstalled")
    )));
    state.activate();
    assert!(std::mem::take(&mut state.update_check_requested));
    state.update_response(Ok(None));
    assert_eq!(state.update_status, Status::Checked);
    for key in ["latestVersionInstalled", "checkAgain"] {
        assert!(scene(&state).draw.iter().any(|draw| matches!(draw,
            Draw::Text(label, ..) if label == state.preferences.language.t(key)
        )));
    }
    state.activate();
    state.update_response(Ok(Some("9.0.0".into())));
    assert_eq!(state.focus, Target::InstallUpdate);
    state.update_response(Err("offline".into()));
    assert_eq!(state.available_update.as_deref(), Some("9.0.0"));
}

#[test]
fn shortcut_tiles_keep_apps_options_and_never_launch_or_uninstall_a_platform_app() {
    let tile = crate::model::CustomTile {
        id: "custom.lamp".into(),
        title: "Lamp".into(),
        url: "http://127.0.0.1:9/hook".into(),
        ..Default::default()
    };
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.replace_tiles(vec![tile.clone()]);
    state.open(0);
    state.focus = Target::App(tile.id.clone());
    assert_eq!(state.activate(), vec![Effect::FireWebhook(tile.clone())]);
    state.options();
    assert_eq!(state.activate(), vec![Effect::FireWebhook(tile.clone())]);

    state.open(1);
    state.focus = Target::App(tile.id.clone());
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::OpenApp);
    assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
    assert!(
        !controls(&state)
            .iter()
            .any(|c| c.target == Target::Option(2))
    );
    assert_eq!(state.activate(), vec![Effect::FireWebhook(tile.clone())]);
    assert!(state.overlay.is_none());

    state.focus = Target::App(tile.id.clone());
    state.options();
    state.focus = Target::Option(0);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(
        !state
            .preferences
            .favorites(&state.inventory.apps)
            .contains(&tile.id)
    );
    state.options();
    state.focus = Target::Option(1);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.hidden.contains(&tile.id));
    assert_eq!(state.preferences.custom_tile(&tile.id), Some(&tile));
}

#[test]
fn shortcut_tiles_alone_do_not_stop_inventory_retries() {
    let mut preferences: Preferences = serde_json::from_value(json!({
        "custom_tiles": [{"id": "custom.lamp", "title": "Lamp", "url": "http://ha.local/hook"}]
    }))
    .unwrap();
    preferences.sanitize();
    let mut state = State::new(preferences);
    state.loading = true;
    state.sync_custom_tiles();
    assert!(!state.inventory.apps.is_empty());
    assert_eq!(
        state.inventory_failed("timeout".into()),
        Some(crate::state::INVENTORY_RETRY_DELAY)
    );
    assert!(state.loading && state.error.is_none());
}

#[test]
fn shortcut_tiles_open_a_modal_editor_for_another_device() {
    use crate::state::WIDGETS_SECTION;
    let mut state = ready();
    state.preferences.reduced_motion = true;
    state.open(2);
    state.focus = Target::Widget(2);
    state.activate();
    assert_eq!(state.focus, Target::TileEditor);
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::TileEditor)));
    assert_eq!(state.focus, Target::CloseDialog);
    let modal = scene(&state);
    assert!(modal.modal.is_some());
    assert_eq!(modal.controls.len(), 1);
    assert_eq!(modal.controls[0].target, Target::CloseDialog);
    assert!(state.direction(Direction::Up, &modal.controls).is_empty());
    assert_eq!(state.focus, Target::CloseDialog);
    state.tile_editor = Some(Err("offline".into()));
    state.activate();
    assert!(state.overlay.is_none());
    assert!(state.tile_editor.is_none());
    assert_eq!(state.focus, Target::TileEditor);
    assert_eq!(state.section, WIDGETS_SECTION);
}

#[test]
fn tiles_edited_elsewhere_join_and_leave_home_like_tv_edits() {
    use crate::model::CustomTile;
    let tile = |id: &str, title: &str| CustomTile {
        id: id.into(),
        title: title.into(),
        url: "http://10.0.0.2/hook".into(),
        ..Default::default()
    };
    let mut state = ready();
    let before = state.preferences.favorites(&state.inventory.apps);
    let saved = state.replace_tiles(vec![
        tile("custom.gate", "Gate"),
        tile("custom.lights", "Lights"),
    ]);
    assert_eq!(saved, vec![Effect::Save]);
    let favorites = state.preferences.favorites.clone().unwrap();
    assert_eq!(&favorites[..before.len()], &before[..]);
    assert_eq!(&favorites[before.len()..], ["custom.gate", "custom.lights"]);
    assert!(
        state
            .inventory
            .apps
            .iter()
            .any(|app| app.id == "custom.gate")
    );
    state.preferences.hidden.push("custom.lights".into());
    // A rename keeps the favorite in place; a removal clears it from favorites and hidden.
    state.replace_tiles(vec![tile("custom.gate", "Garage")]);
    assert_eq!(state.preferences.custom_tiles[0].title, "Garage");
    let favorites = state.preferences.favorites.clone().unwrap();
    assert_eq!(&favorites[before.len()..], ["custom.gate"]);
    assert!(state.preferences.hidden.is_empty());
    assert!(
        !state
            .inventory
            .apps
            .iter()
            .any(|app| app.id == "custom.lights")
    );
    let app = state
        .inventory
        .apps
        .iter()
        .find(|app| app.id == "custom.gate")
        .unwrap();
    assert_eq!(app.title, "Garage");
}

#[test]
fn update_card_shows_progress_restores_retry_and_ignores_presses_while_busy() {
    use crate::update::{InstallStatus, Response, Status};
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    state.focus = Target::CheckUpdates;
    assert!(state.activate().is_empty());
    assert_eq!(state.update_status, Status::Checking);
    assert!(
        state.activate().is_empty(),
        "no second check while one runs"
    );
    state.update_response(Ok(Some("9.9.9".into())));
    assert!(scene(&state).draw.iter().any(|draw| {
        matches!(draw, crate::renderer::Draw::Text(text, ..) if text == "Install 9.9.9")
    }));
    let initial = scene(&state);
    assert!(
        !initial
            .controls
            .iter()
            .any(|control| control.target == Target::Setting(8))
    );
    assert_eq!(
        state.activate(),
        vec![Effect::UpdateDownload("9.9.9".into())]
    );
    assert_eq!(state.update, InstallStatus::Downloading(0));
    assert!(state.activate().is_empty());
    state.install_response(Response::Progress(45));
    assert!(scene(&state).draw.iter().any(|draw| {
        matches!(draw, crate::renderer::Draw::Text(text, ..) if text == "Downloading 45 %")
    }));
    let geometry = |items: Vec<crate::state::Control>| {
        items
            .into_iter()
            .map(|item| {
                (
                    item.target,
                    item.rect.x,
                    item.rect.y,
                    item.rect.w,
                    item.rect.h,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        geometry(controls(&state)),
        geometry(initial.controls),
        "progress must not move hit targets"
    );
    state.install_response(Response::Installing("9.9.9".into()));
    assert!(state.activate().is_empty());
    state.pressed = Some(Target::InstallUpdate);
    state.install_response(Response::Finished(Err("disk full".into())));
    assert_eq!(state.focus, Target::CheckUpdates);
    assert!(state.pressed.is_none());
    assert!(state.available_update.is_none());
    assert!(!state.update_busy());
    state.activate();
    assert_eq!(state.update_status, Status::Checking);
    assert_eq!(state.update, InstallStatus::Idle);
    state.install_response(Response::Finished(Ok(())));
    assert_eq!(state.update_status, Status::Unchecked);
}

#[test]
fn inputs_stay_off_until_chosen_then_switch_from_the_quick_panel() {
    use crate::model::QuickKey;
    let mut state = ready();
    let has_button = |state: &State| state.quick_keys().contains(&QuickKey::Inputs);
    // Off by default: no Inputs button, and Up from a favorite on the right goes to the gear.
    assert!(!state.preferences.inputs_menu);
    assert!(!has_button(&state));
    let right = state.apps()[5].id.clone();
    state.focus = Target::App(right.clone());
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Nav(2));

    // Settings → Widgets → Inputs switches it on; Left leaves the page as from its other rows.
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Inputs);
    state.focus = Target::InputsMenu;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.inputs_menu);
    state.direction(Direction::Left, &controls(&state));
    assert!(state.subpage.is_none());

    // Home's cards and the menu are separate: every card switched Off still leaves the menu
    // listing Live TV, then plugged-in inputs, then the rest.
    for input in [
        "com.webos.app.hdmi1",
        "com.webos.app.hdmi2",
        "com.webos.app.hdmi3",
    ] {
        state
            .preferences
            .input_modes
            .insert(input.into(), InputMode::Off);
    }
    assert_eq!(
        state.input_choices(),
        vec![
            "com.webos.app.livetv",
            "com.webos.app.hdmi1",
            "com.webos.app.hdmi2",
            "com.webos.app.hdmi3"
        ]
    );
    // Each input's page has its own menu switch, below Rename and Icon, while the menu is on.
    state.subpage = Some(crate::state::Subpage::Input("com.webos.app.hdmi2".into()));
    state.focus = Target::InputRename;
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::InputIcon);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::InputInMenu);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(
        state.input_choices(),
        vec![
            "com.webos.app.livetv",
            "com.webos.app.hdmi1",
            "com.webos.app.hdmi3"
        ]
    );
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.subpage, Some(crate::state::Subpage::Inputs));
    state.preferences.inputs_menu = false;
    state.subpage = Some(crate::state::Subpage::Input("com.webos.app.hdmi2".into()));
    assert!(
        !controls(&state)
            .iter()
            .any(|control| control.target == Target::InputInMenu),
        "the switch only shows while the menu is on"
    );
    state.preferences.inputs_menu = true;

    state.open(0);
    assert!(has_button(&state));
    // The gear's quick panel has the Inputs button; OK opens the list on its first row.
    state.open_quick();
    state.focus = Target::Quick(QuickKey::Inputs);
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::Inputs)));
    assert_eq!(
        state.focus,
        Target::InputPick("com.webos.app.livetv".into())
    );
    let menu = controls(&state);
    assert!(
        menu.iter()
            .all(|control| matches!(control.target, Target::InputPick(_))),
        "only the list takes input while it is open"
    );
    state.direction(Direction::Down, &menu);
    assert_eq!(state.focus, Target::InputPick("com.webos.app.hdmi1".into()));
    // Back returns to the quick panel it came from.
    assert!(state.back().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::Quick)));
    assert_eq!(state.focus, Target::Quick(QuickKey::Inputs));

    // OK on a row switches to it and closes everything.
    state.activate();
    state.direction(Direction::Down, &controls(&state));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(
        state.activate(),
        vec![Effect::Launch("com.webos.app.hdmi3".into())]
    );
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, Target::Nav(2));
}

#[test]
fn the_wallpaper_first_home_lowers_the_rows_until_focus_moves_into_them() {
    use crate::state::{LANDING_DROP, LANDING_ROW};
    let tile_y = |state: &State| {
        controls(state)
            .iter()
            .find(|control| matches!(control.target, Target::App(_)))
            .map(|control| control.rect.y)
            .unwrap()
    };
    let mut usual = ready();
    usual.preferences.reduced_motion = true;
    let usual_y = tile_y(&usual);

    let mut state = State::new(Preferences {
        landing_layout: true,
        reduced_motion: true,
        ..Preferences::default()
    });
    state.note_input();
    state.set_inventory(demo_inventory(), None);
    assert_eq!(state.focus, Target::Nav(0));
    assert_eq!(state.landing(), 1.);
    assert_eq!(tile_y(&state), usual_y + LANDING_DROP);
    state.direction(Direction::Down, &controls(&state));
    assert!(matches!(state.focus, Target::App(_)));
    state.tick();
    assert_eq!(state.landing(), 0.);
    assert_eq!(
        state.scroll, 0.,
        "the lowered position never scrolls the page"
    );
    assert_eq!(tile_y(&state), usual_y);
    state.direction(Direction::Up, &controls(&state));
    state.tick();
    assert_eq!(
        state.landing(),
        1.,
        "back in the header the rows lower again"
    );

    // It slides when motion is allowed, and stays put on other pages.
    state.preferences.reduced_motion = false;
    state.direction(Direction::Down, &controls(&state));
    state.tick();
    assert!(state.animating());
    state.preferences.reduced_motion = true;
    state.open(2);
    state.tick();
    assert_eq!(state.focus, Target::Nav(2));
    assert_eq!(
        state.landing(),
        0.,
        "the header of another page keeps the usual layout"
    );

    // General → Wallpaper-first Home switches it.
    let mut settings = ready();
    settings.open(2);
    settings.focus = Target::Setting(LANDING_ROW);
    assert!(scene(&settings).draw.iter().any(|draw| matches!(draw,
        crate::renderer::Draw::Text(value, ..) if value == "Wallpaper-first Home")));
    assert_eq!(settings.activate(), vec![Effect::Save]);
    assert!(settings.preferences.landing_layout);
    let saved = serde_json::to_value(&settings.preferences).unwrap();
    assert_eq!(saved["landing_layout"], true);
    assert!(!Preferences::default().landing_layout);
}

#[test]
fn back_on_home_returns_to_the_first_favourite_when_the_recents_shortcut_is_off() {
    let mut state = ready();
    state.preferences.back_recents = false;
    let first = state.first_content();
    assert!(matches!(first, Target::App(_)));
    // Scrolled along the strip.
    let far = state.apps()[5].id.clone();
    state.focus = Target::App(far);
    state.home_scroll = 400.;
    assert!(state.back().is_empty());
    assert!(state.overlay.is_none());
    assert_eq!(state.focus, first);
    assert_eq!(state.home_scroll, 0.);
    // From the header too.
    state.focus = Target::Nav(0);
    state.back();
    assert_eq!(state.focus, first);
    // With the shortcut on, Back on Home still opens Recent apps.
    state.preferences.back_recents = true;
    state.back();
    assert!(matches!(state.overlay, Some(Overlay::Recents { .. })));
}

#[test]
fn wallpaper_first_home_stays_lowered_without_input_cards() {
    use crate::state::LANDING_DROP;
    let tile_y = |state: &State| {
        controls(state)
            .iter()
            .find(|control| matches!(control.target, Target::App(_)))
            .map(|control| control.rect.y)
            .unwrap()
    };
    let mut usual = ready();
    usual.preferences.reduced_motion = true;
    let usual_y = tile_y(&usual);

    let mut state = State::new(Preferences {
        landing_layout: true,
        reduced_motion: true,
        ..Preferences::default()
    });
    state.note_input();
    state.set_inventory(demo_inventory(), None);
    let inputs: Vec<String> = state
        .inventory
        .inputs
        .iter()
        .map(|i| i.id.clone())
        .collect();
    assert!(!inputs.is_empty());
    for id in inputs {
        state.preferences.input_modes.insert(id, InputMode::Off);
    }
    state.tick();
    assert_eq!(state.landing(), 1.);
    // Down into the favourites keeps them where they are: there is nothing below to reveal.
    state.direction(Direction::Down, &controls(&state));
    assert!(matches!(state.focus, Target::App(_)));
    state.tick();
    assert_eq!(state.landing(), 1.);
    assert_eq!(tile_y(&state), usual_y + LANDING_DROP);
    assert_eq!(state.scroll, 0.);
}

#[test]
fn wallpaper_first_home_treats_the_inputs_button_as_header() {
    let mut state = State::new(Preferences {
        landing_layout: true,
        inputs_menu: true,
        reduced_motion: true,
        ..Preferences::default()
    });
    state.set_inventory(demo_inventory(), None);
    state.focus = Target::InputsButton;
    state.tick();
    assert_eq!(
        state.landing(),
        1.,
        "the top-bar Inputs button keeps the row lowered"
    );
}

#[test]
fn wallpaper_first_pointer_hover_and_press_preserve_card_geometry() {
    for reduced_motion in [false, true] {
        let mut state = State::new(Preferences {
            landing_layout: true,
            reduced_motion,
            ..Preferences::default()
        });
        state.set_inventory(demo_inventory(), None);
        let card = controls(&state)
            .into_iter()
            .find(|control| matches!(control.target, Target::App(_)))
            .unwrap();
        let (x, y) = (
            card.rect.x + card.rect.w / 2.,
            card.rect.y + card.rect.h / 2.,
        );
        for target in [card.target.clone(), Target::Nav(0), card.target.clone()] {
            state.pointer_focus(target);
            state.tick();
            let current = controls(&state);
            let hovered = current
                .iter()
                .find(|control| control.target == card.target)
                .unwrap();
            assert_eq!(hovered.rect.y, card.rect.y);
            assert_eq!(state.hit_test(&current, x, y), Some(card.target.clone()));
        }
        state.pressed = Some(card.target.clone());
        state.pointer_focus(card.target.clone());
        state.tick();
        assert_eq!(state.hit_test(&controls(&state), x, y), state.pressed);
        state.pressed = None;
        assert_eq!(state.activate(), vec![Effect::Launch("netflix".into())]);
        state.tick();
        assert_eq!(state.landing(), 1.);
        // Switching back to the remote resumes expansion, with no page scroll.
        state.direction(Direction::Right, &controls(&state));
        state.tick();
        state.preferences.reduced_motion = true;
        assert_eq!(state.landing(), 0.);
        assert_eq!(state.scroll, 0.);
    }
}

#[test]
fn pointer_interrupts_a_landing_slide_until_remote_navigation_resumes() {
    let mut state = State::new(Preferences {
        landing_layout: true,
        ..Preferences::default()
    });
    state.note_input();
    state.set_inventory(demo_inventory(), None);
    state.direction(Direction::Down, &controls(&state));
    state.tick();
    assert!(state.animating());
    let target = state.focus.clone();
    state.pointer_focus(target);
    let frozen = state.landing();
    std::thread::sleep(std::time::Duration::from_millis(20));
    state.tick();
    assert_eq!(state.landing(), frozen);
    state.direction(Direction::Up, &controls(&state));
    state.tick();
    state.preferences.reduced_motion = true;
    assert_eq!(state.landing(), 1.);
}

#[test]
fn clock_size_is_chosen_in_clock_settings_and_keeps_the_clock_centred_on_the_navigation() {
    use crate::model::ClockSize;
    let clock = |state: &State| {
        let label = state.preferences.language.clock(
            time::OffsetDateTime::UNIX_EPOCH,
            state.preferences.hour_24,
            state.preferences.show_seconds,
        );
        scene(state)
            .draw
            .into_iter()
            .find_map(|draw| match draw {
                crate::renderer::Draw::Text(text, rect, style, ..) if text == label => {
                    Some((rect, style.size))
                }
                _ => None,
            })
            .unwrap()
    };
    let mut state = ready();
    assert_eq!(state.preferences.clock_size, ClockSize::Medium);
    // Medium is the original header clock: 24 px in a 28.8 px line at y 62.4.
    let (rect, size) = clock(&state);
    assert_eq!((rect.x, rect.y, rect.h, size), (84., 62.4, 28.8, 24.));
    // Without tabs, the clock keeps clear of the gear at the top right.
    let nav = scene(&state)
        .controls
        .iter()
        .find(|control| control.target == Target::Nav(2))
        .unwrap()
        .rect;
    assert!(rect.x + rect.w <= nav.x);
    // Settings → General → Clock → Size → Extra large.
    state.open(2);
    state.focus = Target::Setting(1);
    state.direction(Direction::Right, &controls(&state));
    state.direction(Direction::Down, &controls(&state));
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::ClockSize);
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.subpage, Some(crate::state::Subpage::ClockSize));
    assert_eq!(state.focus, Target::Choice(1));
    assert!(
        controls(&state)
            .iter()
            .any(|c| c.target == Target::Choice(3))
    );
    for _ in 0..2 {
        state.direction(Direction::Down, &controls(&state));
    }
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.clock_size, ClockSize::ExtraLarge);
    assert_eq!(state.subpage, Some(crate::state::Subpage::Clock));
    assert_eq!(state.focus, Target::ClockSize);
    for size in ClockSize::ALL {
        state.preferences.clock_size = size;
        let (rect, font) = clock(&state);
        assert_eq!(font, size.font_size());
        assert!(
            (rect.y + rect.h / 2. - 76.8).abs() < 0.01,
            "{size:?} centred on the nav bar"
        );
        assert!(
            rect.x + rect.w <= nav.x,
            "{size:?} stays left of the top-right buttons"
        );
    }
    // Back leaves the clock page for General, like the other clock settings.
    state.back();
    assert_eq!(state.focus, Target::Setting(1));
    assert!(state.subpage.is_none());
}

#[test]
fn clock_size_preferences_round_trip_and_tolerate_unknown_values() {
    use crate::model::{ClockSize, Preferences};
    let saved = serde_json::to_value(Preferences {
        clock_size: ClockSize::ExtraLarge,
        ..Preferences::default()
    })
    .unwrap();
    assert_eq!(saved["clock_size"], "extra-large");
    let loaded: Preferences = serde_json::from_value(saved).unwrap();
    assert_eq!(loaded.clock_size, ClockSize::ExtraLarge);
    for (value, size) in [
        (serde_json::json!("small"), ClockSize::Small),
        (serde_json::json!("huge"), ClockSize::Medium),
        (serde_json::json!(3), ClockSize::Medium),
    ] {
        let loaded: Preferences =
            serde_json::from_value(serde_json::json!({ "clock_size": value })).unwrap();
        assert_eq!(loaded.clock_size, size);
    }
    let older: Preferences =
        serde_json::from_value(serde_json::json!({ "hour_24": true })).unwrap();
    assert_eq!(older.clock_size, ClockSize::Medium);
}

#[test]
fn five_day_forecast_appears_under_the_weather_only_when_switched_on() {
    let mut state = ready();
    with_forecast(&mut state);
    let has_days = |state: &State| {
        let texts = drawn_texts(state);
        ["Fri", "Sat", "Sun", "Mon", "Tue"]
            .iter()
            .all(|day| texts.iter().any(|(value, _)| value == day))
    };
    assert!(!state.preferences.show_forecast);
    assert!(!has_days(&state));
    // Settings → Widgets → Weather → 5-day forecast.
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Weather);
    state.focus = Target::WeatherForecast;
    assert!(
        controls(&state)
            .iter()
            .any(|control| control.target == Target::WeatherForecast)
    );
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.show_forecast);
    state.open(0);
    assert!(has_days(&state));
    // Hiding the weather widget hides the forecast too.
    state.preferences.show_weather = false;
    assert!(!has_days(&state));
}

#[test]
fn glass_clock_replaces_the_greeting_and_header_clock_on_home() {
    use crate::state::GLASS_CLOCK_ROW;
    let mut state = ready();
    with_forecast(&mut state);
    state.preferences.show_forecast = true;
    let greetings: Vec<String> = (0..24)
        .map(|hour| state.preferences.language.greeting(hour, ""))
        .collect();
    let greeting_shown = |state: &State| {
        drawn_texts(state)
            .iter()
            .any(|(value, _)| greetings.contains(value))
    };
    assert!(greeting_shown(&state));
    // General → Glass clock on Home.
    state.open(2);
    state.focus = Target::Setting(GLASS_CLOCK_ROW);
    assert!(
        drawn_texts(&state)
            .iter()
            .any(|(value, _)| value == "Glass clock on Home")
    );
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.glass_clock);
    // Settings keeps the small clock in the header.
    let small_clocks = drawn_texts(&state)
        .iter()
        .filter(|(value, size)| value.contains(':') && *size < 50.)
        .count();
    assert_eq!(small_clocks, 1);
    state.open(0);
    let texts = drawn_texts(&state);
    assert!(!greeting_shown(&state));
    // One large time, no small header clock, and the forecast under it.
    assert!(
        texts
            .iter()
            .any(|(value, size)| value.contains(':') && *size > 80.)
    );
    assert!(
        !texts
            .iter()
            .any(|(value, size)| value.contains(':') && *size < 50.)
    );
    assert!(texts.iter().any(|(value, _)| value == "Tue"));
    // The large time never reaches the favourites.
    let favourites_y = scene(&state)
        .controls
        .iter()
        .filter(|control| matches!(control.target, Target::App(_)))
        .map(|control| control.rect.y)
        .fold(f32::MAX, f32::min);
    for draw in &scene(&state).draw {
        if let crate::renderer::Draw::Text(value, rect, style, ..) = draw
            && value.contains(':')
            && style.size > 80.
        {
            assert!(rect.y + rect.h < favourites_y);
        }
    }
}

#[test]
fn pointer_click_on_the_home_tab_lowers_the_wallpaper_first_rows() {
    let mut state = State::new(Preferences {
        landing_layout: true,
        reduced_motion: true,
        ..Preferences::default()
    });
    state.set_inventory(demo_inventory(), None);
    state.open(2);
    state.tick();
    assert_eq!(state.landing(), 0., "other pages keep the usual layout");
    // The Magic Remote pointer hovers and clicks the Home tab.
    state.pointer_focus(Target::Nav(0));
    assert!(state.pointer);
    state.activate();
    assert_eq!(state.page, crate::state::Page::Home);
    state.tick();
    assert_eq!(
        state.landing(),
        1.,
        "Home opens lowered, as with the remote"
    );
}

#[test]
fn ambient_screen_starts_after_idle_home_and_the_waking_input_only_wakes() {
    use crate::state::AMBIENT_ROW;
    use std::time::Duration;
    let mut state = ready();
    state.preferences.reduced_motion = true;
    // Off by default.
    state.last_input = std::time::Instant::now() - Duration::from_secs(3600);
    state.ambient_tick(true);
    assert!(state.ambient.is_none());
    assert!(!state.ambient_holds_screen_saver());
    // General → Ambient screen cycles Off → 2 → 5 → 10 → Off.
    state.open(2);
    state.section = crate::state::AMBIENT_SECTION;
    state.focus = Target::Setting(AMBIENT_ROW);
    for expected in [2, 5, 10, 0, 2] {
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert_eq!(state.preferences.ambient_minutes, expected);
    }
    assert!(
        drawn_texts(&state)
            .iter()
            .any(|(value, _)| value == "2 min")
    );
    // Not on Settings, only on Home.
    state.last_input = std::time::Instant::now() - Duration::from_secs(600);
    state.ambient_tick(true);
    assert!(state.ambient.is_none());
    state.open(0);
    // Not before the delay, and not while Luma is hidden.
    state.last_input = std::time::Instant::now() - Duration::from_secs(60);
    state.ambient_tick(true);
    assert!(state.ambient.is_none());
    state.last_input = std::time::Instant::now() - Duration::from_secs(121);
    state.ambient_tick(false);
    assert!(state.ambient.is_none());
    state.ambient_tick(true);
    assert!(state.ambient.is_some());
    assert!(state.ambient_holds_screen_saver());
    // The ambient scene: no controls, one large time, no greeting or favourites.
    let scene = scene(&state);
    assert!(scene.controls.is_empty());
    assert!(
        drawn_texts(&state)
            .iter()
            .any(|(value, size)| value.contains(':') && *size > 200.)
    );
    // The waking input is consumed; the next one acts normally.
    let focus = state.focus.clone();
    assert!(state.note_input());
    assert!(state.ambient.is_none());
    assert_eq!(state.focus, focus);
    assert!(!state.note_input());
    // After its hand-off time, LG's own screen saver may take over.
    state.ambient = Some(std::time::Instant::now() - crate::state::AMBIENT_HANDOFF);
    assert!(!state.ambient_holds_screen_saver());
    // Leaving Home or switching it off ends it.
    state.ambient = Some(std::time::Instant::now());
    state.preferences.ambient_minutes = 0;
    state.ambient_tick(true);
    assert!(state.ambient.is_none());
}

#[test]
fn ambient_screen_fades_home_then_tours_the_corners() {
    use crate::state::{AMBIENT_DRIFT_EVERY, AmbientSpot};
    use std::time::{Duration, Instant};
    let mut state = ready();
    with_forecast(&mut state);
    state.preferences.glass_clock = true;
    state.preferences.show_forecast = true;
    state.preferences.ambient_minutes = 2;
    let at = |state: &mut State, seconds: f32| {
        state.ambient = Some(Instant::now() - Duration::from_secs_f32(seconds));
    };
    let clock_rect = |state: &State| {
        scene(state)
            .draw
            .iter()
            .find_map(|draw| match draw {
                crate::renderer::Draw::Text(value, rect, style, color, _)
                    if value.contains(':') && style.size > 80. && color[0] > 0.5 =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap()
    };
    let home = clock_rect(&state);
    // Starting: Home is still fully drawn and the clock is exactly where Home has it.
    at(&mut state, 0.);
    let (ui, dim) = state.ambient_progress();
    assert!(ui < 0.01 && dim == 0.);
    assert_eq!(
        (clock_rect(&state).x, clock_rect(&state).y),
        (home.x, home.y)
    );
    // Home fades, then the wallpaper dims; the clock stays put for the first minute.
    at(&mut state, 3.);
    let (ui, dim) = state.ambient_progress();
    assert_eq!(ui, 1.);
    assert!(dim > 0. && dim < 1.);
    at(&mut state, 59.);
    assert_eq!(state.ambient_progress(), (1., 1.));
    assert_eq!(
        (clock_rect(&state).x, clock_rect(&state).y),
        (home.x, home.y)
    );
    // Then one stop a minute: lower left, upper left, upper right, lower right, centre.
    let mut corners = vec![];
    for (minute, expected) in [
        (1., AmbientSpot::LowerLeft),
        (2., AmbientSpot::UpperLeft),
        (3., AmbientSpot::UpperRight),
        (4., AmbientSpot::LowerRight),
        (5., AmbientSpot::Center),
        (6., AmbientSpot::LowerLeft),
    ] {
        at(&mut state, minute * AMBIENT_DRIFT_EVERY + 30.);
        let (_, to, glide) = state.ambient_stops();
        assert_eq!((to, glide), (expected, 1.));
        corners.push(clock_rect(&state));
    }
    let (lower_left, upper_left, upper_right, lower_right, centre) =
        (corners[0], corners[1], corners[2], corners[3], corners[4]);
    assert!(lower_left.x < 400. && lower_left.y > upper_left.y);
    assert_eq!(lower_left.x, upper_left.x);
    assert!(upper_right.x > 960. && upper_right.y == upper_left.y);
    assert_eq!(lower_right.y, lower_left.y);
    assert!((centre.x + centre.w / 2. - 960.).abs() < 2.);
    // Padding keeps the block clear of the edges.
    assert!(upper_left.x >= 100. && upper_right.x + upper_right.w <= 1820.);
    assert_eq!((corners[5].x, corners[5].y), (lower_left.x, lower_left.y));
    // Mid-glide it is between its stops.
    at(&mut state, AMBIENT_DRIFT_EVERY + 2.);
    let mid = clock_rect(&state);
    assert!(mid.x < home.x && mid.x > lower_left.x);
}

#[test]
fn ambient_fades_and_glides_redraw_every_changed_frame() {
    use crate::state::AMBIENT_DRIFT_EVERY;
    use std::time::{Duration, Instant};
    let mut state = ready();
    state.preferences.ambient_minutes = 2;
    let frame = |state: &mut State, seconds: f32| {
        state.ambient = Some(Instant::now() - Duration::from_secs_f32(seconds));
        state.dirty = false;
        state.tick();
        state.dirty
    };
    // Each step of the fade and of a glide asks for a frame.
    for seconds in [0.2, 0.6, 1.4, 2.4, 3.3, 4.2] {
        assert!(frame(&mut state, seconds), "fade frame at {seconds} s");
    }
    // Settled: the same still frame is not redrawn again.
    assert!(frame(&mut state, 30.));
    assert!(!frame(&mut state, 31.));
    for offset in [0.5, 1.5, 2.5, 3.5] {
        assert!(
            frame(&mut state, AMBIENT_DRIFT_EVERY + offset),
            "glide frame at +{offset} s"
        );
    }
    assert!(frame(&mut state, AMBIENT_DRIFT_EVERY + 10.));
    assert!(!frame(&mut state, AMBIENT_DRIFT_EVERY + 20.));
}

#[test]
fn selected_tile_lifts_with_a_two_tone_ring_and_the_setting_turns_the_lift_off() {
    use crate::state::TILE_LIFT_ROW;
    let mut state = ready();
    state.preferences.reduced_motion = true;
    let first = state.first_content();
    let Target::App(id) = first.clone() else {
        panic!("Home starts on a favourite");
    };
    let second = Target::App(state.apps()[1].id.clone());
    state.focus = first.clone();
    state.tick();
    let shadow_of = |state: &State, target: &Target| {
        let hit = controls(state)
            .into_iter()
            .find(|control| control.target == *target)
            .unwrap()
            .rect;
        let scene = scene(state);
        let shadow = scene
            .draw
            .iter()
            .find_map(|draw| match draw {
                crate::renderer::Draw::Shadow(rect, ..)
                    if (rect.x - hit.x).abs() < 20. && (rect.y - hit.y).abs() < 20. =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap();
        (hit, shadow)
    };
    // Lifted: larger and higher than its unchanged hit area.
    let (hit, drawn) = shadow_of(&state, &first);
    assert!(drawn.w > hit.w * 1.06 && drawn.x < hit.x - 8.);
    // Its neighbour is not.
    let (hit2, drawn2) = shadow_of(&state, &second);
    assert_eq!((drawn2.w, drawn2.h), (hit2.w, hit2.h));
    // The ring has a dark band inside the white one.
    let borders: Vec<[f32; 4]> = scene(&state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Shape(_, _, stroke, color) if *stroke > 0. => Some(*color),
            _ => None,
        })
        .collect();
    assert!(borders.iter().any(|color| color[0] < 0.1 && color[3] > 0.9));
    assert!(borders.iter().any(|color| color[0] > 0.9 && color[3] == 1.));
    // Moving on: the new tile rises and the old one settles.
    state.focus = second.clone();
    state.tick();
    assert_eq!(state.tile_lift(&second), 1.);
    assert_eq!(state.tile_lift(&first), 0.);
    // General → Lift selected tile turns it off; the ring stays.
    state.open(2);
    state.focus = Target::Setting(TILE_LIFT_ROW);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.tile_lift);
    state.open(0);
    state.focus = Target::App(id);
    state.tick();
    let (hit, drawn) = shadow_of(&state, &first);
    assert_eq!((drawn.w, drawn.h), (hit.w, hit.h));
}

fn with_forecast(state: &mut State) {
    let place = crate::weather::Location {
        name: "Fort Collins".into(),
        region: "Colorado".into(),
        country: "United States".into(),
        latitude: 40.5853,
        longitude: -105.0844,
    };
    let day = |weekday, code| crate::weather::Day {
        weekday,
        high: 10.,
        low: -2.,
        code,
    };
    state.preferences.weather_location = Some(place.clone());
    state.weather.cache = Some(crate::weather::Cache {
        key: place.key(),
        updated_at: 1000,
        reading: crate::weather::Reading {
            temperature: 8.,
            code: 2,
            is_day: true,
            daily: Some([day(4, 2), day(5, 0), day(6, 0), day(0, 3), day(1, 73)]),
            sun: None,
        },
    });
}

fn drawn_texts(state: &State) -> Vec<(String, f32)> {
    scene(state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(value, _, style, ..) => Some((value.clone(), style.size)),
            _ => None,
        })
        .collect()
}

#[test]
fn cold_start_focus_moves_from_tab_bar_to_first_app_once_apps_load() {
    let mut state = State::new(Preferences::default());
    assert_eq!(state.focus, Target::Nav(0));
    state.set_inventory(demo_inventory(), None);
    assert!(matches!(state.focus, Target::App(_)), "{:?}", state.focus);
}

#[test]
fn cold_start_focus_stays_put_after_the_user_presses_something() {
    let mut state = State::new(Preferences::default());
    state.note_input();
    state.set_inventory(demo_inventory(), None);
    assert_eq!(state.focus, Target::Nav(0));
}

#[test]
fn hovering_the_gear_lights_it_only_while_hovered() {
    use crate::renderer::Draw;
    let mut state = ready();
    let lit = |state: &State| {
        let scene = scene(state);
        let rect = scene
            .controls
            .iter()
            .find(|c| c.target == Target::Nav(2))
            .unwrap()
            .rect;
        scene.draw.iter().any(|draw| {
            matches!(draw, Draw::RoundedGradient(r, ..) if (r.x, r.y, r.w, r.h) == (rect.x, rect.y, rect.w, rect.h))
        })
    };
    assert!(!lit(&state));
    state.pointer_focus(Target::Nav(2));
    state.hover = Some(Target::Nav(2));
    assert!(lit(&state));
    state.hover = None;
    assert!(!lit(&state), "leaving the gear clears the highlight");
}

#[test]
fn waking_glides_the_clock_home_while_home_returns_and_a_second_press_skips_ahead() {
    use std::time::{Duration, Instant};
    let mut state = ready();
    state.preferences.glass_clock = true;
    state.preferences.ambient_minutes = 2;
    // Two minutes in, the clock block rests in the upper left corner.
    state.ambient = Some(Instant::now() - Duration::from_secs(130));
    let clock_x = |state: &State| {
        scene(state)
            .draw
            .iter()
            .filter_map(|draw| match draw {
                crate::renderer::Draw::Text(value, rect, style, ..)
                    if value.contains(':') && style.size > 150. =>
                {
                    Some(rect.x)
                }
                _ => None,
            })
            .fold(f32::MAX, f32::min)
    };
    let corner = clock_x(&state);
    assert!(corner < 400., "{corner}");
    // The waking press is still only a wake, but now starts a transition back to Home.
    assert!(state.note_input());
    assert!(state.ambient.is_none());
    let waking = state.waking.expect("waking");
    assert_eq!(waking.dim, 1.);
    // It begins where the ambient screen left off, with Home already usable.
    assert!((clock_x(&state) - corner).abs() < 2.);
    assert!(!scene(&state).controls.is_empty());
    // Halfway, the clock block is on its way home.
    state.waking.as_mut().unwrap().since = Instant::now() - Duration::from_millis(400);
    let halfway = clock_x(&state);
    assert!(halfway > corner + 50., "{halfway}");
    // At the end it rests exactly where Home draws it, and the wake clears itself.
    state.waking.as_mut().unwrap().since = Instant::now() - Duration::from_secs(1);
    let landed = clock_x(&state);
    state.tick();
    assert!(state.waking.is_none());
    assert!((clock_x(&state) - landed).abs() < 1.);
    // A second press mid-wake skips straight to Home and acts.
    state.ambient = Some(Instant::now() - Duration::from_secs(130));
    assert!(state.note_input());
    assert!(!state.note_input());
    state.finish_wake();
    assert!(state.waking.is_none());
}

#[test]
fn more_apps_hint_follows_its_setting_and_opens_the_drawer() {
    use crate::model::MoreAppsHint;
    let mut state = ready();
    state.preferences.reduced_motion = true;
    let hint = |state: &State| {
        controls(state)
            .into_iter()
            .find(|control| control.target == Target::MoreApps)
    };
    let shown = hint(&state).expect("shown by default");
    assert!(shown.rect.y > 900., "under Home's last row");
    state.preferences.more_apps_hint = MoreAppsHint::Selected;
    state.focus = Target::Nav(2);
    assert!(hint(&state).is_none(), "only while an app is selected");
    state.focus = Target::App("netflix".into());
    assert!(hint(&state).is_some());
    state.preferences.more_apps_hint = MoreAppsHint::Off;
    assert!(hint(&state).is_none());
    // Settings → General cycles it; a click on the hint opens the drawer.
    state.preferences.more_apps_hint = MoreAppsHint::Always;
    state.focus = Target::MoreApps;
    state.activate();
    assert_eq!(state.page, Page::Apps);
    state.open(2);
    state.focus = Target::Setting(crate::state::MORE_APPS_ROW);
    state.activate();
    assert_eq!(state.preferences.more_apps_hint, MoreAppsHint::Selected);
}

#[test]
fn clock_position_and_size_move_and_scale_the_glass_clock_without_reaching_the_favourites() {
    use crate::model::{ClockPosition, ClockSize};
    // The wallpaper-first Home, as on the owner's TV: the rows stay lowered.
    let mut state = State::new(Preferences {
        landing_layout: true,
        reduced_motion: true,
        glass_clock: true,
        ..Preferences::default()
    });
    state.note_input();
    state.set_inventory(demo_inventory(), None);
    assert_eq!(state.landing(), 1.);
    let digits = |state: &State| {
        scene(state)
            .draw
            .into_iter()
            .filter_map(|draw| match draw {
                crate::renderer::Draw::Text(value, rect, style, ..)
                    if value.contains(':') && style.size > 90. =>
                {
                    Some((rect.y, style.size))
                }
                _ => None,
            })
            .fold((f32::MAX, 0_f32), |(y, s), (ry, rs)| (y.min(ry), s.max(rs)))
    };
    let (default_y, medium) = digits(&state);
    state.preferences.clock_position = ClockPosition::Higher;
    assert!(digits(&state).0 < default_y - 40.);
    state.preferences.clock_position = ClockPosition::Lower;
    assert!(digits(&state).0 > default_y + 30.);
    state.preferences.clock_position = ClockPosition::Default;
    state.preferences.clock_size = ClockSize::Small;
    assert!(digits(&state).1 < medium * 0.8);
    state.preferences.clock_size = ClockSize::ExtraLarge;
    state.preferences.clock_position = ClockPosition::Lower;
    let (y, size) = digits(&state);
    assert!(size > medium);
    // The favourites heading sits at 478.5 + 280 with the rows lowered; the clock stays above.
    assert!(
        y + size * 1.02 < 478.5 + crate::state::LANDING_DROP,
        "{y} + {size}"
    );
    // Clock → Position cycles the setting.
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Clock);
    state.focus = Target::ClockPosition;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.clock_position, ClockPosition::Higher);
}

fn scene_at(state: &State, now: time::OffsetDateTime) -> view::Scene {
    let assets = view::Assets {
        root: PathBuf::new(),
        version: "test".into(),
        metrics: BTreeMap::new(),
        artwork: crate::app_artwork::Service::new(false),
    };
    let navigation = view::navigation(&assets, state.preferences.language);
    view::scene_with_pill(
        state,
        &assets,
        navigation.items[state.page_index()].x,
        navigation.items[state.page_index()].w,
        now,
    )
}

#[test]
fn day_and_night_wallpapers_follow_sunset_and_sunrise() {
    let at = |hour: u8, minute: u8| {
        time::OffsetDateTime::UNIX_EPOCH
            .replace_time(time::Time::from_hms(hour, minute, 0).unwrap())
    };
    let mut state = ready();
    let ids: Vec<String> = (0..state.background_count())
        .map(|i| state.background_id(i).unwrap().to_owned())
        .collect();
    state.preferences.background = ids[0].clone();
    state.preferences.night_wallpapers = vec![ids[2].clone()];
    // Without weather: 7 am to 7 pm is day.
    assert!(!state.is_night(at(12, 0)));
    assert!(state.is_night(at(20, 0)));
    assert!(state.is_night(at(6, 59)));
    // Off: nothing changes at night.
    assert!(!state.day_night_due(at(21, 0)));
    state.preferences.day_night = true;
    assert!(!state.day_night_due(at(12, 0)), "a day wallpaper by day");
    assert!(state.day_night_due(at(21, 0)));
    assert!(state.cycle_wallpaper_at(at(21, 0)));
    assert_eq!(
        state.preferences.background, ids[2],
        "the night one after dark"
    );
    assert!(!state.day_night_due(at(21, 5)));
    // By day again, rotation leaves the night wallpaper out.
    assert!(state.day_night_due(at(8, 0)));
    assert!(state.cycle_wallpaper_at(at(8, 0)));
    assert_ne!(state.preferences.background, ids[2]);
    // The weather service's sunset takes over from 7 pm.
    with_forecast(&mut state);
    if let Some(cache) = &mut state.weather.cache {
        cache.reading.sun = Some((6 * 3600 + 50 * 60, 16 * 3600 + 40 * 60));
    }
    assert!(state.is_night(at(17, 0)));
    assert!(!state.is_night(at(16, 0)));
    // The wallpaper menu marks one for night, and back.
    state.open(2);
    state.section = 1;
    state.focus = Target::Background(1);
    state.options();
    state.focus = Target::NightBackground;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.night_wallpapers.contains(&ids[1]));
    state.activate();
    assert!(!state.preferences.night_wallpapers.contains(&ids[1]));
}

#[test]
fn pixel_shift_nudges_the_glass_clock_only_when_on() {
    let mut state = ready();
    state.preferences.glass_clock = true;
    let clock_x = |state: &State, minutes: i64| {
        scene_at(
            state,
            time::OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(minutes),
        )
        .draw
        .into_iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(value, rect, style, ..)
                if value.contains(':') && style.size > 90. =>
            {
                Some(rect.x)
            }
            _ => None,
        })
        .fold(f32::MAX, f32::min)
    };
    // 0:00 and 0:01 draw the same digits' width, three minutes apart nudges.
    let (first, later) = (clock_x(&state, 600), clock_x(&state, 603));
    assert_eq!(first, later, "off by default");
    state.preferences.pixel_shift = true;
    let (first, later) = (clock_x(&state, 600), clock_x(&state, 603));
    assert!((first - later).abs() > 0.5 && (first - later).abs() <= 10.);
}

#[test]
fn rain_and_snow_show_only_with_the_option_and_matching_weather() {
    use crate::state::Precipitation;
    let mut state = ready();
    with_forecast(&mut state);
    let set_code = |state: &mut State, code: u8| {
        if let Some(cache) = &mut state.weather.cache {
            cache.reading.code = code;
        }
    };
    set_code(&mut state, 73);
    assert_eq!(state.precipitation(), None, "off by default");
    state.preferences.weather_effects = true;
    assert_eq!(state.precipitation(), Some(Precipitation::Snow));
    set_code(&mut state, 63);
    assert_eq!(state.precipitation(), Some(Precipitation::Rain));
    let drops = |state: &State| {
        scene(state)
            .draw
            .iter()
            .filter(|draw| matches!(draw, crate::renderer::Draw::Shape(rect, ..) if rect.w < 3.))
            .count()
    };
    assert!(drops(&state) >= 100);
    assert!(state.animating(), "falling weather keeps frames coming");
    set_code(&mut state, 2);
    assert_eq!(state.precipitation(), None, "partly cloudy: nothing falls");
    set_code(&mut state, 73);
    state.open(1);
    assert_eq!(state.precipitation(), None, "Home only");
}

#[test]
fn drawer_toolbar_off_hides_filters_and_search_but_keeps_hidden_apps_reachable() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    let has = |state: &State, target: Target| {
        controls(state)
            .into_iter()
            .any(|control| control.target == target)
    };
    state.open_drawer();
    assert!(has(&state, Target::Filter(1)) && has(&state, Target::Search));
    // Settings → General turns the toolbar off and forgets any search it could no longer clear.
    state.query = "net".into();
    state.open(2);
    state.focus = Target::Setting(crate::state::DRAWER_TOOLBAR_ROW);
    state.activate();
    assert!(!state.preferences.drawer_toolbar);
    assert!(state.query.is_empty());
    state.open_drawer();
    assert!(!has(&state, Target::Filter(1)) && !has(&state, Target::Search));
    assert!(
        !has(&state, Target::Filter(5)),
        "nothing hidden: an empty top row"
    );
    let first = |state: &State| Target::App(state.apps()[0].id.clone());
    let top = |state: &State| {
        controls(state)
            .into_iter()
            .filter_map(|control| match control.target {
                Target::App(_) => Some(control.rect.y),
                _ => None,
            })
            .fold(f32::MAX, f32::min)
    };
    assert!(top(&state) < 200., "the grid moves up into the empty row");
    // Up from the first row goes straight back to Home.
    state.focus = first(&state);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.page, Page::Home);
    // With a hidden app the lone Hidden pill stays, so it can still be restored.
    let hidden = state.apps()[0].id.clone();
    state.preferences.hidden.push(hidden.clone());
    state.open_drawer();
    assert!(has(&state, Target::Filter(5)) && !has(&state, Target::Search));
    assert!(top(&state) > 250.);
    state.focus = first(&state);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::Filter(5));
    state.activate();
    assert!(state.apps().iter().any(|app| app.id == hidden));
}

#[test]
fn most_used_drawer_order_keeps_favourites_first_and_counts_only_real_launches() {
    use crate::model::AppSorting;
    let mut state = ready();
    state.inventory.apps = vec![
        App::new("z", "Zulu"),
        App::new("b", "beta"),
        App::new("a", "Alpha"),
        App::new("f2", "Favourite two"),
        App::new("f1", "Favourite one"),
    ];
    state.preferences.favorites = Some(vec!["f2".into(), "f1".into()]);
    state.preferences.reduced_motion = true;
    state.open_drawer();
    let ids = |state: &State| -> Vec<String> {
        state.apps().into_iter().map(|app| app.id.clone()).collect()
    };
    assert_eq!(ids(&state), ["f2", "f1", "a", "b", "z"], "A–Z by default");
    for _ in 0..3 {
        assert_eq!(state.record_app_launch("z"), [Effect::Save]);
    }
    state.record_app_launch("b");
    state.record_app_launch("f1");
    assert!(state.record_app_launch("not-installed").is_empty());
    assert_eq!(
        ids(&state),
        ["f2", "f1", "a", "b", "z"],
        "counts alone change nothing"
    );
    state.preferences.app_sorting = AppSorting::Usage;
    assert_eq!(ids(&state), ["f2", "f1", "z", "b", "a"]);
    // Older preference files load with A–Z and no counts.
    let old: Preferences = serde_json::from_str("{}").unwrap();
    assert_eq!(old.app_sorting, AppSorting::Alphabetical);
    assert!(old.app_launch_counts.is_empty());
    let saved: Preferences =
        serde_json::from_slice(&serde_json::to_vec(&state.preferences).unwrap()).unwrap();
    assert_eq!(saved.app_launch_counts["z"], 3);
    assert_eq!(saved.app_sorting, AppSorting::Usage);
}

#[test]
fn calculated_sun_times_match_fort_collins_in_november() {
    // Fort Collins, 16 Nov 2026: sunrise about 6:50 and sunset about 16:40 local (UTC-7).
    let date = time::Date::from_calendar_date(2026, time::Month::November, 16).unwrap();
    let (rise, set) = crate::weather::solar_times(40.5853, -105.0844, date).unwrap();
    let local = |unix: i64| {
        let at = time::OffsetDateTime::from_unix_timestamp(unix)
            .unwrap()
            .to_offset(time::UtcOffset::from_hms(-7, 0, 0).unwrap());
        at.hour() as i32 * 60 + at.minute() as i32
    };
    assert!((local(rise) - (6 * 60 + 50)).abs() <= 6, "{}", local(rise));
    assert!((local(set) - (16 * 60 + 40)).abs() <= 6, "{}", local(set));
    // Without the service's times, 6:30 pm in November is night.
    let mut state = ready();
    state.preferences.weather_location = Some(crate::weather::Location {
        name: "Fort Collins".into(),
        region: "Colorado".into(),
        country: "United States".into(),
        latitude: 40.5853,
        longitude: -105.0844,
    });
    let evening = time::PrimitiveDateTime::new(date, time::Time::from_hms(18, 30, 0).unwrap())
        .assume_offset(time::UtcOffset::from_hms(-7, 0, 0).unwrap());
    assert!(state.is_night(evening));
    let noon = evening.replace_hour(12).unwrap();
    assert!(!state.is_night(noon));
}

#[test]
fn sound_output_cycles_default_then_each_tv_output_and_back() {
    use crate::state::{CONTROLS_SECTION, UI_SOUND_OUTPUT_ROW};
    let mut state = ready();
    state.preferences.ui_sounds = true;
    state.open(2);
    state.section = CONTROLS_SECTION;
    assert!(
        controls(&state)
            .iter()
            .any(|control| control.target == Target::Setting(UI_SOUND_OUTPUT_ROW))
    );
    state.ui_sound_outputs = vec!["pdefaultapp".into(), "pfeedback".into()];
    state.focus = Target::Setting(UI_SOUND_OUTPUT_ROW);
    let mut seen = vec![];
    for _ in 0..3 {
        assert_eq!(state.activate(), vec![Effect::Save]);
        seen.push(state.preferences.ui_sound_output.clone());
    }
    assert_eq!(
        seen,
        [Some("pdefaultapp".into()), Some("pfeedback".into()), None]
    );
    state.preferences.ui_sounds = false;
    assert!(
        !controls(&state)
            .iter()
            .any(|control| control.target == Target::Setting(UI_SOUND_OUTPUT_ROW))
    );
}

#[test]
fn wallpaper_tint_follows_the_fort_collins_day() {
    use crate::model::WallpaperTint;
    let mut state = ready();
    state.preferences.weather_location = Some(crate::weather::Location {
        name: "Fort Collins".into(),
        region: "Colorado".into(),
        country: "United States".into(),
        latitude: 40.5853,
        longitude: -105.0844,
    });
    let date = time::Date::from_calendar_date(2026, time::Month::November, 16).unwrap();
    let at = |hour: u8, minute: u8| {
        time::PrimitiveDateTime::new(date, time::Time::from_hms(hour, minute, 0).unwrap())
            .assume_offset(time::UtcOffset::from_hms(-7, 0, 0).unwrap())
    };
    assert_eq!(state.wallpaper_tint(at(12, 0)), None, "Off by default");
    state.preferences.wallpaper_tint = WallpaperTint::Subtle;
    let (night, night_alpha) = state.wallpaper_tint(at(23, 0)).unwrap();
    assert_eq!((night, night_alpha), (0x1b2a6b, 0.1));
    let (dawn, _) = state.wallpaper_tint(at(6, 55)).unwrap();
    assert!(
        dawn >> 16 > 0xe0,
        "rose just after a 6:47 sunrise: {dawn:06x}"
    );
    assert_eq!(state.wallpaper_tint(at(12, 0)), None, "nothing at midday");
    let (golden, _) = state.wallpaper_tint(at(16, 15)).unwrap();
    assert!(
        golden >> 16 > 0xe0 && (golden >> 8) & 0xff > 0x80,
        "amber before a 4:40 sunset: {golden:06x}"
    );
    let (dusk, _) = state.wallpaper_tint(at(17, 0)).unwrap();
    assert!(dusk & 0xff > 0x90, "violet after sunset: {dusk:06x}");
    state.preferences.wallpaper_tint = WallpaperTint::Medium;
    assert_eq!(state.wallpaper_tint(at(23, 0)).unwrap().1, 0.2);
}

#[test]
fn an_empty_holiday_is_mentioned_once_a_week_before() {
    let mut state = ready();
    state.page = Page::Home;
    state.note_input();
    let day = |month, day| {
        time::PrimitiveDateTime::new(
            time::Date::from_calendar_date(2026, month, day).unwrap(),
            time::Time::from_hms(19, 0, 0).unwrap(),
        )
        .assume_offset(time::UtcOffset::from_hms(-7, 0, 0).unwrap())
    };
    // Mid-November: Thanksgiving week starts Nov 23, five days away.
    assert_eq!(
        state.remind_holiday(day(time::Month::November, 18)),
        [Effect::Save]
    );
    let (text, _) = state.notice.clone().unwrap();
    assert!(
        text.starts_with("Thanksgiving starts November 23."),
        "{text}"
    );
    // Only once a year.
    state.notice = None;
    assert!(
        state
            .remind_holiday(day(time::Month::November, 19))
            .is_empty()
    );
    // Christmas (Dec 1) is due on Nov 25, but not once it has wallpapers.
    state
        .preferences
        .wallpaper_collections
        .insert("dunes".into(), "christmas".into());
    assert!(
        state
            .remind_holiday(day(time::Month::November, 25))
            .is_empty()
    );
    state.preferences.wallpaper_collections.clear();
    // Nor when switched off, or nobody is watching.
    state.preferences.holiday_reminders = false;
    assert!(
        state
            .remind_holiday(day(time::Month::November, 25))
            .is_empty()
    );
    state.preferences.holiday_reminders = true;
    state.last_input = std::time::Instant::now() - std::time::Duration::from_secs(60);
    assert!(
        state
            .remind_holiday(day(time::Month::November, 25))
            .is_empty()
    );
    state.note_input();
    assert_eq!(
        state.remind_holiday(day(time::Month::November, 25)),
        [Effect::Save]
    );
    assert!(
        state
            .notice
            .unwrap()
            .0
            .starts_with("Christmas starts December 1.")
    );
}

#[test]
fn standby_is_noticed_from_the_clocks_and_restarts_are_not() {
    use std::time::{Duration, Instant, SystemTime};
    let wall = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
    let monotonic = Instant::now();
    let mut clock = crate::platform::PowerClock::new(wall, monotonic);
    // Ordinary frames and a slow minute: both clocks agree.
    assert!(!clock.check(
        wall + Duration::from_secs(1),
        monotonic + Duration::from_secs(1)
    ));
    assert!(!clock.check(
        wall + Duration::from_secs(91),
        monotonic + Duration::from_secs(91)
    ));
    // Eight hours of standby: the wall clock moved, the monotonic one barely did.
    let wall = wall + Duration::from_secs(91 + 8 * 3600);
    assert!(clock.check(wall, monotonic + Duration::from_secs(92)));
    // Only once, and a clock set backwards never counts.
    assert!(!clock.check(
        wall + Duration::from_secs(1),
        monotonic + Duration::from_secs(93)
    ));
    assert!(!clock.check(
        wall - Duration::from_secs(3600),
        monotonic + Duration::from_secs(94)
    ));
}

#[test]
fn power_on_shows_home_or_the_ambient_screen() {
    let mut state = State::new(Preferences::default());
    // Ambient was showing (or Settings was open) when the TV went off: Home comes back.
    state.ambient = Some(std::time::Instant::now());
    state.powered_on();
    assert_eq!(state.page, Page::Home);
    assert!(state.ambient.is_none());
    state.open(2);
    state.powered_on();
    assert_eq!(state.page, Page::Home);
    // Ambient: shown at once, even with the ambient timer off, and kept by the tick.
    state.preferences.power_on = crate::model::PowerOn::Ambient;
    state.open(2);
    state.powered_on();
    assert_eq!(state.page, Page::Home);
    assert!(state.ambient.is_some());
    assert_eq!(state.preferences.ambient_minutes, 0);
    state.ambient_tick(true);
    assert!(state.ambient.is_some());
    // Any key wakes it as usual.
    assert!(state.note_input());
    assert!(state.ambient.is_none());
}

#[test]
fn power_on_is_the_return_to_active_from_standby() {
    use serde_json::json;
    let mut watch = crate::platform::PowerWatch::default();
    // The G5's own sequence (2026-11-22): off with the remote, then on again.
    let replies = [
        json!({"subscribed": true, "returnValue": true, "state": "Active"}),
        json!({"state": "Active", "processing": "Request Power Off", "onOff": "off"}),
        json!({"state": "Active", "processing": "Prepare Active Standby", "onOff": "off"}),
        json!({"returnValue": true, "state": "Active Standby"}),
        json!({"state": "Active Standby", "reason": "oslbSaving", "onOff": "off"}),
        json!({"state": "Active Standby", "processing": "Prepare Power On", "onOff": "on"}),
        json!({"state": "Active Standby", "processing": "Screen On", "onOff": "on"}),
        json!({"returnValue": true, "state": "Active"}),
        json!({"returnValue": true, "state": "Active"}),
    ];
    use crate::platform::PowerChange::{Off, On};
    let fired: Vec<_> = replies.iter().map(|reply| watch.update(reply)).collect();
    // Off as soon as the power off is requested (screen still lit), On only at Active.
    assert_eq!(
        fired,
        vec![
            None,
            Some(Off),
            None,
            None,
            None,
            None,
            None,
            Some(On),
            None
        ]
    );
    assert_eq!(watch.update(&json!({"returnValue": true})), None);
    // The next power off is announced again.
    assert_eq!(
        watch
            .update(&json!({"state": "Active", "processing": "Request Power Off", "onOff": "off"})),
        Some(Off)
    );
}

#[test]
fn ambient_at_power_on_is_ready_before_the_screen_comes_back() {
    let mut state = ready();
    state.preferences.power_on = crate::model::PowerOn::Ambient;
    state.open(1);
    assert!(state.arm_power_on_ambient());
    assert_eq!(state.page, Page::Home);
    // Fully faded in already: nothing of Home shows when the TV wakes.
    assert_eq!(state.ambient_progress(), (1., 1.));
    state.powered_on();
    assert_eq!(
        state.ambient_progress(),
        (1., 1.),
        "power on keeps it, no new fade"
    );
    // Without arming (e.g. a wake the off message missed) it still fades in as before.
    let mut state = ready();
    state.preferences.power_on = crate::model::PowerOn::Ambient;
    state.powered_on();
    assert!(state.ambient.is_some());
    assert!(state.ambient_progress().0 < 0.5);
}

#[test]
fn a_wake_into_another_app_drops_the_prepared_ambient_screen() {
    let mut state = ready();
    state.preferences.power_on = crate::model::PowerOn::Ambient;
    assert!(state.arm_power_on_ambient());
    state.power_on_missed();
    assert!(state.ambient.is_none());
    // Home at power on never prepares it.
    let mut state = ready();
    assert!(!state.arm_power_on_ambient());
    assert!(state.ambient.is_none());
    // A missed wake without arming leaves a timer-started ambient screen alone.
    state.ambient = Some(std::time::Instant::now());
    state.power_on_missed();
    assert!(state.ambient.is_some());
}

#[test]
fn stationary_pointer_events_do_not_steal_remote_focus() {
    let mut state = ready();
    state.pointer = false;
    assert!(!crate::pointer_motion_active(&state, 0, 0));
    assert!(crate::pointer_motion_active(&state, 1, 0));
    assert!(crate::pointer_motion_active(&state, 0, -1));
    state.pointer = true;
    assert!(crate::pointer_motion_active(&state, 0, 0));
}

#[test]
fn home_return_holds_hover_off_until_the_pointer_really_moves() {
    use std::time::{Duration, Instant};
    let mut state = ready();
    state.pointer = false;
    let mut motion = crate::PointerMotion::default();
    let now = Instant::now();
    motion.reset_at(now);
    state.return_home();
    let home = state.focus.clone();
    for millis in [0, 100, 400, 749] {
        let x = 1600 + millis as i32;
        let at = now + Duration::from_millis(millis);
        assert!(!motion.accept_at(&state, (x, 100), (10, 2), at));
    }
    assert_eq!(state.focus, home);
    let resumed = now + Duration::from_millis(750);
    // The restored position, even with a delta, only sets a baseline.
    assert!(!motion.accept_at(&state, (2349, 100), (10, 2), resumed));
    assert!(!motion.accept_at(&state, (2349, 100), (5, -3), resumed));
    // Real movement afterwards brings the pointer back.
    assert!(motion.accept_at(&state, (2350, 100), (1, 0), resumed));
}

#[test]
fn clock_style_cycles_through_all_eight_and_survives_a_restart() {
    use crate::model::ClockStyle;
    let mut state = ready();
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Clock);
    state.focus = Target::ClockStyle;
    let mut seen = vec![state.preferences.clock_style];
    for _ in 0..8 {
        assert_eq!(state.activate(), vec![Effect::Save]);
        seen.push(state.preferences.clock_style);
    }
    assert_eq!(&seen[..8], &ClockStyle::ALL);
    assert_eq!(seen[8], ClockStyle::Glass);
    state.preferences.clock_style = ClockStyle::Ticks;
    let saved = serde_json::to_value(&state.preferences).unwrap();
    assert_eq!(saved["clock_style"], "ticks");
    let restored: Preferences = serde_json::from_value(saved).unwrap();
    assert_eq!(restored.clock_style, ClockStyle::Ticks);
    // Older settings files and unknown names keep the original glass clock.
    let old: Preferences = serde_json::from_value(json!({})).unwrap();
    assert_eq!(old.clock_style, ClockStyle::Glass);
    let odd: Preferences = serde_json::from_value(json!({"clock_style": "sundial"})).unwrap();
    assert_eq!(odd.clock_style, ClockStyle::Glass);
}

#[test]
fn analog_clock_styles_draw_hands_instead_of_digits() {
    use crate::model::ClockStyle;
    use crate::renderer::Draw;
    // 2026-11-25 10:25 UTC
    let now = time::OffsetDateTime::from_unix_timestamp(1_795_602_300).unwrap();
    for style in ClockStyle::ALL {
        let mut state = ready();
        state.preferences.glass_clock = true;
        state.preferences.clock_style = style;
        let scene = scene_at(&state, now);
        let rotated = scene
            .draw
            .iter()
            .filter(|draw| matches!(draw, Draw::Rotated(..)))
            .count();
        let digits = scene.draw.iter().any(
            |draw| matches!(draw, Draw::Text(value, ..) if value.contains(":25") || value == "25"),
        );
        if style.analog() {
            // Two hands with their shadows, plus twelve ticks for Ticks.
            let expected = if style == ClockStyle::Ticks { 16 } else { 4 };
            assert_eq!(rotated, expected, "{style:?}");
            assert!(!digits, "{style:?}");
        } else {
            assert_eq!(rotated, 0, "{style:?}");
            assert!(digits, "{style:?}");
        }
    }
}

#[test]
fn clock_settings_list_the_style_row() {
    let mut state = ready();
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Clock);
    state.focus = Target::ClockPosition;
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::ClockStyle);
    let texts: Vec<_> = scene(&state)
        .draw
        .iter()
        .filter_map(|draw| match draw {
            crate::renderer::Draw::Text(value, ..) => Some(value.clone()),
            _ => None,
        })
        .collect();
    assert!(texts.iter().any(|t| t == "Style"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Glass"), "{texts:?}");
}

#[test]
fn magic_remote_jitter_leaves_ambient_alone() {
    use std::time::{Duration, Instant};
    let mut state = ready();
    let mut motion = crate::AmbientMotion::default();
    let now = Instant::now();
    // Zero moves never count, ambient or not.
    assert!(!motion.real_move(&state, 0, 0, now));
    // On Home any real move counts (resets the idle timer as before).
    assert!(motion.real_move(&state, 1, 0, now));
    state.ambient = Some(now);
    // Small twitches, even several, stay below the wake distance.
    for step in 0..6 {
        assert!(!motion.real_move(&state, 3, -2, now + Duration::from_millis(step * 50)));
    }
    // Twitches spread out over time never add up.
    for step in 0..10 {
        assert!(!motion.real_move(&state, 20, 0, now + Duration::from_secs(2 + step)));
    }
    // A real sweep wakes it.
    let later = now + Duration::from_secs(30);
    assert!(!motion.real_move(&state, 30, 0, later));
    assert!(motion.real_move(&state, 30, 0, later + Duration::from_millis(100)));
}

#[test]
fn spotlight_tabs_sit_above_the_drawer_and_load_plex_rows() {
    let mut state = ready();
    state.preferences.spotlight = true;
    state.plex_ready = true;
    state.open(1);
    state.focus = Target::Filter(1);
    let c = controls(&state);
    state.direction(Direction::Up, &c);
    assert_eq!(state.focus, Target::DrawerTab(0));
    let c = controls(&state);
    let effects = state.direction(Direction::Right, &c);
    assert_eq!(state.focus, Target::DrawerTab(1));
    assert!(state.spotlight_open);
    assert!(matches!(effects.as_slice(), [Effect::LoadSpotlight]));
    // A second visit within a few minutes doesn't reload.
    let item = |key: &str| crate::plex::Item {
        key: key.into(),
        title: format!("Title {key}"),
        subtitle: String::new(),
        progress: 0.5,
        poster_path: None,
        poster: None,
    };
    state.spotlight_loaded(Ok(vec![
        crate::plex::Row {
            title: "Continue watching".into(),
            service: None,
            items: vec![item("1"), item("2"), item("3")],
        },
        crate::plex::Row {
            title: "Trending on Prime Video".into(),
            service: Some("prime-video"),
            items: vec![item("4")],
        },
    ]));
    let c = controls(&state);
    state.direction(Direction::Down, &c);
    assert_eq!(state.focus, Target::SpotlightCard(0, 0));
    let c = controls(&state);
    state.direction(Direction::Right, &c);
    state.direction(Direction::Right, &c);
    assert_eq!(state.focus, Target::SpotlightCard(0, 2));
    let c = controls(&state);
    state.direction(Direction::Down, &c);
    assert_eq!(state.focus, Target::SpotlightCard(1, 0));
    let c = controls(&state);
    state.direction(Direction::Up, &c);
    assert_eq!(
        state.focus,
        Target::SpotlightCard(0, 2),
        "row position remembered"
    );
    assert_eq!(
        state.activate(),
        vec![Effect::Launch(crate::state::PLEX_APP.into())]
    );
    // A card in a service's row opens that service's app when it is installed.
    state.focus = Target::SpotlightCard(1, 0);
    let prime_installed = state.inventory.apps.iter().any(|app| app.id == "amazon");
    assert_eq!(
        state.activate(),
        vec![Effect::Launch(
            if prime_installed {
                "amazon"
            } else {
                crate::state::PLEX_APP
            }
            .into()
        )]
    );
    // Back to Apps, then Spotlight again: no reload yet.
    state.focus = Target::DrawerTab(1);
    let c = controls(&state);
    state.direction(Direction::Left, &c);
    assert!(!state.spotlight_open);
    assert!(state.select_drawer_tab(true).is_empty());
    // Off: no tabs, and the drawer is just apps.
    state.preferences.spotlight = false;
    assert!(!state.spotlight_tabs());
    assert!(state.select_drawer_tab(true).is_empty());
    assert!(!state.spotlight_open);
}

#[test]
fn the_drawer_can_open_on_spotlight() {
    let mut state = ready();
    state.preferences.spotlight = true;
    state.plex_ready = true;
    state.open_drawer();
    assert!(!state.spotlight_open, "Apps first by default");
    assert!(!state.take_spotlight_load());
    state.return_home();
    state.preferences.spotlight_first = true;
    state.open_drawer();
    assert!(state.spotlight_open);
    assert_eq!(state.focus, Target::DrawerTab(1), "no rows yet: the tab");
    assert!(state.take_spotlight_load());
    assert!(!state.take_spotlight_load(), "only once while loading");
    // Rows loaded earlier: the drawer opens on the first card of the first row.
    state.spotlight_loaded(Ok(vec![crate::plex::Row {
        title: "Recently added".into(),
        service: None,
        items: vec![crate::plex::Item {
            key: "1".into(),
            title: "Dune".into(),
            subtitle: String::new(),
            progress: 0.,
            poster_path: None,
            poster: None,
        }],
    }]));
    state.return_home();
    state.open_drawer();
    assert_eq!(state.focus, Target::SpotlightCard(0, 0));
    // The option shows only with Spotlight on.
    assert!(
        crate::state::section_rows(crate::state::DRAWER_SECTION, &state.preferences)
            .contains(&crate::state::SPOTLIGHT_FIRST_ROW)
    );
    state.preferences.spotlight = false;
    assert!(
        !crate::state::section_rows(crate::state::DRAWER_SECTION, &state.preferences)
            .contains(&crate::state::SPOTLIGHT_FIRST_ROW)
    );
    state.return_home();
    state.open_drawer();
    assert!(!state.spotlight_open);
}

#[test]
fn every_spotlight_row_is_reachable_with_down() {
    let mut state = ready();
    state.preferences.spotlight = true;
    state.plex_ready = true;
    state.open(1);
    state.select_drawer_tab(true);
    let rows = (0..8)
        .map(|row| crate::plex::Row {
            title: format!("Row {row}"),
            service: None,
            items: (0..12)
                .map(|i| crate::plex::Item {
                    key: format!("{row}x{i}"),
                    title: format!("Title {i}"),
                    subtitle: String::new(),
                    progress: 0.,
                    poster_path: None,
                    poster: None,
                })
                .collect(),
        })
        .collect();
    state.spotlight_loaded(Ok(rows));
    state.focus = Target::DrawerTab(1);
    for row in 0..8 {
        let c = controls(&state);
        state.direction(Direction::Down, &c);
        let c = controls(&state);
        state.reveal_focus(&c);
        assert_eq!(state.focus, Target::SpotlightCard(row, 0), "row {row}");
    }
    assert!(state.scroll > 0., "the page scrolled to the lower rows");
    // And back up to the tabs.
    for _ in 0..8 {
        let c = controls(&state);
        state.direction(Direction::Up, &c);
        let c = controls(&state);
        state.reveal_focus(&c);
    }
    assert_eq!(state.focus, Target::DrawerTab(1));
}
