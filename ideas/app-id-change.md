# Plan: moving My Luma to its own app id

Status (2026-11-23): renamed. The app is now **Lume** (`com.theshadillac.lume`, version 1.0.0,
icon option A "Glow"). The 11.0.0 `com.theshadillac.myluma` release was never installed and is
superseded; everything below applies to Lume with the folder `/media/developer/com.theshadillac.lume`.

Earlier (same day): Stephen approved the move to `com.theshadillac.myluma`.
- Phase 0 done: settings backup in 10.1.0 (old id); new-id code with takeover guard and copy guard.
- Phase 1 done: 11.0.0 released as "My Luma" (`com.theshadillac.myluma_11.0.0_arm.ipk`,
  sha256 b012e39cb9e6504c8fddb4b7dda07c7dc84ddca6210e9699dee880dde8f326eb).
- Phase 2 (side by side) is next: copy data as root, install, checklist. The old app stays Home.

Changes from the plan: data is copied by one checked root command (`cp -a`, owners and modes
kept) before first launch, not by the app, because the app may not run as root when opened
from the app list. Until the copy exists the new app saves nothing and shows a notice. The old
10.1.0 app ignores 11.x releases (its updater only accepts its own id).
Backlog: B80. Short-term guard already done: version jumped to 10.0.0, and Stephen does not use
Update on Luma in the Homebrew Channel.

## Why

Upstream Luma is in the Homebrew Channel's default repo as `com.ecklf.luma`, the same id My Luma
uses. Same id means the Channel treats them as one app: a higher upstream version shows up as an
"update" that would overwrite the fork. A separate id (for example `com.theshadillac.myluma`)
makes them two different apps for good.

## What the id touches (from the code, 2026-11-23)

| Area | Where | What changes |
|---|---|---|
| App identity | `appinfo.json`, `src/platform.rs` `APP_ID`, `tv-helper/src/lib.rs` `APP`, `tools/package.mjs` check | One constant each; the package check must accept the new id. |
| Install path | `/media/developer/apps/usr/palm/applications/<id>` in `tv/autostart.sh`, `tv/self-update.sh`, `tv/install-directories.txt`, `tv-helper/src/home.rs` | Follows the id. |
| Your data | `/media/developer/<id>/`: `preferences.json`, `unsplash-key`, `wallpapers/` (up to 150 wallpapers, each with a plain and a shaded copy) | Must be **copied** to the new folder on first start. The old folder stays untouched. |
| Home takeover | `tv/home.sh`, `tv-helper/src/home.rs`: bind mount over LG's Home, marker `/var/run/luma-home`, lifecycle marker | Paths follow the id. The marker names are shared, so only one copy may own the takeover at a time. |
| Boot hook | `/var/lib/webosbrew/init.d/65-luma` → `<install path>/autostart.sh` | Same hook name for both. The new copy must not enable it while the old one owns it. |
| In-app updater | `src/update.rs`, `tv/self-update.sh` (filters releases by id) | The new copy updates only from new-id releases. The old copy stops seeing updates once releases switch id, which is fine because it is retired. |
| Recents and app list | `src/recents.rs`, `src/platform.rs` (hides Luma from its own lists) | Hides the new id; should also hide the old id while both are installed. |
| Tests and CI | `src/update.rs`, `tests/*.mjs`, `.github/workflows/*.yml` | Mechanical. |

About 25 files, mostly a constant. The real work is the data copy and the takeover handover.

## Risks and how each is covered

| Risk | How likely | Effect | Cover |
|---|---|---|---|
| Settings or wallpapers missing after the move | Medium without care | Annoying, recoverable | Copy, never move. Verify counts and sizes before using the copy. B07 settings export first as a second backup. Old folder untouched until Phase 4. |
| Not enough space to copy the wallpapers | Low | Copy fails | Check free space first. If short, stop with a clear message; nothing changes. |
| Both copies try to take over Home | Medium if done carelessly | Home flicker or LG Home showing | The new copy ships with takeover locked off until the cutover. Cutover = turn the old one off, then the new one on. |
| New copy's takeover fails on the TV | Low (same code, new path) | Home press goes to LG Home | Roll back: turn the old copy's takeover back on. The old copy is still installed with its own data. |
| Boot hook or lifecycle marker left pointing at the old copy | Medium | Slow Home or wrong app at boot | Cutover step rewrites the hook and clears both markers; checklist verifies after a reboot. |
| Something only visible on the real TV | Medium | Varies | Phase 2 runs both side by side for days before anything depends on the new copy. |

The current setup is never at risk before Phase 3, and Phase 3 has a one-step rollback.

## Phases

0. **Prep (no TV change).** B07 settings export/import, so a backup exists independent of the move.
   Make the id a single build constant; add "copy data from the old id on first start" with
   checks; teach the new build to keep takeover off while the old copy owns it. All tested on the
   desktop and in the GitHub ARM build.
1. **Build the new-id package** under a new name (for example "My Luma (new)") with takeover
   locked off. Released separately; the 10.x old-id releases continue in parallel for now.
2. **Side by side on the TV.** Install the new package (Homebrew Channel or Dev Manager). Open it
   from the app list. It copies settings and wallpapers on first start. Checklist: wallpapers
   and collections, Unsplash, weather, tiles, inputs, ambient, sounds, updater. Run it for a few
   days; the old copy stays Home throughout.
3. **Cutover (about 5 minutes).** In the old copy: Home takeover off. In the new copy: Home
   takeover on, boot hook on. Reboot and run the checklist. Rollback is the reverse.
4. **Retire (after a week or two).** Remove the old copy. Its data folder goes to a `_to_delete`
   folder, never straight to deletion.

## Effort

- Claude: Phases 0–1 are about two working sessions, with three releases (B07, the migration
  code, the new-id package), each passing the GitHub ARM build.
- Stephen: install one package (Phase 2), about 15 minutes of checklist; the 5-minute cutover
  (Phase 3); one uninstall later (Phase 4).

## Open decisions for Stephen

- The new id. Suggestion: `com.theshadillac.myluma`.
- The app name shown in LG's app list during side-by-side ("My Luma (new)" until cutover, then
  "My Luma").
- When to start Phase 0. B07 is useful on its own and could go first.
