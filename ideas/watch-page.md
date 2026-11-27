# Watch page (working name): discovery, recommendations and trending

Status: planning, 2026-11-27. Stephen approved storing a Plex token and a TMDB key on the TV (0600 files, like the Unsplash key; never in chats, logs or the repo).

## Idea
The app drawer gets its tab switcher back: Apps | Watch. Watch shows what is new, trending and worth watching across Stephen's services and Plex. Opt-in; Home untouched.

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
