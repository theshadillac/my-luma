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
    #[serde(skip)]
    pub(crate) poster_path: Option<String>,
    /// The poster saved on the TV, once fetched.
    pub poster: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum RowKind {
    ContinueWatching,
    UpNext,
    RecentlyAdded,
}

impl RowKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::ContinueWatching => "Continue watching",
            Self::UpNext => "Up next",
            Self::RecentlyAdded => "Recently added",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Row {
    pub kind: RowKind,
    pub items: Vec<Item>,
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

    /// Spotlight's Plex rows, with posters saved into `posters`.
    pub fn rows(&mut self, posters: &Path) -> Result<Vec<Row>, String> {
        let token = self
            .token()
            .ok_or("Add your Plex token on the phone page first")?;
        let server = self.server().ok_or("Lume couldn't find your Plex server")?;
        let deck = self.get_json(&server, &token, "/library/onDeck");
        let added = self.get_json(&server, &token, "/library/recentlyAdded");
        if deck.is_err() && added.is_err() {
            // The server may have moved (new address): look again next time.
            self.server = None;
            return Err(deck.err().unwrap_or_default());
        }
        let (continuing, next): (Vec<Item>, Vec<Item>) = deck
            .map(|value| items(&value))
            .unwrap_or_default()
            .into_iter()
            .partition(|item| item.progress > 0.);
        let mut rows: Vec<Row> = [
            (RowKind::ContinueWatching, continuing),
            (RowKind::UpNext, next),
            (
                RowKind::RecentlyAdded,
                added.map(|value| items(&value)).unwrap_or_default(),
            ),
        ]
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(kind, items)| Row { kind, items })
        .collect();
        let _ = std::fs::create_dir_all(posters);
        let mut keep = Vec::new();
        for row in &mut rows {
            for item in &mut row.items {
                let file = posters.join(format!("{}.jpg", item.key));
                if !file.exists()
                    && let Some(path) = &item.poster_path
                    && let Ok(bytes) = self.poster(&server, &token, path)
                {
                    let _ = std::fs::write(&file, bytes);
                }
                if file.exists() {
                    keep.push(file.clone());
                    item.poster = Some(file);
                }
            }
        }
        // Posters of items no longer shown are removed, so the cache stays small.
        if let Ok(entries) = std::fs::read_dir(posters) {
            for entry in entries.flatten() {
                if !keep.contains(&entry.path()) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        Ok(rows)
    }

    /// A small poster (about 240 × 360) resized by the server.
    fn poster(&self, server: &str, token: &str, path: &str) -> Result<Vec<u8>, String> {
        let mut response = self
            .agent
            .get(&format!("{server}/photo/:/transcode"))
            .query("width", "240")
            .query("height", "360")
            .query("minSize", "1")
            .query("upscale", "1")
            .query("url", path)
            .header("X-Plex-Token", token)
            .header("X-Plex-Product", "Lume")
            .header("X-Plex-Client-Identifier", "lume-tv")
            .call()
            .map_err(|_| "A poster couldn't be loaded")?;
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
        let mut response = self
            .agent
            .get(&format!("{server}{path}"))
            .query("X-Plex-Container-Start", "0")
            .query("X-Plex-Container-Size", ROW_LIMIT.to_string())
            .header("Accept", "application/json")
            .header("X-Plex-Token", token)
            .header("X-Plex-Product", "Lume")
            .header("X-Plex-Client-Identifier", "lume-tv")
            .call()
            .map_err(|error| match error {
                ureq::Error::StatusCode(401) => "Plex didn't accept the token".to_string(),
                _ => "Lume couldn't reach your Plex server".to_string(),
            })?;
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
        poster_path: (poster.starts_with('/') && poster.len() < 300).then_some(poster),
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
