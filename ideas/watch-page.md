# Spotlight: discovery, recommendations and trending

Status: stage 1 released as Lume 1.3.0 (2026-11-27). Named Spotlight by Stephen. Planned 2026-11-27. Stephen approved storing a Plex token and a TMDB key on the TV (0600 files, like the Unsplash key; never in chats, logs or the repo).

## Idea
The app drawer gets its tab switcher back: Apps | Spotlight. Spotlight shows what is new, trending and worth watching across Stephen's services and Plex. Opt-in; Home untouched.

## Services (Stephen, 2026-11-27)
Main: YouTube TV, Plex, Netflix, Apple TV, Prime Video. Occasionally: Disney+, BritBox.
Wanted too: what's trending on network TV (he has YouTube TV).

## Data sources
- Plex (local, no internet): the server's hubs (continue watching, on deck, recently added, recommendations). Needs the X-Plex-Token. Server found by GDM discovery on the LAN, with a manual address as fallback.
- TMDB (internet, free key, attribution in Settings > About): trending movies and TV (day/week), watch providers per region (US, JustWatch data) to tag and filter by Stephen's services; airing today / on the air for broadcast networks (ABC, CBS, NBC, FOX, PBS, The CW) for the YouTube TV row.
- Cache everything on the TV; offline shows the last results.

## Opening a card (what is realistic)
- Netflix, Apple TV, Prime, Disney+, BritBox, YouTube TV: no public deep links into a title for third-party launchers on webOS. Opening a card opens the app; the card can show the title so it is easy to search.
- YouTube (not YouTube TV) accepts contentTarget "v=<id>" (trailers could open in YouTube).
- Plex for LG: external deep links reported as unsupported (Plex forum, Home Assistant). Test on the TV before promising; otherwise open Plex.

## Stages
1. Plex rows: Continue watching, Recently added, On deck. Tab switcher, poster cache, phone-page fields for the Plex token (and server address if discovery fails).
2. TMDB rows: Trending this week (filtered to his services, with service badges), New on network TV (YouTube TV).
3. Polish: hero banner with backdrop art, offline state, maybe "Leaving soon"/"New on <service>".

## Principles check
Opt-in (1, 4); posters lazy-loaded and cached, page built only when opened (2); two new secrets on the TV only, approved (5); Home untouched, page can be switched off (6).

## Stage 1 as built (1.3.0)
- Settings > App drawer > Spotlight tab (off by default). Tabs centred at the top of the drawer.
- Rows: Continue watching (on deck with progress), Up next (rest of on deck), Recently added. Posters 240x360 via /photo/:/transcode, cached in spotlight/ next to the wallpapers, pruned to what is shown.
- Server: phone-page address if given, else GDM. Token: phone page > Spotlight: Plex; plex-token and plex-server files, 0600.
- OK on a card opens Plex (cdp-30). Reload when shown after 5 minutes.
- Not yet tested against Stephen's real server.

## 1.3.1 and 1.4.0 (2026-11-27)
- 1.3.1: dark scrim behind Spotlight; "Drawer opens on: Apps / Spotlight".
- Stephen: Up next and Recently added not wanted; wants discovery like Plex's own home (trending, popular, top rated from his collection).
- Probed on the TV: discover.provider.plex.tv/hubs/sections/home answers for his token; each hub is a placeholder whose items come from its key, which 302-redirects to add his chosen services (aande, abc, amazon-prime-video, amc, appletv, disney-plus, max, hulu, nbc, netflix, paramount-network, paramount-plus, the-roku-channel, showtime, starz, sundance-now, vice-tv). Items: type movie/show/placeholder (ad tiles), thumb on image.tmdb.org or metadata-static.plex.tv. Per-service hubs carry "platform". Items don't name their service.
- 1.4.0 rows: Continue watching; Trending / Recommended / New on Your Services; Trending on <service>; Available on Your Services; Most Watchlisted This Week; Top rated (unwatched) and Most played in his libraries (library 1 movies, 2 and 7 TV). Service rows open the service app; others open Plex. TMDB key no longer needed for trending; only for network TV later.
