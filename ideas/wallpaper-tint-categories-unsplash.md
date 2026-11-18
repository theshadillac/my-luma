# Idea: time-of-day tint, wallpaper categories, Unsplash source

Status: idea from scout chat 2026-11-18. Not started. Take it to the main chat to decide.

## 1. Time-of-day tint (chosen: tint over the current wallpaper, not a replacement)
- A faint colour wash over whatever wallpaper is showing: rose at dawn, neutral at midday, amber at golden hour, darker and bluer at night.
- Timing comes from real sunrise and sunset for the weather location, with a fixed schedule as the fallback.
- Settings: Off (default) / Subtle / Medium. Applies on Home and in ambient mode.
- Must keep day/night wallpapers, the wallpaper cycle, the gradient and weather effects working exactly as they do now.
- Inspired by Halcyon's "living sky" (https://github.com/ipatek420/halcyon), which is a backdrop that replaces the wallpaper. Ours would be an overlay.

## 2. Wallpaper categories
- Tag each wallpaper with one or more categories (Landscape, Abstract, Art, Space, Personal, plus custom).
- The cycle can be limited to selected categories.
- The phone upload page gets a category picker for multi-select uploads.
- Categories sit alongside the existing day/night sets; they don't replace them.

## 3. Unsplash as a source
- Unsplash could be a category, or it could fill other categories (Landscape and Abstract mapped to Unsplash topics or searches).
- It pulls a few new landscape-orientation photos per day at TV resolution.
- "Keep this one" saves a photo to the local library.
- Needs a free Unsplash developer access key, stored in settings and never logged. The demo tier rate limit is far above a daily pull.
- The Unsplash API guidelines (https://help.unsplash.com/api-guidelines) require crediting the photographer and Unsplash (a small caption), using the image URLs the API returns, and calling the download-tracking endpoint when a photo is used. Check whether caching photos on the TV for offline use is allowed before relying on it.
- Sources that need no key: Bing photo of the day, Wikimedia featured pictures, and NASA APOD, which uses a free key.

## Open questions
- Unsplash as its own category, or mixed into the topic categories?
- How many new photos per day, and how many to keep before rotating out?
- Should the credit caption show on Home, only in ambient, or both?
