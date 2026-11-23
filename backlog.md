# My Luma enhancement backlog

Running list of enhancements and upgrades from the daily scout, sorted by priority. Nothing is removed until Stephen says so.

Priority: **1** highly recommended · **2** nice to have · **3** maybe · **4** no.

Status: Open, Done (kept on the list), or whatever Stephen sets. A priority or status Stephen sets is never changed by the scout.

Type: **Stability** (keeps Luma from breaking or getting stuck) · **Performance** (speed, smoothness, power) · **Platform** (build, install, updates, backups, diagnostics, things that run behind the scenes) · **Feature** (something you see or use on the TV).

Risk: how likely it is to break something that works today, conflict with My Luma's own changes, or change how things feel. **Low** small and self-contained · **Medium** touches shared code or changes behaviour, needs a test on the TV · **High** large, conflicts likely, or goes against a rule (system controls, losing features).

## Guiding principles (go-forward, set by Stephen 2026-11-20)

Every item is weighed against these, in this order:

1. **Stability is critical.** Nothing may introduce stability issues; weigh every item against that first.
2. **Performance.** No noticeable slowdowns. Items that improve performance are strongly considered.
3. **Simplicity.** Avoid adding too many complex features.
4. **Never lose what works today.** New things are opt-in and off by default; existing behaviour only changes if Stephen chooses.
5. **Offline first, nothing new to keep secret.** Prefer items with no network need. Anything online fails quietly, caches, and keeps keys out of logs and backups.
6. **Easy to back out.** Every change can be undone with a setting or by rolling back a release.
7. **Keep upstream merges cheap.** Prefer small, self-contained cherry-picks over rewrites of shared code.
8. **Gentle on the OLED.** No new bright static elements; ambient stays dim at night with pixel shift; idle animations rest.
9. **No LG system controls.** (Standing rule.)

Scoring guide: Priority 1 is for Low or Medium risk stability/performance items and small features that add no complexity. High-risk features are 3 at most, and 4 if they also add complexity or cost performance. Network-dependent features are 3 at most unless Stephen raises them.

## Decisions log

- 2026-11-20: Stability/platform order agreed: B01 + B02 first, then B05, B04, B07. B64 and B24 rejected; B38 parked.
- 2026-11-21: "Return to the launcher when an app closes" checked on the TV: Stephen never sees LG Home, so Home takeover already covers it. No row.
- 2026-11-21: Holidays setting (Take over / Mix in / Off) shipped in 4.11.0; gradient strength levels in 4.10.0.

- 2026-11-22: **True fork.** My Luma keeps its own Home takeover and will not follow upstream v4 / Home Chooser. Upstream is a source of small, self-contained cherry-picks (like B01, B02, B72), not full merges.
- 2026-11-22: At power on fixed to follow tvpower's power state (the G5 stays in Active Standby when turned off, it doesn't suspend).

- 2026-11-23: B80: version jumped to 10.0.0 and the app id change is being planned (ideas/app-id-change.md); start only with Stephen's go-ahead.

- 2026-11-23: Backlog duplicates note checked: B56, B58, B59, B64 and B78 each appear once; nothing to remove.

- 2026-11-23: My Luma renamed **Lume** (id com.theshadillac.lume, 1.0.0, Glow icon). The 11.0.0 com.theshadillac.myluma release is superseded and was never installed.

## 1. Features

Things you see or use on the TV.

| # | Priority | Type | Risk | Name | Description | Example | Added | Status |
|---|---|---|---|---|---|---|---|---|
| B16 | 1 | Feature | Low | Night amber ambient palette | At night, ambient clock switches to very dim amber-red; brighter morning view in set hours. Colours only, no picture changes. | [link](https://github.com/WMSDev1/TeleDash-Pro) | 2026-11-19 | Open |
| B19 | 1 | Feature | Low | Upstream: Down does nothing when drawer is empty | Stops Down opening an empty drawer when every app is a favourite or hidden (#339). | [link](https://github.com/ecklf/lg-luma/pull/339) | 2026-11-19 | Open |
| B67 | 1 | Feature | Low | Upstream: centred move arrows | The left/right arrows shown while moving an app sit centred in their badge (v3.11.1, #367). | [link](https://github.com/ecklf/lg-luma/pull/367) | 2026-11-21 | Done (4.11.1) |
| B09 | 2 | Feature | Medium | Four time-of-day wallpaper periods | Dawn / day / dusk / night instead of just day/night, from real sunrise/sunset, with rotation inside each period and optional 4K copies. | [link](https://github.com/Seagey/couch-tv/commit/0c04918) | 2026-11-17 | Open |
| B10 | 2 | Feature | Low | Time-of-day tint overlay | Faint colour wash over the current wallpaper that follows the sun (Off/Subtle/Medium). See ideas/wallpaper-tint-categories-unsplash.md. | [link](https://github.com/ipatek420/halcyon) | 2026-11-18 | Done (4.6.0) |
| B17 | 2 | Feature | Low | Clock font choice | Pick the glass clock's font in addition to size and position. | [link](https://github.com/aabytt/custom-screensaver-aerial/pull/98) | 2026-11-19 | Open |
| B18 | 2 | Feature | Low | Usage-based app sorting | A–Z or "most used" order in the drawer from local launch counts; favourite order untouched (upstream v3.1.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.1.0) | 2026-11-16 | Done (4.4.0) |
| B20 | 2 | Feature | Low | Upstream: centre a short favourites row | With 1–5 favourites the row shrinks and centres instead of hugging the left (#340). | [link](https://github.com/ecklf/lg-luma/pull/340) | 2026-11-19 | Open |
| B21 | 2 | Feature | Low | Upstream: input badge fixes | Amber badge when an input reports no signal; hover badges no longer stick (v3.2.1). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.2.1) | 2026-11-16 | Done (4.4.0) |
| B23 | 2 | Feature | Low | Focused app's own background as wallpaper | When an app with a background image in its metadata is focused (Netflix, Disney+), show it full screen, Top Shelf style. No network. | [link](https://github.com/swenz/webos-custom-launcher) | 2026-11-19 | Open |
| B82 | 2 | Feature | Low | Moon phase line | A small offline-computed line such as "Full moon Wed" next to the weather or ambient clock; no network. | [link](https://github.com/Seagey/couch-tv/commit/b3d45ab) | 2026-11-23 | Open |
| B08 | 3 | Feature | Medium | Plex Continue Watching row | Optional Home row with Plex Continue Watching / Recently Added from your server, opening straight into the Plex app. Token kept in settings, never logged. Also: [Jellyfin version](https://github.com/Hajadlo/lg-tv-home). | [link](https://github.com/ramondrent/drent-os) | 2026-11-17 | Open |
| B11 | 3 | Feature | Medium | Photo-of-the-day wallpaper sources | Optional Bing / Wikimedia / NASA APOD / Unsplash wallpapers with a credit and place caption, cached with a size limit. Also: [Bing in couch-tv](https://github.com/Seagey/couch-tv/commit/8e99763). Unsplash search and Discover done in 4.8.0–4.9.0; the other sources remain open. | [link](https://github.com/ipatek420/Lumen-Launcher) | 2026-11-16 | Open |
| B12 | 3 | Feature | Medium | Wallpaper categories | Tag wallpapers (Landscape, Art, Space, Personal…) and limit the cycle to chosen categories; picker on the phone upload page. | [link](https://github.com/theshadillac/my-luma/blob/scout/ideas/wallpaper-tint-categories-unsplash.md) | 2026-11-18 | Superseded by collections (4.7.0) |
| B14 | 3 | Feature | Medium | Ambient instead of "No Signal" | When the current HDMI input has had no signal ~30 s, show Luma's ambient clock; any key goes back. Reads signal state only. | [link](https://github.com/FrAcTi0N/webos-aerial-nosignal) | 2026-11-17 | Open |
| B15 | 3 | Feature | Medium | Clock behind the subject (depth effect) | A precomputed matte drawn over the glass clock so the clock sits behind the wallpaper's subject; optional slow-zoom loop. | [link](https://github.com/Seagey/couch-tv/commit/a434920) | 2026-11-20 | Open |
| B25 | 3 | Feature | Medium | Upstream: wrap-around navigation | Right on the last favourite or drawer app wraps to the first (v3.11.0, #357). Try it before adopting. | [link](https://github.com/ecklf/lg-luma/pull/357) | 2026-11-20 | Open |
| B26 | 3 | Feature | Medium | Upstream: configurable Back for Recents | Setting for what Back does about Recent apps; may be a superset of My Luma's back_recents toggle (#287). | [link](https://github.com/ecklf/lg-luma/commit/0eadcab) | 2026-11-17 | Open |
| B27 | 3 | Feature | Low | UI feedback sounds | Optional navigate/select/Back sounds, off by default (upstream v3.2.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.2.0) | 2026-11-16 | Done (4.4.0, with volume and output options) |
| B28 | 3 | Feature | Low | Compact weather styles | Standard / No icon / Compact weather looks with previews in Settings (upstream v3.1.0, #288). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.1.0) | 2026-11-16 | Open |
| B29 | 3 | Feature | Low | Selected app tints the room | Glow behind Home and the glass blend toward the focused app's colour. | [link](https://github.com/cocombee/glass-tv) | 2026-11-16 | Open |
| B30 | 3 | Feature | Low | Public-domain art pack with captions | Bundled paintings rotating every 10/30/60 min in ambient, optional title/painter caption. | [link](https://github.com/SikeXML/glasshouse-dock) | 2026-11-18 | Open |
| B31 | 3 | Feature | Low | Glass icon style switch | Option to show apps as white marks on dark glass or app-tinted glass; original icons stay default. | [link](https://github.com/benosss/molakaz-tv) | 2026-11-19 | Open |
| B32 | 3 | Feature | Low | Live preview of theme choices | Moving over a wallpaper tint or clock position previews it; OK keeps, Back cancels. | [link](https://github.com/Shaku-Med/lg-tv-launcher) | 2026-11-19 | Open |
| B33 | 3 | Feature | Low | Up to peek at the wallpaper | Up slides the shelf away so only the wallpaper shows. | [link](https://github.com/ipatek420/halcyon) | 2026-11-18 | Open |
| B37 | 3 | Feature | Medium | Upstream: USB drives in Inputs | List USB drives in Inputs, rename/hide them, open LG's media browser (#347; upstream says needs hardware testing). | [link](https://github.com/ecklf/lg-luma/pull/347) | 2026-11-19 | Open |
| B39 | 3 | Feature | Low | Upstream: favourites kept inside their glass card | Cut-off neighbour tile hidden and unclickable; drawer peek hints at more apps (v3.5.1). Check if My Luma clips the same way. | [link](https://github.com/ecklf/lg-luma/commit/2ac0eda) | 2026-11-18 | Open |
| B40 | 3 | Feature | Low | Upstream: readable focused pills | Focused Settings label gets a dark fill and white outline instead of white-on-white (#351). | [link](https://github.com/ecklf/lg-luma/pull/351) | 2026-11-19 | Open |
| B42 | 3 | Feature | Low | Deep-link tiles | Shortcut tiles that open a specific video or page inside an app, not just the app. | [link](https://github.com/Seagey/couch-tv/commit/bc216d9) | 2026-11-20 | Open |
| B46 | 3 | Feature | Low | Upstream: Steam input icon | Steam device icon for inputs (v3.4.0); only if a Steam device is connected. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.4.0) | 2026-11-16 | Open |
| B65 | 3 | Feature | Medium | Upstream: weather icon styles | Four bundled offline weather icon sets, a Show icon toggle, a one-line weather style and per-widget city name (v3.12.0, #363). Also: [one-line becomes the default for new widgets](https://github.com/ecklf/lg-luma/commit/3085234). | [link](https://github.com/ecklf/lg-luma/pull/363) | 2026-11-21 | Open |
| B68 | 3 | Feature | Medium | Next calendar event under the clock | Shows your next calendar event below the date, read from a calendar link and cached. | [link](https://github.com/JWille/CanvasFrame) | 2026-11-21 | Open |
| B79 | 3 | Feature | Low | Rolling clock digits | When the minute changes, only the changed digits roll up and fade, using opacity and movement only; off by default. | [link](https://github.com/ViieeS/mytvlauncher/commit/8cb5a4a) | 2026-11-22 | Open |
| B83 | 3 | Feature | Medium | Focus glint and press sink on tiles | A soft highlight that slides across the focused tile (following the pointer) and a slight sink while OK is held; author measured under 1% missed frames. | [link](https://github.com/ViieeS/mytvlauncher/commit/88542de) | 2026-11-23 | Open |
| B34 | 4 | Feature | Medium | Ambient sounds that follow the weather | Opt-in birds at dawn, rain when raining, in ambient mode. | [link](https://github.com/benosss/molakaz-tv) | 2026-11-19 | Open |
| B35 | 4 | Feature | High | Video wallpapers | Short looping video as a Home or ambient background; needs a G5 performance test first. | [link](https://github.com/Seagey/couch-tv/commit/51f3096) | 2026-11-16 | Open |
| B41 | 4 | Feature | Medium | Upstream: app drawer style setting | Default drawer with app-row peek or a "Minimal" lower dock (v3.5.0); would need adapting to the tab-less Home. | [link](https://github.com/ecklf/lg-luma/commit/c8935b5) | 2026-11-18 | Open |
| B43 | 4 | Feature | Medium | HD App Store icons | Optional higher-res icons fetched from the App Store, falling back to webOS icons. Needs network and third-party artwork. | [link](https://github.com/ratelimits/atlauncher) | 2026-11-18 | Open |
| B44 | 4 | Feature | High | App folders | Group apps into folders in the drawer. | [link](https://github.com/Knikt/APL-Launcher) | 2026-11-18 | Open |
| B45 | 4 | Feature | Medium | Live sky ambient | Generated sky that matches your real sky and time of day as an ambient backdrop. | [link](https://github.com/marelucent/quiet-skies) | 2026-11-17 | Open |
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
| B69 | 4 | Feature | High | Remote app buttons open chosen apps | Re-binds the remote's app/assistant buttons to apps via a hook in LG's input daemon. | [link](https://github.com/ViieeS/mytvlauncher/commit/1a260cc) | 2026-11-21 | Open |
| B70 | 4 | Feature | Medium | Reopen Home when an app closes to an input | When an app closes straight to an HDMI input, the launcher takes the screen back. | [link](https://github.com/Seagey/couch-tv/commit/dbae3ed) | 2026-11-21 | Open |
| B71 | 4 | Feature | Medium | Spotify and football score cards | Home cards for what Spotify is playing and live football scores, from online services. | [link](https://github.com/Seagey/couch-tv/commit/06e66e6) | 2026-11-21 | Open |
| B85 | 4 | Feature | High | Inputs picker on the Input key | Hooks LG's input processes so the remote's Input button opens a launcher-drawn inputs list over any app. | [link](https://github.com/lasharor/cleanyourtv-WebOS/blob/main/docs/input-key.md) | 2026-11-23 | Open |
| B86 | 4 | Feature | High | Hidden LG TV settings in the launcher | Puts 100+ of the TV's buried settings (screen saver, Live Plus, HDMI options) into the launcher; LG system controls. | [link](https://github.com/axellaffite/openhome-webos) | 2026-11-23 | Open |


## 2. Stability, performance and platform

Behind-the-scenes work: keeping Luma from breaking, making it faster, builds, updates, backups and diagnostics.

| # | Priority | Type | Risk | Name | Description | Example | Added | Status |
|---|---|---|---|---|---|---|---|---|
| B01 | 1 | Stability | Low | Upstream: retry Home lifecycle after update | Clears the "no lifecycle" fallback marker when the version changes, so one failed check no longer forces slow restarts on every Home press after an update (v3.6.2). | [link](https://github.com/ecklf/lg-luma/commit/9664872) | 2026-11-18 | Done (4.11.1) |
| B02 | 1 | Stability | Low | Upstream: keep Home preload hidden | A background preload of Luma no longer steals the screen from a playing app or HDMI input (v3.6.1). | [link](https://github.com/ecklf/lg-luma/commit/a137e43) | 2026-11-18 | Done (4.11.1) |
| B03 | 1 | Performance | High | Upstream: TV performance batch | Worker-thread image decode, shader/image reuse, VAOs, pre-built shadows, correct pacing without VSync, compact shadow masks (v3.7.0–3.11.0). Cherry-pick in pieces. | [link](https://github.com/ecklf/lg-luma/pull/333) | 2026-11-19 | Split into B61–B64 |
| B04 | 1 | Performance | Medium | Upstream: lazy hidden pages | Apps and Settings pages render only when needed; returning Home skips the old drawer close animation (v3.6.0). | [link](https://github.com/ecklf/lg-luma/commit/f87b78c) | 2026-11-18 | Open |
| B05 | 1 | Platform | Low | Upstream: smaller release binary | Full LTO, one codegen unit, trimmed font-parser features. Smaller IPK and faster in-app updates, no behaviour change (#293). | [link](https://github.com/ecklf/lg-luma/commit/a3e4234) | 2026-11-17 | Done (4.12.1) |
| B07 | 1 | Platform | Low | Settings backup and restore | Export/import all preferences (favourites, input names, tiles, ambient options) as one file, secrets left out, so a reinstall loses nothing. | [link](https://github.com/Heymez0/PureTV) | 2026-11-20 | Done (10.1.0) |
| B72 | 1 | Stability | Low | Upstream: quicker weather retry at startup | If the first forecast fails because the network is not up yet, retry after 10, 20, 30, 60 and 120 s instead of waiting 5 minutes (v3.12.1, #375). Retry logic only, not the new default layout. | [link](https://github.com/ecklf/lg-luma/commit/3085234) | 2026-11-22 | Done (4.12.1) |
| B80 | 1 | Stability | Medium | Keep Homebrew Channel from replacing My Luma | Upstream Luma (same app id com.ecklf.luma) is now in the default Homebrew Channel repo; once its version passes My Luma's, the Channel could offer it as an "update" that overwrites the fork. 2026-11-23: version jumped to 10.0.0; Stephen avoids Update on Luma in the Channel; app id change planned in ideas/app-id-change.md. | [link](https://github.com/webosbrew/apps-repo/commit/6a3fea1) | 2026-11-23 | In progress: renamed to Lume (com.theshadillac.lume 1.0.0, released 2026-11-23); side-by-side test next |
| B77 | 2 | Stability | Medium | Clear a ghost Home entry before launching | After a cut-short pixel refresh at boot, webOS can list the launcher as running when it is not, so Home presses hang; check for that, close the ghost entry and time out launches. Also: [follow-up fix](https://github.com/ViieeS/mytvlauncher/commit/35475ad). Check whether Luma's takeover can hit this first. 2026-11-22: no stuck or slow Home presses seen; revisit only if it happens. | [link](https://github.com/ViieeS/mytvlauncher/commit/2d453c5) | 2026-11-22 | Open |
| B06 | 2 | Stability | Medium | Crash guard back to LG Home | If Luma crashes 3 times in 5 minutes, Home falls back to LG's own home so the TV is never stuck. New code in the Home takeover path; do after the easy wins, with careful design. | [link](https://github.com/benosss/molakaz-tv) | 2026-11-19 | Open |
| B13 | 2 | Performance | Medium | Idle animations settle and pause when covered | Focus sway stops after a few seconds idle; timers and polling pause while an app or screensaver is in front. Less GPU, less OLED wear. Luma already sleeps when idle or covered; measure before building. | [link](https://github.com/Seagey/couch-tv/commit/4e5d96d) | 2026-11-17 | Open |
| B22 | 2 | Platform | Low | Diagnostics problem report | One "copy diagnostics" file with TV model, firmware, Luma version and last errors, secrets stripped. Helps when updater or Home takeover misbehaves. | [link](https://github.com/SikeXML/glasshouse-dock) | 2026-11-20 | Open |
| B61 | 2 | Performance | Low | Upstream: fewer app-list requests, no "Opening" toast | Home asks for the app list once per press instead of twice, and the "Opening" toast is gone on TV (v3.7–3.8). Decision 2026-11-20: take the request reduction only; keep the "Opening" toast. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.8.0) | 2026-11-20 | Open |
| B62 | 2 | Performance | Medium | Upstream: decode images on worker threads and reuse them | First-scene and dock images load off the main thread and are reused between launches (v3.7). Do after B04. | [link](https://github.com/ecklf/lg-luma/blob/main/CHANGELOG.md) | 2026-11-20 | Open |
| B63 | 2 | Performance | Medium | Upstream: correct animation pacing without VSync | Steadier animation timing when the TV does not sync frames (b22b4fc). Only matters if the TV log shows VSync unavailable; check first. | [link](https://github.com/ecklf/lg-luma/commit/b22b4fc) | 2026-11-20 | Open |
| B73 | 2 | Performance | Medium | Upstream: skip off-screen draws in scrolling lists | Rows hidden outside a scrolling list's clip are no longer drawn; upstream measured Settings → Apps at 16.5 → 12.1 ms CPU (#365). Take the clip skip only, not the card batching tied to B64. | [link](https://github.com/ecklf/lg-luma/pull/365) | 2026-11-22 | Open |
| B76 | 3 | Platform | Low | Confirm before a major-version update | The in-app updater stops before a major jump, explains what changes and defaults to Close until you confirm (#383). 2026-11-22: My Luma majors carry no migration; low value. | [link](https://github.com/ecklf/lg-luma/pull/383) | 2026-11-22 | Open |
| B38 | 3 | Performance | Medium | Upstream: precomputed glass backdrops | Blurred wallpaper backdrops made at upload time instead of first frame; check if wallpaper changes feel slow (v3.2.0). Parked 2026-11-20: no slow wallpaper changes seen. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.2.0) | 2026-11-16 | Open |
| B66 | 3 | Platform | Low | Upstream: Attributions page | Read-only, scrollable System → Attributions page listing bundled artwork and font licences (v3.12.0, #363). | [link](https://github.com/ecklf/lg-luma/pull/363) | 2026-11-21 | Open |
| B81 | 3 | Stability | Low | Upstream: Home key handled whenever Luma is in front | Luma claims the Home key while it is foreground even without Home takeover (#388); My Luma already does this when takeover is on. | [link](https://github.com/ecklf/lg-luma/commit/d0149ba) | 2026-11-23 | Open |
| B74 | 4 | Performance | Medium | Upstream: redraw only changed shadows, prepare labels while idle | Focus moves redraw only the shadows that changed and menu labels are built while idle (#370). Builds on upstream's idle shadow code My Luma does not have. | [link](https://github.com/ecklf/lg-luma/pull/370) | 2026-11-22 | Rejected 2026-11-22 (needs B64 plumbing) |
| B75 | 4 | Platform | High | Upstream: Home Chooser compatibility | Lets webosbrew's Home Chooser make Luma the Home app instead of Luma's own takeover scripts (v3.13.0); upstream v4 will expect it, so it shapes future merges. Revisit only if a firmware update breaks the takeover. | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.13.0) | 2026-11-22 | Rejected 2026-11-22: keep My Luma's own Home takeover |
| B24 | 4 | Stability | Low | Upstream: glass focus and text tint fix | Restores the glass focus ring and applies text tint in the shader (v3.10.1). Needed if B03 is merged. | [link](https://github.com/ecklf/lg-luma/pull/355) | 2026-11-20 | Rejected with B64 |
| B36 | 4 | Platform | Medium | Upstream: game controller navigation | SDL controller support (D-pad/stick, OK/Back holds, hot-plug), only useful if a controller is paired (v3.8.0). | [link](https://github.com/ecklf/lg-luma/releases/tag/v3.8.0) | 2026-11-19 | Open |
| B52 | 4 | Platform | Low | Upstream: open Luma at power-on on webOS 5/6 | Only for older webOS, not the G5. | [link](https://github.com/ecklf/lg-luma/commit/0526c11) | 2026-11-18 | Open |
| B56 | 4 | Platform | High | Overlay alert cards over any app | Needs a separate overlay app running over other apps. Also: [CleanYourTV picture notifications](https://github.com/lasharor/cleanyourtv-WebOS/blob/main/docs/notifications.md). | [link](https://github.com/ShaneRunham/VICE-Home) | 2026-11-18 | Open |
| B58 | 4 | Platform | High | Ask AI, Hue / Home Assistant via a bridge server | Needs an always-on bridge; webhook tiles already cover smart home. | [link](https://github.com/Seagey/couch-tv/releases) | 2026-11-19 | Open |
| B59 | 4 | Platform | High | Local AI assistant on the TV | Gemini assistant backend on the TV; heavy and off-mission. | [link](https://github.com/paulhoehne2006-a11y/Modern-LG-Hub) | 2026-11-20 | Open |
| B64 | 4 | Performance | High | Upstream: renderer draw batching | VAOs, grouped logo draws and compact shadow masks: biggest CPU saving but deepest renderer change; do last, one step at a time (#333). | [link](https://github.com/ecklf/lg-luma/pull/333) | 2026-11-20 | Rejected 2026-11-20 |
| B78 | 4 | Performance | High | Close background apps when leaving them | A boot watcher stops LG preloading YouTube/Netflix and closes apps left in the background to free RAM, at the cost of slower cold launches. | [link](https://github.com/kennysarnoski-jarvis/webos-dock) | 2026-11-22 | Rejected 2026-11-22 |
| B84 | 4 | Stability | High | Hold LG's last-input record at the launcher | A watcher rewrites LG's lastinput file so a cold boot or power cut opens the launcher instead of the last HDMI input; edits an LG system file. | [link](https://github.com/ViieeS/mytvlauncher/commit/5da923f) | 2026-11-23 | Open |
