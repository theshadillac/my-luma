# My Luma enhancement backlog

Running list of enhancements and upgrades from the daily scout, sorted by priority. Nothing is removed until Stephen says so.

Priority: **1** highly recommended · **2** nice to have · **3** maybe · **4** no.

Status: Open, Done (kept on the list), or whatever Stephen sets. A priority or status Stephen sets is never changed by the scout.

Type: **Stability** (keeps Luma from breaking or getting stuck) · **Performance** (speed, smoothness, power) · **Platform** (build, install, updates, backups, diagnostics, things that run behind the scenes) · **Feature** (something you see or use on the TV).

Risk: how likely it is to break something that works today, conflict with My Luma's own changes, or change how things feel. **Low** small and self-contained · **Medium** touches shared code or changes behaviour, needs a test on the TV · **High** large, conflicts likely, or goes against a rule (system controls, losing features).

## 1. Features

Things you see or use on the TV.

| # | Priority | Type | Risk | Name | Description | Example | Added | Status |
|---|---|---|---|---|---|---|---|---|
| B08 | 2 | Feature | Medium | Plex Continue Watching row | Optional Home row with Plex Continue Watching / Recently Added from your server, opening straight into the Plex app. Token kept in settings, never logged. | [link](https://github.com/ramondrent/drent-os) | 2026-11-17 | Open |
| B09 | 2 | Feature | Medium | Four time-of-day wallpaper periods | Dawn / day / dusk / night instead of just day/night, from real sunrise/sunset, with rotation inside each period and optional 4K copies. | [link](https://github.com/Seagey/couch-tv/commit/0c04918) | 2026-11-17 | Open |
| B10 | 2 | Feature | Low | Time-of-day tint overlay | Faint colour wash over the current wallpaper that follows the sun (Off/Subtle/Medium). See ideas/wallpaper-tint-categories-unsplash.md. | [link](https://github.com/ipatek420/halcyon) | 2026-11-18 | Open |
| B11 | 2 | Feature | Medium | Photo-of-the-day wallpaper sources | Optional Bing / Wikimedia / NASA APOD / Unsplash wallpapers with a credit and place caption, cached with a size limit. | [link](https://github.com/ipatek420/Lumen-Launcher) | 2026-11-16 | Open |
| B12 | 2 | Feature | Medium | Wallpaper categories | Tag wallpapers (Landscape, Art, Space, Personal…) and limit the cycle to chosen categories; picker on the phone upload page. | [link](https://github.com/theshadillac/my-luma/blob/scout/ideas/wallpaper-tint-categories-unsplash.md) | 2026-11-18 | Open |
| B14 | 2 | Feature | Medium | Ambient instead of "No Signal" | When the current HDMI input has had no signal ~30 s, show Luma's ambient clock; any key goes back. Reads signal state only. | [link](https://github.com/FrAcTi0N/webos-aerial-nosignal) | 2026-11-17 | Open |
| B15 | 2 | Feature | Medium | Clock behind the subject (depth effect) | A precomputed matte drawn over the glass clock so the clock sits behind the wallpaper's subject; optional slow-zoom loop. | [link](https://github.com/Seagey/couch-tv/commit/a434920) | 2026-11-20 | Open |
| B16 | 2 | Feature | Low | Night amber ambient palette | At night, ambient clock switches to very dim amber-red; brighter morning view in set hours. Colours only, no picture changes. | [link](https://github.com/WMSDev1/TeleDash-Pro) | 2026-11-19 | Open |
| B17 | 2 | Feature | Low | Clock font choice | Pick the glass clock's font in addition to size and position. | [link](https://github.com/aabytt/custom-screensaver-aerial/pull/98) | 2026-11-19 | Open |
| B18 | 2 | Feature | Low | Usage-based app sorting | A–Z or "most used" order in the drawer from local launch counts; favourite order untouched (upstream v3.1.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.1.0) | 2026-11-16 | Open |
| B19 | 2 | Feature | Low | Upstream: Down does nothing when drawer is empty | Stops Down opening an empty drawer when every app is a favourite or hidden (#339). | [link](https://github.com/ecklf/lg-luma/pull/339) | 2026-11-19 | Open |
| B20 | 2 | Feature | Low | Upstream: centre a short favourites row | With 1–5 favourites the row shrinks and centres instead of hugging the left (#340). | [link](https://github.com/ecklf/lg-luma/pull/340) | 2026-11-19 | Open |
| B21 | 2 | Feature | Low | Upstream: input badge fixes | Amber badge when an input reports no signal; hover badges no longer stick (v3.2.1). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.2.1) | 2026-11-16 | Open |
| B23 | 2 | Feature | Low | Focused app's own background as wallpaper | When an app with a background image in its metadata is focused (Netflix, Disney+), show it full screen, Top Shelf style. No network. | [link](https://github.com/swenz/webos-custom-launcher) | 2026-11-19 | Open |
| B25 | 3 | Feature | Medium | Upstream: wrap-around navigation | Right on the last favourite or drawer app wraps to the first (v3.11.0, #357). Try it before adopting. | [link](https://github.com/ecklf/lg-luma/pull/357) | 2026-11-20 | Open |
| B26 | 3 | Feature | Medium | Upstream: configurable Back for Recents | Setting for what Back does about Recent apps; may be a superset of My Luma's back_recents toggle (#287). | [link](https://github.com/ecklf/lg-luma/commit/0eadcab) | 2026-11-17 | Open |
| B27 | 3 | Feature | Low | UI feedback sounds | Optional navigate/select/Back sounds, off by default (upstream v3.2.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.2.0) | 2026-11-16 | Open |
| B28 | 3 | Feature | Low | Compact weather styles | Standard / No icon / Compact weather looks with previews in Settings (upstream v3.1.0, #288). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.1.0) | 2026-11-16 | Open |
| B29 | 3 | Feature | Low | Selected app tints the room | Glow behind Home and the glass blend toward the focused app's colour. | [link](https://github.com/cocombee/glass-tv) | 2026-11-16 | Open |
| B30 | 3 | Feature | Low | Public-domain art pack with captions | Bundled paintings rotating every 10/30/60 min in ambient, optional title/painter caption. | [link](https://github.com/SikeXML/glasshouse-dock) | 2026-11-18 | Open |
| B31 | 3 | Feature | Low | Glass icon style switch | Option to show apps as white marks on dark glass or app-tinted glass; original icons stay default. | [link](https://github.com/benosss/molakaz-tv) | 2026-11-19 | Open |
| B32 | 3 | Feature | Low | Live preview of theme choices | Moving over a wallpaper tint or clock position previews it; OK keeps, Back cancels. | [link](https://github.com/Shaku-Med/lg-tv-launcher) | 2026-11-19 | Open |
| B33 | 3 | Feature | Low | Up to peek at the wallpaper | Up slides the shelf away so only the wallpaper shows. | [link](https://github.com/ipatek420/halcyon) | 2026-11-18 | Open |
| B34 | 3 | Feature | Medium | Ambient sounds that follow the weather | Opt-in birds at dawn, rain when raining, in ambient mode. | [link](https://github.com/benosss/molakaz-tv) | 2026-11-19 | Open |
| B35 | 3 | Feature | High | Video wallpapers | Short looping video as a Home or ambient background; needs a G5 performance test first. | [link](https://github.com/Seagey/couch-tv/commit/51f3096) | 2026-11-16 | Open |
| B37 | 3 | Feature | Medium | Upstream: USB drives in Inputs | List USB drives in Inputs, rename/hide them, open LG's media browser (#347; upstream says needs hardware testing). | [link](https://github.com/ecklf/lg-luma/pull/347) | 2026-11-19 | Open |
| B39 | 3 | Feature | Low | Upstream: favourites kept inside their glass card | Cut-off neighbour tile hidden and unclickable; drawer peek hints at more apps (v3.5.1). Check if My Luma clips the same way. | [link](https://github.com/ecklf/lg-luma/commit/2ac0eda) | 2026-11-18 | Open |
| B40 | 3 | Feature | Low | Upstream: readable focused pills | Focused Settings label gets a dark fill and white outline instead of white-on-white (#351). | [link](https://github.com/ecklf/lg-luma/pull/351) | 2026-11-19 | Open |
| B41 | 3 | Feature | Medium | Upstream: app drawer style setting | Default drawer with app-row peek or a "Minimal" lower dock (v3.5.0); would need adapting to the tab-less Home. | [link](https://github.com/ecklf/lg-luma/commit/c8935b5) | 2026-11-18 | Open |
| B42 | 3 | Feature | Low | Deep-link tiles | Shortcut tiles that open a specific video or page inside an app, not just the app. | [link](https://github.com/Seagey/couch-tv/commit/bc216d9) | 2026-11-20 | Open |
| B43 | 3 | Feature | Medium | HD App Store icons | Optional higher-res icons fetched from the App Store, falling back to webOS icons. Needs network and third-party artwork. | [link](https://github.com/ratelimits/atlauncher) | 2026-11-18 | Open |
| B44 | 3 | Feature | High | App folders | Group apps into folders in the drawer. | [link](https://github.com/Knikt/APL-Launcher) | 2026-11-18 | Open |
| B45 | 3 | Feature | Medium | Live sky ambient | Generated sky that matches your real sky and time of day as an ambient backdrop. | [link](https://github.com/marelucent/quiet-skies) | 2026-11-17 | Open |
| B46 | 3 | Feature | Low | Upstream: Steam input icon | Steam device icon for inputs (v3.4.0); only if a Steam device is connected. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.4.0) | 2026-11-16 | Open |
| B47 | 4 | Feature | High | Upstream: Home widget grid | 6×4 movable widget grid (v3.3.0); clashes with My Luma's clock, glass clock and ambient layout. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.3.0) | 2026-11-16 | Open |
| B48 | 4 | Feature | High | Upstream: crypto/stock widgets and multiple widgets | Tied to the widget grid and adds network calls (#292). | [link](https://github.com/ecklf/lg-luma/commit/82e72bb) | 2026-11-17 | Open |
| B49 | 4 | Feature | High | Upstream: 3.0 removals of captions, greeting, date | Would remove features My Luma keeps. | [link](https://github.com/ecklf/lg-luma/releases) | 2026-11-16 | Open |
| B50 | 4 | Feature | High | Upstream: Midnight-only wallpaper and static splash | Would drop My Luma's wallpapers and splash_background (v3.8.3). | [link](https://github.com/ecklf/lg-luma/commit/48bc9e0) | 2026-11-19 | Open |
| B51 | 4 | Feature | High | Upstream: Hold-only Back for Recents | Replaces short Back with a long press everywhere (v3.3.0); only acceptable as an option (see B26). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.3.0) | 2026-11-16 | Open |
| B53 | 4 | Feature | High | Quick panel with Wi-Fi, Bluetooth, Screen off | LG system controls, ruled out. | [link](https://github.com/Seagey/couch-tv/commit/4d50dca) | 2026-11-20 | Open |
| B54 | 4 | Feature | High | Panel brightness, dimming and panel-off timers | LG system controls, ruled out. | [link](https://github.com/aabytt/custom-screensaver-aerial) | 2026-11-19 | Open |
| B55 | 4 | Feature | High | Picture mode / backlight control centre | LG system controls, ruled out. | [link](https://github.com/ViieeS/mytvlauncher) | 2026-11-18 | Open |
| B57 | 4 | Feature | High | Music visualizer | Needs audio capture Luma doesn't have. | [link](https://github.com/championsnet/wmp-visualizer) | 2026-11-18 | Open |
| B60 | 4 | Feature | Low | Hungarian translation | Upstream language pack (v3.4.0); not needed. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.4.0) | 2026-11-16 | Open |

## 2. Stability, performance and platform

Behind-the-scenes work: keeping Luma from breaking, making it faster, builds, updates, backups and diagnostics.

| # | Priority | Type | Risk | Name | Description | Example | Added | Status |
|---|---|---|---|---|---|---|---|---|
| B01 | 1 | Stability | Low | Upstream: retry Home lifecycle after update | Clears the "no lifecycle" fallback marker when the version changes, so one failed check no longer forces slow restarts on every Home press after an update (v3.6.2). | [link](https://github.com/ecklf/lg-luma/commit/9664872) | 2026-11-18 | Open |
| B02 | 1 | Stability | Low | Upstream: keep Home preload hidden | A background preload of Luma no longer steals the screen from a playing app or HDMI input (v3.6.1). | [link](https://github.com/ecklf/lg-luma/commit/a137e43) | 2026-11-18 | Open |
| B03 | 1 | Performance | High | Upstream: TV performance batch | Worker-thread image decode, shader/image reuse, VAOs, pre-built shadows, correct pacing without VSync, compact shadow masks (v3.7.0–3.11.0). Cherry-pick in pieces. | [link](https://github.com/ecklf/lg-luma/pull/333) | 2026-11-19 | Open |
| B04 | 1 | Performance | Medium | Upstream: lazy hidden pages | Apps and Settings pages render only when needed; returning Home skips the old drawer close animation (v3.6.0). | [link](https://github.com/ecklf/lg-luma/commit/f87b78c) | 2026-11-18 | Open |
| B05 | 1 | Platform | Low | Upstream: smaller release binary | Full LTO, one codegen unit, trimmed font-parser features. Smaller IPK and faster in-app updates, no behaviour change (#293). | [link](https://github.com/ecklf/lg-luma/commit/a3e4234) | 2026-11-17 | Open |
| B06 | 1 | Stability | Medium | Crash guard back to LG Home | If Luma crashes 3 times in 5 minutes, Home falls back to LG's own home so the TV is never stuck. | [link](https://github.com/benosss/molakaz-tv) | 2026-11-19 | Open |
| B07 | 1 | Platform | Low | Settings backup and restore | Export/import all preferences (favourites, input names, tiles, ambient options) as one file, secrets left out, so a reinstall loses nothing. | [link](https://github.com/Heymez0/PureTV) | 2026-11-20 | Open |
| B13 | 2 | Performance | Medium | Idle animations settle and pause when covered | Focus sway stops after a few seconds idle; timers and polling pause while an app or screensaver is in front. Less GPU, less OLED wear. | [link](https://github.com/Seagey/couch-tv/commit/4e5d96d) | 2026-11-17 | Open |
| B22 | 2 | Platform | Low | Diagnostics problem report | One "copy diagnostics" file with TV model, firmware, Luma version and last errors, secrets stripped. Helps when updater or Home takeover misbehaves. | [link](https://github.com/SikeXML/glasshouse-dock) | 2026-11-20 | Open |
| B24 | 2 | Stability | Low | Upstream: glass focus and text tint fix | Restores the glass focus ring and applies text tint in the shader (v3.10.1). Needed if B03 is merged. | [link](https://github.com/ecklf/lg-luma/pull/355) | 2026-11-20 | Open |
| B36 | 3 | Platform | Medium | Upstream: game controller navigation | SDL controller support (D-pad/stick, OK/Back holds, hot-plug), only useful if a controller is paired (v3.8.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.8.0) | 2026-11-19 | Open |
| B38 | 3 | Performance | Medium | Upstream: precomputed glass backdrops | Blurred wallpaper backdrops made at upload time instead of first frame; check if wallpaper changes feel slow (v3.2.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.2.0) | 2026-11-16 | Open |
| B52 | 4 | Platform | Low | Upstream: open Luma at power-on on webOS 5/6 | Only for older webOS, not the G5. | [link](https://github.com/ecklf/lg-luma/commit/0526c11) | 2026-11-18 | Open |
| B56 | 4 | Platform | High | Overlay alert cards over any app | Needs a separate overlay app running over other apps. | [link](https://github.com/ShaneRunham/VICE-Home) | 2026-11-18 | Open |
| B58 | 4 | Platform | High | Ask AI, Hue / Home Assistant via a bridge server | Needs an always-on bridge; webhook tiles already cover smart home. | [link](https://github.com/Seagey/couch-tv/releases) | 2026-11-19 | Open |
| B59 | 4 | Platform | High | Local AI assistant on the TV | Gemini assistant backend on the TV; heavy and off-mission. | [link](https://github.com/paulhoehne2006-a11y/Modern-LG-Hub) | 2026-11-20 | Open |
