//! Spotlight's Plex rows (fork): the user's own Plex Media Server, read over the home network.
//! The token lives only on the TV, in a 0600 file next to the preferences, like the Unsplash
//! key; it is never logged, shown or part of a settings backup.
use serde::Serialize;
use serde_json::Value;
use std::{
    io::Read,
    net::{SocketAddr, UdpSocket},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Items per row, and the most posters fetched for one refresh.
pub const ROW_LIMIT: usize = 20;
const POSTER_LIMIT: u64 = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item {
    /// Plex's id for the item (ratingKey); also names its cached poster.
    pub key: String,
    pub title: String,
    /// "S2 · E5 · Episode name", the year, or the season.
    pub subtitle: String,
    /// How far into it the user is, 0 to 1; 0 when not started.
    pub progress: f32,
    /// Where the poster comes from: a path on the server or a web address.
    #[serde(skip)]
    pub(crate) poster_path: Option<String>,
    /// The poster saved on the TV, once fetched.
    pub poster: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Row {
    pub title: String,
    /// The Lume brand key of the service this row is about ("prime-video"); its cards open
    /// that app. None opens Plex.
    pub service: Option<&'static str>,
    pub items: Vec<Item>,
}

/// Plex's discovery service: trending and recommendations for the user's chosen services.
const DISCOVER: &str = "https://discover.provider.plex.tv";

/// The discovery service's address; desktop test builds can point it at a local stand-in.
fn discover_base() -> String {
    if cfg!(debug_assertions)
        && let Ok(base) = std::env::var("LUMA_PLEX_DISCOVER")
    {
        return base;
    }
    DISCOVER.to_owned()
}
/// Discovery rows shown, in this order; "Trending on <service>" rows follow the first three.
const DISCOVER_HUBS: [&str; 4] = [
    "home.trending-for-you",
    "home.recommended-on-services",
    "home.new-for-you",
    "home.watchlist-available",
];
const DISCOVER_LATE: [&str; 1] = ["home.top_watchlisted"];

fn row(title: &str, service: Option<&'static str>, items: Vec<Item>) -> Row {
    Row {
        title: title.to_owned(),
        service,
        items,
    }
}

/// Plex's service names, as Lume's brands (whose apps a card can open).
fn service_brand(platform: &str) -> Option<&'static str> {
    Some(match platform {
        "amazon-prime-video" => "prime-video",
        "netflix" => "netflix",
        "appletv" | "apple-tv-plus" => "apple-tv",
        "disney-plus" => "disney-plus",
        "max" | "hbo-max" => "hbo-max",
        "hulu" => "hulu",
        "paramount-plus" => "paramount-plus",
        "peacock" => "peacock",
        "crunchyroll" => "crunchyroll",
        _ => return None,
    })
}

pub struct Client {
    agent: ureq::Agent,
    token_path: PathBuf,
    server_path: PathBuf,
    /// The server found last, so a refresh needn't search the network again.
    server: Option<String>,
}

impl Client {
    pub fn new(preferences_directory: &Path) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(8)))
                .max_redirects(0)
                .proxy(None)
                .build()
                .new_agent(),
            token_path: preferences_directory.join("plex-token"),
            server_path: preferences_directory.join("plex-server"),
            server: None,
        }
    }

    fn token(&self) -> Option<String> {
        let token = std::fs::read_to_string(&self.token_path).ok()?;
        let token = token.trim();
        valid_token(token).then(|| token.to_owned())
    }

    pub fn has_token(&self) -> bool {
        self.token().is_some()
    }

    /// A server address typed on the phone page, if any (http://host:port).
    fn fixed_server(&self) -> Option<String> {
        let saved = std::fs::read_to_string(&self.server_path).ok()?;
        valid_server(saved.trim()).then(|| saved.trim().to_owned())
    }

    /// Saves (or, when empty, removes) the token after checking it against the server.
    /// `server` is optional: empty means find the server on the home network.
    pub fn set_token(&mut self, token: &str, server: &str) -> Result<(), String> {
        let token = token.trim();
        let server = server.trim().trim_end_matches('/');
        if token.is_empty() {
            let _ = std::fs::remove_file(&self.token_path);
            let _ = std::fs::remove_file(&self.server_path);
            self.server = None;
            return Ok(());
        }
        if !valid_token(token) {
            return Err("That doesn't look like a Plex token".into());
        }
        let server = if server.is_empty() {
            discover().ok_or("No Plex server found on the home network. Enter its address.")?
        } else {
            let server = if server.contains("://") {
                server.to_owned()
            } else if server.contains(':') {
                format!("http://{server}")
            } else {
                format!("http://{server}:32400")
            };
            if !valid_server(&server) {
                return Err("Enter the server as an address like 10.0.0.5:32400".into());
            }
            server
        };
        // One small call proves the token and the address before either is kept.
        self.get_json(&server, token, "/library/sections")?;
        crate::unsplash::write_private(&self.token_path, token.as_bytes())?;
        crate::unsplash::write_private(&self.server_path, server.as_bytes())?;
        self.server = Some(server);
        Ok(())
    }

    fn server(&mut self) -> Option<String> {
        if self.server.is_none() {
            self.server = self.fixed_server().or_else(discover);
        }
        self.server.clone()
    }

    /// Spotlight's rows, with the posters already saved; `fill_posters` fetches the rest.
    pub fn rows(&mut self, posters: &Path) -> Result<Vec<Row>, String> {
        let loading = Instant::now();
        let token = self
            .token()
            .ok_or("Add your Plex token on the phone page first")?;
        let server = self.server();
        let mut rows = Vec::new();
        let mut local_error = None;
        if let Some(server) = &server {
            match self.get_json(server, &token, "/library/onDeck") {
                Ok(deck) => {
                    let continuing: Vec<Item> = items(&deck)
                        .into_iter()
                        .filter(|item| item.progress > 0.)
                        .collect();
                    rows.push(row("Continue watching", None, continuing));
                }
                Err(error) => {
                    // The server may have moved (new address): look again next time.
                    self.server = None;
                    local_error = Some(error);
                }
            }
        }
        let discovered = self.discover_rows(&token);
        let reached_discover = !discovered.is_empty();
        rows.extend(discovered);
        if let Some(server) = &server
            && local_error.is_none()
        {
            rows.extend(self.library_rows(server, &token));
        }
        for row in &rows {
            eprintln!("Luma spotlight: {}: {} cards", row.title, row.items.len());
        }
        eprintln!(
            "Luma spotlight: {} rows in {} ms",
            rows.len(),
            loading.elapsed().as_millis()
        );
        rows.retain(|row| !row.items.is_empty());
        if rows.is_empty() {
            return Err(local_error
                .or((!reached_discover).then(|| "Lume couldn't reach Plex".to_string()))
                .unwrap_or_else(|| "Nothing to show yet".into()));
        }
        for item in rows.iter_mut().flat_map(|row| row.items.iter_mut()) {
            let file = posters.join(format!("{}.jpg", item.key));
            if file.exists() {
                item.poster = Some(file);
            }
        }
        Ok(rows)
    }

    /// Fetches missing posters into `posters`, then removes the ones no longer shown.
    pub fn fill_posters(&mut self, rows: &mut [Row], posters: &Path) {
        let _ = std::fs::create_dir_all(posters);
        let token = self.token();
        let server = self.server.clone();
        let mut keep = Vec::new();
        for item in rows.iter_mut().flat_map(|row| row.items.iter_mut()) {
            let file = posters.join(format!("{}.jpg", item.key));
            if !file.exists()
                && let Some(path) = &item.poster_path
                && let Ok(bytes) = self.poster(server.as_deref(), token.as_deref(), path)
            {
                let _ = std::fs::write(&file, bytes);
            }
            if file.exists() {
                item.poster = Some(file.clone());
                keep.push(file);
            }
        }
        if let Ok(entries) = std::fs::read_dir(posters) {
            for entry in entries.flatten() {
                if !keep.contains(&entry.path()) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    /// Plex's discovery rows for the services chosen in the user's Plex account.
    fn discover_rows(&self, token: &str) -> Vec<Row> {
        let home = match self.get_json(&discover_base(), token, "/hubs/sections/home") {
            Ok(home) => home,
            Err(error) => {
                eprintln!("Luma spotlight: Plex Discover: {error}");
                return Vec::new();
            }
        };
        discover_hubs(&home)
            .into_iter()
            .filter_map(|(key, title, service)| {
                let started = Instant::now();
                match self.get_json(&discover_base(), token, &key) {
                    Ok(list) => Some(row(&title, service, items(&list))),
                    Err(error) => {
                        eprintln!(
                            "Luma spotlight: {title}: {error} after {} ms",
                            started.elapsed().as_millis()
                        );
                        None
                    }
                }
            })
            .collect()
    }

    /// From the user's own movie and TV libraries: top rated not yet watched, most played.
    fn library_rows(&self, server: &str, token: &str) -> Vec<Row> {
        let sections = match self.get_json(server, token, "/library/sections") {
            Ok(sections) => sections,
            Err(error) => {
                eprintln!("Luma spotlight: libraries: {error}");
                return Vec::new();
            }
        };
        let libraries: Vec<(String, &str)> = sections["MediaContainer"]["Directory"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|section| {
                let key = section["key"].as_str()?;
                let kind = match section["type"].as_str()? {
                    "movie" => "1",
                    "show" => "2",
                    _ => return None,
                };
                key.chars()
                    .all(|c| c.is_ascii_digit())
                    .then(|| (key.to_owned(), kind))
            })
            .collect();
        let gather = |query: &str, score: &dyn Fn(&Value) -> f64| {
            let mut found: Vec<(f64, Item)> = libraries
                .iter()
                .filter_map(|(key, kind)| {
                    self.get_json(
                        server,
                        token,
                        &format!("/library/sections/{key}/all?type={kind}&{query}"),
                    )
                    .map_err(|error| eprintln!("Luma spotlight: library {key}: {error}"))
                    .ok()
                })
                .flat_map(|list| {
                    list["MediaContainer"]["Metadata"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                })
                .filter_map(|value| Some((score(&value), item(&value)?)))
                .collect();
            found.sort_by(|a, b| b.0.total_cmp(&a.0));
            let mut seen = std::collections::HashSet::new();
            found
                .into_iter()
                .map(|(_, item)| item)
                .filter(|item| seen.insert(item.key.clone()))
                .take(ROW_LIMIT)
                .collect()
        };
        let rating = |value: &Value| {
            value["audienceRating"]
                .as_f64()
                .or(value["rating"].as_f64())
                .unwrap_or(0.)
        };
        let plays = |value: &Value| value["viewCount"].as_f64().unwrap_or(0.);
        vec![
            row(
                "Top rated in your library",
                None,
                gather("unwatched=1&sort=audienceRating:desc", &rating),
            ),
            row(
                "Most played in your library",
                None,
                gather("sort=viewCount:desc", &plays),
            ),
        ]
    }

    /// A small poster (about 240 × 360). TMDB posters come straight from TMDB at a small
    /// size (no token sent); others are resized by the user's own server.
    fn poster(
        &self,
        server: Option<&str>,
        token: Option<&str>,
        path: &str,
    ) -> Result<Vec<u8>, String> {
        let request = if let Some(name) = path.strip_prefix("https://image.tmdb.org/t/p/original/")
        {
            self.agent
                .get(&format!("https://image.tmdb.org/t/p/w342/{name}"))
        } else {
            let (Some(server), Some(token)) = (server, token) else {
                return Err("No server to resize the poster".into());
            };
            self.agent
                .get(&format!("{server}/photo/:/transcode"))
                .query("width", "240")
                .query("height", "360")
                .query("minSize", "1")
                .query("upscale", "1")
                .query("url", path)
                .header("X-Plex-Token", token)
                .header("X-Plex-Product", "Lume")
                .header("X-Plex-Client-Identifier", "lume-tv")
        };
        let mut response = request.call().map_err(|_| "A poster couldn't be loaded")?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(POSTER_LIMIT)
            .read_to_end(&mut bytes)
            .map_err(|_| "A poster couldn't be loaded")?;
        Ok(bytes)
    }

    fn get_json(&self, server: &str, token: &str, path: &str) -> Result<Value, String> {
        let unreachable = |error: ureq::Error| match error {
            ureq::Error::StatusCode(401) => "Plex didn't accept the token".to_string(),
            _ => "Lume couldn't reach Plex".to_string(),
        };
        let mut url = format!("{server}{path}");
        let mut hops = 0;
        let mut response = loop {
            let response = self
                .agent
                .get(&url)
                .query("X-Plex-Container-Start", "0")
                .query("X-Plex-Container-Size", ROW_LIMIT.to_string())
                .header("Accept", "application/json")
                .header("X-Plex-Token", token)
                .header("X-Plex-Product", "Lume")
                .header("X-Plex-Client-Identifier", "lume-tv")
                .call()
                .map_err(unreachable)?;
            let mut response = response;
            if !response.status().is_redirection() {
                break response;
            }
            // Read the redirect's own reply to the end, so its connection is clean for the
            // next request.
            let _ = response
                .body_mut()
                .as_reader()
                .take(256 * 1024)
                .read_to_end(&mut Vec::new());
            // Plex's discovery service redirects to add the user's chosen services. Follow
            // only to the same host, so the token never goes anywhere else.
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_owned();
            let next = if location.starts_with('/') {
                format!("{server}{location}")
            } else {
                location
            };
            hops += 1;
            if hops > 3 || !next.starts_with(&format!("{server}/")) {
                return Err("Lume couldn't reach Plex".into());
            }
            url = next;
        };
        let limit = 4 * 1024 * 1024;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Plex sent something Lume couldn't read")?;
        if bytes.len() as u64 > limit {
            return Err("Plex's reply was too large".into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "Plex sent something Lume couldn't read".into())
    }
}

/// The discovery rows Spotlight shows, in order: (address, title, service brand). Trending
/// on each of the user's services follows the first three rows.
fn discover_hubs(home: &Value) -> Vec<(String, String, Option<&'static str>)> {
    let hubs = home["MediaContainer"]["Hub"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let hub = |id: &str| {
        hubs.iter()
            .find(|hub| hub["hubIdentifier"].as_str() == Some(id))
    };
    let services = hubs.iter().filter(|hub| {
        hub["hubIdentifier"]
            .as_str()
            .is_some_and(|id| id.starts_with("home.trend-"))
    });
    DISCOVER_HUBS[..3]
        .iter()
        .filter_map(|id| hub(id))
        .chain(services)
        .chain(DISCOVER_HUBS[3..].iter().filter_map(|id| hub(id)))
        .chain(DISCOVER_LATE.iter().filter_map(|id| hub(id)))
        .filter_map(|hub| {
            let key = hub["key"].as_str()?;
            (key.starts_with("/hubs/sections/home/")
                && key.len() <= 120
                && key[20..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c)))
            .then(|| {
                (
                    key.to_owned(),
                    hub["title"].as_str().unwrap_or_default().to_owned(),
                    hub["platform"].as_str().and_then(service_brand),
                )
            })
        })
        .collect()
}

fn valid_token(token: &str) -> bool {
    (8..=200).contains(&token.len())
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.~".contains(c))
}

fn valid_server(server: &str) -> bool {
    let Some(rest) = server.strip_prefix("http://") else {
        return false;
    };
    rest.len() <= 100
        && !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
}

/// The items of a Plex list, movies and TV only.
fn items(value: &Value) -> Vec<Item> {
    value["MediaContainer"]["Metadata"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(item)
        .take(ROW_LIMIT)
        .collect()
}

fn item(value: &Value) -> Option<Item> {
    let text = |name: &str| value[name].as_str().unwrap_or_default().to_owned();
    let number = |name: &str| value[name].as_i64();
    let key = text("ratingKey");
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let (title, subtitle, poster) = match value["type"].as_str()? {
        "movie" => (
            text("title"),
            number("year").map(|y| y.to_string()).unwrap_or_default(),
            text("thumb"),
        ),
        "episode" => {
            let mut parts = Vec::new();
            if let Some(season) = number("parentIndex") {
                parts.push(format!("S{season}"));
            }
            if let Some(episode) = number("index") {
                parts.push(format!("E{episode}"));
            }
            parts.push(text("title"));
            let show = text("grandparentThumb");
            (
                text("grandparentTitle"),
                parts.join(" · "),
                if show.is_empty() { text("thumb") } else { show },
            )
        }
        "season" => (text("parentTitle"), text("title"), text("thumb")),
        "show" => (
            text("title"),
            number("year").map(|y| y.to_string()).unwrap_or_default(),
            text("thumb"),
        ),
        _ => return None,
    };
    if title.is_empty() {
        return None;
    }
    let progress = match (number("viewOffset"), number("duration")) {
        (Some(offset), Some(duration)) if duration > 0 => {
            (offset as f32 / duration as f32).clamp(0., 1.)
        }
        _ => 0.,
    };
    Some(Item {
        key,
        title,
        subtitle,
        progress,
        poster_path: ((poster.starts_with('/') || poster.starts_with("https://"))
            && poster.len() < 300)
            .then_some(poster),
        poster: None,
    })
}

/// Finds a Plex Media Server on the home network with Plex's own discovery (GDM): a broadcast
/// on UDP 32414 that servers answer with their port.
fn discover() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.set_broadcast(true).ok()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(300)))
        .ok()?;
    let probe = b"M-SEARCH * HTTP/1.1\r\n\r\n";
    for target in ["255.255.255.255:32414", "239.0.0.250:32414"] {
        let _ = socket.send_to(probe, target);
    }
    let deadline = Instant::now() + Duration::from_millis(1500);
    let mut buffer = [0u8; 2048];
    while Instant::now() < deadline {
        let Ok((length, SocketAddr::V4(from))) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let reply = String::from_utf8_lossy(&buffer[..length]);
        if let Some(port) = gdm_port(&reply) {
            return Some(format!("http://{}:{port}", from.ip()));
        }
    }
    None
}

/// The port of a Plex Media Server's discovery reply.
fn gdm_port(reply: &str) -> Option<u16> {
    if !reply.contains("plex/media-server") {
        return None;
    }
    reply.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("port")
            .then(|| value.trim().parse().ok())
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn plex_lists_become_spotlight_items() {
        let deck = json!({"MediaContainer": {"Metadata": [
            {"type": "episode", "ratingKey": "101", "title": "Pilot", "grandparentTitle": "Slow Horses",
             "parentIndex": 2, "index": 5, "grandparentThumb": "/library/metadata/90/thumb/1",
             "viewOffset": 600000, "duration": 2400000},
            {"type": "movie", "ratingKey": "202", "title": "Dune", "year": 2021,
             "thumb": "/library/metadata/202/thumb/2"},
            {"type": "track", "ratingKey": "303", "title": "Song"},
            {"type": "movie", "ratingKey": "../x", "title": "Bad key"}
        ]}});
        let list = items(&deck);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "Slow Horses");
        assert_eq!(list[0].subtitle, "S2 · E5 · Pilot");
        assert!((list[0].progress - 0.25).abs() < 1e-6);
        assert_eq!(
            list[0].poster_path.as_deref(),
            Some("/library/metadata/90/thumb/1")
        );
        assert_eq!(list[1].title, "Dune");
        assert_eq!(list[1].subtitle, "2021");
        assert_eq!(list[1].progress, 0.);
        let season = json!({"MediaContainer": {"Metadata": [
            {"type": "season", "ratingKey": "404", "title": "Season 3", "parentTitle": "Bluey",
             "thumb": "/library/metadata/404/thumb/3"}
        ]}});
        assert_eq!(items(&season)[0].title, "Bluey");
        assert_eq!(items(&season)[0].subtitle, "Season 3");
        assert!(items(&json!({})).is_empty());
    }

    #[test]
    fn discovery_rows_follow_lumes_order_and_skip_ads() {
        let home = json!({"MediaContainer": {"Hub": [
            {"hubIdentifier": "home.recently-released-trailers", "key": "/hubs/sections/home/recently-released-trailers", "title": "New Trailers"},
            {"hubIdentifier": "home.top_watchlisted", "key": "/hubs/sections/home/top_watchlisted", "title": "Most Watchlisted This Week"},
            {"hubIdentifier": "home.trending-for-you", "key": "/hubs/sections/home/trending-for-you", "title": "Trending on Your Services"},
            {"hubIdentifier": "home.trend-amazon-prime-video", "key": "/hubs/sections/home/trend-amazon-prime-video", "title": "Trending on Prime Video", "platform": "amazon-prime-video"},
            {"hubIdentifier": "home.watchlist-available", "key": "/hubs/sections/home/watchlist-available", "title": "Available on Your Services"},
            {"hubIdentifier": "home.recommended-on-services", "key": "/hubs/sections/home/recommended-on-services", "title": "Recommended on Your Services"},
            {"hubIdentifier": "home.new-for-you", "key": "https://elsewhere.example/x", "title": "New on Your Services"},
            {"hubIdentifier": "home.festive-favorites", "key": "/hubs/sections/home/festive-favorites", "title": "Santa’s Nice List"}
        ]}});
        let hubs = discover_hubs(&home);
        let titles: Vec<&str> = hubs.iter().map(|(_, title, _)| title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Trending on Your Services",
                "Recommended on Your Services",
                "Trending on Prime Video",
                "Available on Your Services",
                "Most Watchlisted This Week"
            ]
        );
        assert_eq!(hubs[2].2, Some("prime-video"));
        let list = json!({"MediaContainer": {"Metadata": [
            {"ratingKey": "6b0a39ff7c1ba206aa77cd4a", "type": "placeholder", "url": "https://discover.provider.plex.tv/ad/tile"},
            {"ratingKey": "5d9c0855ffd9ef001e991de0", "type": "show", "title": "Bob's Burgers", "year": 2011,
             "thumb": "https://image.tmdb.org/t/p/original/iIHsQe3Qjs3NH62HdamyQEPeqTR.jpg"}
        ]}});
        let cards = items(&list);
        assert_eq!(cards.len(), 1, "the ad tile is skipped");
        assert_eq!(cards[0].title, "Bob's Burgers");
        assert_eq!(cards[0].subtitle, "2011");
        assert!(
            cards[0]
                .poster_path
                .as_deref()
                .unwrap()
                .starts_with("https://image.tmdb.org/")
        );
    }

    #[test]
    fn gdm_replies_and_inputs_are_checked() {
        let reply = "HTTP/1.0 200 OK\r\nContent-Type: plex/media-server\r\nName: Office Mac\r\nPort: 32400\r\n";
        assert_eq!(gdm_port(reply), Some(32400));
        assert_eq!(
            gdm_port("HTTP/1.0 200 OK\r\nContent-Type: plex/player\r\nPort: 32500\r\n"),
            None
        );
        assert!(valid_token("abcDEF123-_xyz"));
        assert!(!valid_token("short"));
        assert!(!valid_token("has space in it"));
        assert!(valid_server("http://10.0.0.5:32400"));
        assert!(!valid_server("https://10.0.0.5:32400"));
        assert!(!valid_server("http://evil/path?x"));
    }
}
