//! Unsplash as a wallpaper source (fork feature), used by the phone upload page.
//!
//! The access key is entered once on the phone, kept in a private file next to the TV's
//! preferences and sent only to api.unsplash.com; it is never returned, shown or logged.
//! Following the API guidelines, photos are shown from the image URLs the API returns, each
//! photo kept on the TV triggers its download endpoint, and the photographer is credited.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const API: &str = "https://api.unsplash.com";
const JSON_LIMIT: u64 = 2 * 1024 * 1024;
const IMAGE_LIMIT: u64 = 12 * 1024 * 1024;
/// Referral parameters the guidelines ask for on links back to Unsplash.
const REFERRAL: &str = "utm_source=my_luma&utm_medium=referral";

/// Who took a photo, kept with the wallpaper and shown on screen.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Credit {
    pub name: String,
    /// The photographer's Unsplash profile, with referral parameters.
    pub link: String,
}

/// A search result as the phone sees it (no key, no download endpoint).
#[derive(Clone, Debug, Serialize)]
pub struct Photo {
    pub id: String,
    pub thumb: String,
    pub color: String,
    pub alt: String,
    pub name: String,
    pub profile: String,
    pub page: String,
}

#[derive(Clone, Debug)]
struct Source {
    raw: String,
    download_location: String,
    credit: Credit,
}

pub struct Client {
    agent: ureq::Agent,
    key_path: PathBuf,
    /// Search results by photo id, so adding one needs no second lookup.
    seen: HashMap<String, Source>,
}

impl Client {
    pub fn new(preferences_directory: &Path) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(15)))
                .max_redirects(2)
                .proxy(None)
                .build()
                .new_agent(),
            key_path: preferences_directory.join("unsplash-key"),
            seen: HashMap::new(),
        }
    }

    fn key(&self) -> Option<String> {
        let key = std::fs::read_to_string(&self.key_path).ok()?;
        let key = key.trim();
        valid_key(key).then(|| key.to_owned())
    }

    pub fn has_key(&self) -> bool {
        self.key().is_some()
    }

    /// Saves (or, when empty, removes) the access key after checking it with Unsplash.
    pub fn set_key(&mut self, key: &str) -> Result<(), String> {
        let key = key.trim();
        if key.is_empty() {
            let _ = std::fs::remove_file(&self.key_path);
            self.seen.clear();
            return Ok(());
        }
        if !valid_key(key) {
            return Err("That doesn't look like an Unsplash access key".into());
        }
        // One cheap call proves the key works before it is kept.
        self.get_json(key, "/photos", &[("per_page", "1")])?;
        write_private(&self.key_path, key.as_bytes())
    }

    pub fn search(&mut self, query: &str, page: u32) -> Result<Vec<Photo>, String> {
        let key = self.key().ok_or("Add your Unsplash access key first")?;
        let query: String = query.trim().chars().take(80).collect();
        if query.is_empty() {
            return Err("Type something to search for".into());
        }
        let page = page.clamp(1, 50).to_string();
        let value = self.get_json(
            &key,
            "/search/photos",
            &[
                ("query", &query),
                ("page", &page),
                ("per_page", "24"),
                ("orientation", "landscape"),
                ("content_filter", "high"),
            ],
        )?;
        let mut photos = Vec::new();
        for item in value["results"].as_array().into_iter().flatten() {
            let Some((photo, source)) = parse(item) else {
                continue;
            };
            self.seen.insert(photo.id.clone(), source);
            photos.push(photo);
        }
        if self.seen.len() > 2000 {
            self.seen.clear();
        }
        Ok(photos)
    }

    /// A search result's small preview, for drawing on the TV (about 400 px wide).
    pub fn thumbnail(&self, photo: &Photo) -> Result<Vec<u8>, String> {
        if !photo.thumb.starts_with("https://images.unsplash.com/") {
            return Err("Unexpected preview link".into());
        }
        let mut response = self
            .agent
            .get(&photo.thumb)
            .header("User-Agent", "MyLuma")
            .call()
            .map_err(|_| "The preview couldn't be downloaded")?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(2 * 1024 * 1024)
            .read_to_end(&mut bytes)
            .map_err(|_| "The preview couldn't be downloaded")?;
        Ok(bytes)
    }

    /// Downloads one result as a 1920 × 1080 JPEG and tells Unsplash it was used.
    pub fn fetch(&mut self, id: &str) -> Result<(Vec<u8>, Credit), String> {
        let key = self.key().ok_or("Add your Unsplash access key first")?;
        if !valid_id(id) {
            return Err("Unknown photo".into());
        }
        let source = match self.seen.get(id) {
            Some(source) => source.clone(),
            None => {
                let value = self.get_json(&key, &format!("/photos/{id}"), &[])?;
                let (_, source) = parse(&value).ok_or("Unsplash returned an unexpected photo")?;
                self.seen.insert(id.into(), source.clone());
                source
            }
        };
        let url = sized(&source.raw).ok_or("Unsplash returned an unexpected image link")?;
        let mut response = self
            .agent
            .get(&url)
            .header("User-Agent", "MyLuma")
            .call()
            .map_err(|_| "The photo couldn't be downloaded. Try again.")?;
        if !response.status().is_success() {
            return Err("The photo couldn't be downloaded. Try again.".into());
        }
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(IMAGE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The photo couldn't be downloaded. Try again.")?;
        if bytes.len() as u64 > IMAGE_LIMIT {
            return Err("The photo is too large".into());
        }
        // Required by the API guidelines whenever a photo is kept, e.g. as a wallpaper.
        if !source.download_location.starts_with(API) {
            return Err("Unsplash returned an unexpected download link".into());
        }
        let _ = self.get_json(&key, source.download_location.trim_start_matches(API), &[]);
        Ok((bytes, source.credit))
    }

    fn get_json(&self, key: &str, path: &str, query: &[(&str, &str)]) -> Result<Value, String> {
        let mut request = self
            .agent
            .get(&format!("{API}{path}"))
            .header("Accept", "application/json")
            .header("Accept-Version", "v1")
            .header("User-Agent", "MyLuma")
            .header("Authorization", &format!("Client-ID {key}"));
        for (name, value) in query {
            request = request.query(name, value);
        }
        let mut response = request.call().map_err(describe)?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(JSON_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Unsplash's reply couldn't be read")?;
        if bytes.len() as u64 > JSON_LIMIT {
            return Err("Unsplash's reply was too large".into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "Unsplash's reply couldn't be read".into())
    }
}

/// Says plainly what went wrong, so a rejected key can be told apart from a TV that can't
/// reach Unsplash. Never includes the key (it travels only in a header).
fn describe(error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(401) => {
            "Unsplash rejected the key (401). Copy the Access Key, not the Secret key.".into()
        }
        ureq::Error::StatusCode(403) => {
            "Unsplash refused (403): the hourly limit was reached, or the app isn't allowed yet."
                .into()
        }
        ureq::Error::StatusCode(code) => format!("Unsplash answered with an error ({code})."),
        ureq::Error::Timeout(_) => "The TV couldn't reach Unsplash in time.".into(),
        other => format!("The TV couldn't reach Unsplash: {other}"),
    }
}

/// Unsplash access keys are 43 URL-safe characters; allow some slack, nothing else.
fn valid_key(key: &str) -> bool {
    (20..=128).contains(&key.len())
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn referral(link: &str) -> String {
    format!(
        "{link}{}{REFERRAL}",
        if link.contains('?') { "&" } else { "?" }
    )
}

/// The raw image link sized and cropped to exactly the wallpaper size.
fn sized(raw: &str) -> Option<String> {
    raw.starts_with("https://images.unsplash.com/").then(|| {
        format!(
            "{raw}{}w=1920&h=1080&fit=crop&crop=entropy&fm=jpg&q=85",
            if raw.contains('?') { "&" } else { "?" }
        )
    })
}

fn parse(item: &Value) -> Option<(Photo, Source)> {
    let text = |value: &Value| value.as_str().map(str::to_owned);
    let id = text(&item["id"]).filter(|id| valid_id(id))?;
    let thumb = text(&item["urls"]["small"])?;
    let raw = text(&item["urls"]["raw"])?;
    let download_location = text(&item["links"]["download_location"])?;
    let name = text(&item["user"]["name"]).unwrap_or_else(|| "Unknown".into());
    let profile = referral(&text(&item["user"]["links"]["html"])?);
    let page = referral(&text(&item["links"]["html"])?);
    if !thumb.starts_with("https://images.unsplash.com/") {
        return None;
    }
    let photo = Photo {
        id,
        thumb,
        color: text(&item["color"]).unwrap_or_else(|| "#222".into()),
        alt: text(&item["alt_description"]).unwrap_or_default(),
        name: name.clone(),
        profile: profile.clone(),
        page,
    };
    let source = Source {
        raw,
        download_location,
        credit: Credit {
            name,
            link: profile,
        },
    };
    Some((photo, source))
}

pub fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| "The key couldn't be saved on the TV")?;
    use std::io::Write;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "The key couldn't be saved on the TV")?;
    std::fs::rename(&temporary, path).map_err(|_| "The key couldn't be saved on the TV".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_results_keep_credit_and_links_but_never_the_key() {
        let item = serde_json::json!({
            "id": "abc_123",
            "color": "#335577",
            "alt_description": "a red porsche",
            "urls": {
                "raw": "https://images.unsplash.com/photo-1?ixid=XYZ",
                "small": "https://images.unsplash.com/photo-1?ixid=XYZ&w=400"
            },
            "links": {
                "html": "https://unsplash.com/photos/abc_123",
                "download_location": "https://api.unsplash.com/photos/abc_123/download?ixid=XYZ"
            },
            "user": { "name": "Ana Lens", "links": { "html": "https://unsplash.com/@ana" } }
        });
        let (photo, source) = parse(&item).unwrap();
        assert_eq!(photo.name, "Ana Lens");
        assert_eq!(
            photo.profile,
            "https://unsplash.com/@ana?utm_source=my_luma&utm_medium=referral"
        );
        assert_eq!(source.credit.link, photo.profile);
        assert_eq!(
            sized(&source.raw).unwrap(),
            "https://images.unsplash.com/photo-1?ixid=XYZ&w=1920&h=1080&fit=crop&crop=entropy&fm=jpg&q=85"
        );
        let shown = serde_json::to_string(&photo).unwrap();
        assert!(!shown.contains("download"));
        // Links that don't point at Unsplash's image host are refused.
        assert_eq!(sized("https://example.com/x.jpg"), None);
        let mut foreign = item.clone();
        foreign["urls"]["small"] = "https://example.com/x.jpg".into();
        assert!(parse(&foreign).is_none());
    }

    #[test]
    fn keys_and_ids_are_checked_before_use() {
        assert!(valid_key("abcdefghijklmnopqrstuvwxyz_ABCDEFGH-123456"));
        assert!(!valid_key("short"));
        assert!(!valid_key("has spaces in it which are not allowed here"));
        assert!(!valid_id("../../etc"));
        assert!(valid_id("Xy-9_z"));
    }

    #[test]
    fn the_key_file_is_private_and_removable() {
        let directory = std::env::temp_dir().join(format!("luma-unsplash-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let mut client = Client::new(&directory);
        assert!(!client.has_key());
        write_private(
            &client.key_path,
            b"abcdefghijklmnopqrstuvwxyz_ABCDEFGH-123456",
        )
        .unwrap();
        assert!(client.has_key());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&client.key_path)
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        client.set_key("").unwrap();
        assert!(!client.has_key());
        let _ = std::fs::remove_dir_all(directory);
    }
}
