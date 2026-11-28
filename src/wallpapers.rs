//! Immutable, app-owned wallpapers and a short-lived, capability-protected LAN upload server.
//! Network I/O, decoding, normalization, metadata discovery and QR generation stay on this worker.
pub use crate::companion::Session;
use crate::companion::{
    Request, error_response, listen, local_peer, random_token, read_exact, redirect_to_page,
    response, session,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufReader, Cursor, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

const MAX_BODY: usize = 12 * 1024 * 1024;
// Room for a Photos library plus theme and holiday collections; files are 1080p JPEGs.
const MAX_WALLPAPERS: usize = 150;
/// Images dropped here (e.g. with webOS Dev Manager's file browser) are imported into the
/// gallery at start and whenever the gallery is opened, then removed (fork option).
pub const IMPORT_FOLDER: &str = "import";
/// Import accepts full-size photos: up to 40 MiB and 48 megapixels each.
const MAX_IMPORT_FILE: u64 = 40 * 1024 * 1024;
const MAX_IMPORT_PIXELS: u64 = 48_000_000;
const SESSION_TIME: Duration = Duration::from_secs(600);
// Room for the page's own files plus a full gallery's worth of uploads and retries.
const SESSION_REQUEST_LIMIT: usize = 160;
const REQUEST_TIME: Duration = Duration::from_secs(20);
const STOP: u64 = u64::MAX;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Wallpaper {
    pub id: String,
    /// The collection chosen on the phone when uploading (Photos when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    /// The photographer, for photos added from Unsplash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit: Option<crate::unsplash::Credit>,
    /// The Unsplash photo id, so Discover can tell a photo is already here (fork; photos
    /// added before 1.2.0 don't have it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsplash: Option<String>,
    #[serde(skip)]
    pub directory: PathBuf,
    #[serde(skip)]
    pub has_original: bool,
}

impl Wallpaper {
    pub fn unshaded_image(&self) -> String {
        // Preserve full resolution for older uploads without an unshaded original.
        self.directory
            .join(if self.has_original {
                "original.jpg"
            } else {
                "wallpaper.jpg"
            })
            .to_string_lossy()
            .into_owned()
    }
    pub fn image(&self) -> String {
        self.directory
            .join("wallpaper.jpg")
            .to_string_lossy()
            .into_owned()
    }
    pub fn thumbnail(&self) -> String {
        self.directory
            .join("thumbnail.jpg")
            .to_string_lossy()
            .into_owned()
    }
}

pub fn valid_id(id: &str) -> bool {
    id.strip_prefix("custom-").is_some_and(|id| {
        id.len() == 32
            && id
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

/// An Unsplash suggestion for the TV's Discover page, with its preview saved locally.
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub photo: crate::unsplash::Photo,
    pub thumbnail: PathBuf,
}

pub enum Event {
    Catalog(Vec<Wallpaper>),
    /// Discover suggestions for a page of a search.
    Suggestions(u32, Result<Vec<Suggestion>, String>),
    /// A suggestion was saved as a wallpaper (or why not).
    SuggestionAdded(String, Result<(), String>),
    Deleted(String, Result<(), String>),
    Session(u64, Result<Session, String>),
    Ended(u64),
    /// Settings restored from a backup on the phone page, already checked.
    Restore(Box<crate::model::Preferences>),
    /// Spotlight's Plex rows (or why not).
    Spotlight(Result<Vec<crate::plex::Row>, String>),
}

enum Command {
    Wake,
    Delete { id: String, selected: String },
    Suggest { query: String, page: u32 },
    AddSuggestion { id: String, collection: String },
    Spotlight,
}

pub struct Service {
    desired: Arc<AtomicU64>,
    unsplash_ready: Arc<std::sync::atomic::AtomicBool>,
    plex_ready: Arc<std::sync::atomic::AtomicBool>,
    wake: mpsc::Sender<Command>,
    pub receiver: mpsc::Receiver<Event>,
    active: bool,
    generation: u64,
}

impl Service {
    pub fn new(preferences: &Path, loopback_only: bool) -> Result<Self, String> {
        let directory = preferences
            .parent()
            .unwrap_or(Path::new("."))
            .join("wallpapers");
        let settings = preferences.to_path_buf();
        let desired = Arc::new(AtomicU64::new(0));
        let worker_desired = desired.clone();
        let unsplash_ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_unsplash = unsplash_ready.clone();
        let plex_ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_plex = plex_ready.clone();
        let (sender, receiver) = mpsc::sync_channel(4);
        let (wake, updates) = mpsc::channel();
        thread::Builder::new()
            .name("luma-wallpapers".into())
            .spawn(move || {
                worker(
                    &directory,
                    &settings,
                    &worker_desired,
                    Ready {
                        unsplash: &worker_unsplash,
                        plex: &worker_plex,
                    },
                    &sender,
                    &updates,
                    loopback_only,
                );
            })
            .map_err(|e| format!("Could not start wallpaper worker: {e}"))?;
        Ok(Self {
            desired,
            unsplash_ready,
            plex_ready,
            wake,
            receiver,
            active: false,
            generation: 0,
        })
    }

    pub fn set_active(&mut self, active: bool) {
        if self.active != active {
            self.active = active;
            self.generation += 1;
            self.desired
                .store(if active { self.generation } else { 0 }, Ordering::Release);
            let _ = self.wake.send(Command::Wake);
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Looks for new images in the import folder.
    pub fn refresh(&self) {
        let _ = self.wake.send(Command::Wake);
    }

    pub fn delete(&self, id: String, selected: String) {
        let _ = self.wake.send(Command::Delete { id, selected });
    }

    /// Whether a Plex token is saved on the TV (Spotlight's Plex rows can load).
    pub fn plex_ready(&self) -> bool {
        self.plex_ready.load(Ordering::Acquire)
    }

    /// Loads Spotlight's Plex rows; answered by `Event::Spotlight`.
    pub fn spotlight(&self) {
        let _ = self.wake.send(Command::Spotlight);
    }

    /// Whether an Unsplash access key is saved on the TV (Discover is offered).
    pub fn unsplash_ready(&self) -> bool {
        self.unsplash_ready.load(Ordering::Acquire)
    }

    /// Searches Unsplash for Discover; the answer arrives as `Event::Suggestions`.
    pub fn suggest(&self, query: String, page: u32) {
        let _ = self.wake.send(Command::Suggest { query, page });
    }

    /// Saves a Discover suggestion into a collection; answered by `Event::SuggestionAdded`.
    pub fn add_suggestion(&self, id: String, collection: String) {
        let _ = self.wake.send(Command::AddSuggestion { id, collection });
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.desired.store(STOP, Ordering::Release);
        let _ = self.wake.send(Command::Wake);
    }
}

fn random_id() -> Result<String, String> {
    let mut bytes = [0; 16];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| format!("Secure random source unavailable: {e}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

fn small_file(path: &Path, maximum: u64) -> Option<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > maximum {
        return None;
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= maximum).then_some(bytes)
}

fn catalog(directory: &Path) -> Vec<Wallpaper> {
    if !real_directory(directory) {
        return vec![];
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return vec![];
    };
    let mut result = Vec::new();
    // Bound discovery even if someone manually populates this private directory.
    for entry in entries.take(256).flatten() {
        let path = entry.path();
        let id = entry.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) || !real_directory(&path) {
            continue;
        }
        let Some(bytes) = small_file(&path.join("metadata.json"), 2048) else {
            continue;
        };
        let Ok(mut item) = serde_json::from_slice::<Wallpaper>(&bytes) else {
            continue;
        };
        // Legacy metadata may contain a name; serde ignores it. Identity and
        // selection depend only on the generated ID, never a display label.
        if item.id != id {
            continue;
        }
        if ["wallpaper.jpg", "thumbnail.jpg"].iter().any(|file| {
            !fs::symlink_metadata(path.join(file))
                .is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= MAX_BODY as u64)
        }) {
            continue;
        }
        item.has_original = fs::symlink_metadata(path.join("original.jpg"))
            .is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= MAX_BODY as u64);
        item.directory = path;
        result.push(item);
        if result.len() == MAX_WALLPAPERS {
            break;
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

fn delete_wallpaper(directory: &Path, id: &str, selected: &str) -> Result<(), String> {
    if id == selected || !valid_id(id) || !real_directory(directory) {
        return Err("Background cannot be deleted".into());
    }
    let path = directory.join(id);
    if !real_directory(&path) {
        return Err("Background directory unavailable".into());
    }
    // Only the strictly validated app-owned directory is removed; symlink
    // entries within it are unlinked, never followed by remove_dir_all.
    fs::remove_dir_all(path).map_err(|error| error.to_string())
}

/// Root Home creates files readable by the same app user after takeover is disabled.
fn create_directory(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => inherit_owner(path)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && real_directory(path) => {
        }
        Err(error) => return Err(error.to_string()),
    }
    if !real_directory(path) {
        return Err("Wallpaper storage is not a real directory".into());
    }
    Ok(())
}

fn inherit_owner(path: &Path) -> Result<(), String> {
    #[cfg(all(feature = "webos", unix))]
    if unsafe { libc::geteuid() } == 0 {
        use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
        let parent = path.parent().ok_or("Missing wallpaper storage parent")?;
        let owner = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
        if !owner.is_dir() {
            return Err("Invalid wallpaper storage parent".into());
        }
        use std::os::unix::fs::OpenOptionsExt;
        let file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|e| e.to_string())?;
        if unsafe { libc::fchown(file.as_raw_fd(), owner.uid(), owner.gid()) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    let _ = path;
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    inherit_owner(path)
}

fn decode(bytes: &[u8]) -> Result<image::RgbImage, String> {
    if bytes.len() > MAX_BODY {
        return Err("Image exceeds 12 MiB".into());
    }
    let format = image::guess_format(bytes).map_err(|_| "Choose a JPEG, PNG or WebP image")?;
    if !matches!(
        format,
        image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
    ) {
        return Err("Choose a JPEG, PNG or WebP image".into());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1920);
    limits.max_image_height = Some(1080);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("Invalid image: {e}"))?;
    use image::ImageDecoder;
    if decoder.dimensions() != (1920, 1080) {
        return Err("Crop the image to 1920 × 1080 before uploading".into());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    if image.width() != 1920 || image.height() != 1080 {
        return Err("Unsupported image orientation".into());
    }
    // Composite alpha against the launcher's base instead of exposing transparent pixels as black.
    let rgba = image.to_rgba8();
    Ok(image::RgbImage::from_fn(1920, 1080, |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let alpha = p[3] as f32 / 255.;
        image::Rgb(std::array::from_fn(|i| {
            (p[i] as f32 * alpha + [23., 25., 28.][i] * (1. - alpha)).round() as u8
        }))
    }))
}

/// Decodes a full-size photo of any shape, applies its orientation, and fills 1920 × 1080:
/// centre-cropped to 16:9, then resized. Transparency is composited like uploads.
fn decode_import(bytes: &[u8]) -> Result<image::RgbImage, String> {
    let format = image::guess_format(bytes).map_err(|_| "Not a JPEG, PNG or WebP image")?;
    if !matches!(
        format,
        image::ImageFormat::Jpeg | image::ImageFormat::Png | image::ImageFormat::WebP
    ) {
        return Err("Not a JPEG, PNG or WebP image".into());
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(12_000);
    limits.max_image_height = Some(12_000);
    limits.max_alloc = Some(MAX_IMPORT_PIXELS * 4 + 16 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| format!("Invalid image: {e}"))?;
    use image::ImageDecoder;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_IMPORT_PIXELS {
        return Err("Image is larger than 48 megapixels".into());
    }
    if width < 640 || height < 360 {
        return Err("Image is smaller than 640 × 360".into());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    let (width, height) = (image.width(), image.height());
    // The largest centred 16:9 region.
    let (crop_w, crop_h) = if u64::from(width) * 9 > u64::from(height) * 16 {
        ((u64::from(height) * 16 / 9) as u32, height)
    } else {
        (width, (u64::from(width) * 9 / 16) as u32)
    };
    let cropped = image.crop_imm((width - crop_w) / 2, (height - crop_h) / 2, crop_w, crop_h);
    let rgba = cropped
        .resize_exact(1920, 1080, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    Ok(image::RgbImage::from_fn(1920, 1080, |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let alpha = p[3] as f32 / 255.;
        image::Rgb(std::array::from_fn(|i| {
            (p[i] as f32 * alpha + [23., 25., 28.][i] * (1. - alpha)).round() as u8
        }))
    }))
}

/// Imports the images waiting in the import folder, oldest name first, until the gallery is
/// full. Each imported file is removed; one that cannot be read is renamed `*.failed` so it
/// is not retried. Returns how many were added.
fn import_folder(directory: &Path, mut added: impl FnMut()) -> usize {
    let folder = directory.join(IMPORT_FOLDER);
    if !folder.exists() {
        // Create it, so the folder is there to drop images into.
        if create_directory(directory).is_ok() {
            let _ = create_directory(&folder);
        }
        return 0;
    }
    if !real_directory(&folder) {
        return 0;
    }
    let Ok(entries) = fs::read_dir(&folder) else {
        return 0;
    };
    let mut files: Vec<PathBuf> = entries
        .take(512)
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                ["jpg", "jpeg", "png", "webp"].contains(&e.to_ascii_lowercase().as_str())
            }) && fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
        })
        .collect();
    files.sort();
    let mut count = 0;
    for path in files {
        if catalog(directory).len() >= MAX_WALLPAPERS {
            eprintln!("Luma wallpaper import: the gallery is full ({MAX_WALLPAPERS} images)");
            break;
        }
        let result = fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())
            .and_then(|m| {
                if m.len() > MAX_IMPORT_FILE {
                    Err("Image exceeds 40 MiB".into())
                } else {
                    fs::read(&path).map_err(|e| e.to_string())
                }
            })
            .and_then(|bytes| decode_import(&bytes))
            .and_then(|image| store_image(directory, image, || true, None, None, None));
        match result {
            Ok(_) => {
                let _ = fs::remove_file(&path);
                count += 1;
                added();
            }
            Err(error) => {
                eprintln!("Luma wallpaper import: {}: {error}", path.display());
                let mut failed = path.clone().into_os_string();
                failed.push(".failed");
                let _ = fs::rename(&path, failed);
            }
        }
    }
    count
}

fn shade(image: &mut image::RgbImage) {
    // Same base, image opacity and gradients as tools/assets.mjs. Only source-specific
    // saturation adjustments are omitted for owner-supplied photographs.
    let stops = [
        (0., [13., 16., 19.], 0.24),
        (0.19, [14., 17., 21.], 0.08),
        (0.35, [15., 17., 20.], 0.35),
        (0.63, [17., 19., 23.], 0.86),
        (0.85, [23., 25., 29.], 0.92157),
        (1., [23., 25., 29.], 0.96078),
    ];
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let y = y as f32 / 1079.;
        let pair = stops.windows(2).find(|s| y <= s[1].0).unwrap();
        let t = (y - pair[0].0) / (pair[1].0 - pair[0].0);
        let alpha = pair[0].2 * (1. - t) + pair[1].2 * t;
        let horizontal = 0.42 * (1. - x as f32 / (1919. * 0.75)).max(0.);
        for i in 0..3 {
            let base = pixel[i] as f32 * 0.92 + [23., 25., 28.][i] * 0.08;
            let base = base * (1. - horizontal) + [17., 20., 23.][i] * horizontal;
            // SVG gradients interpolate premultiplied colors.
            let gradient = pair[0].1[i] * pair[0].2 * (1. - t) + pair[1].1[i] * pair[1].2 * t;
            pixel[i] = (base * (1. - alpha) + gradient).round() as u8;
        }
    }
}

fn jpeg(image: &image::RgbImage, quality: u8) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
        .encode_image(image)
        .map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn store(
    directory: &Path,
    bytes: &[u8],
    valid: impl Fn() -> bool,
    collection: Option<String>,
) -> Result<Wallpaper, String> {
    if catalog(directory).len() >= MAX_WALLPAPERS {
        return Err(format!(
            "The wallpaper gallery is full ({MAX_WALLPAPERS} images)"
        ));
    }
    store_image(directory, decode(bytes)?, valid, collection, None, None)
}

/// Free space the TV must keep after wallpapers; a saved wallpaper takes a few MB.
const MIN_FREE_BYTES: u64 = 300_000_000;

/// Free bytes on the filesystem holding `path`, where the platform can tell.
#[cfg(feature = "webos")]
fn free_bytes(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    // SAFETY: statvfs is plain old data; all zeroes is a valid value.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: a valid C string and a statvfs owned by this frame.
    if unsafe { libc::statvfs(path.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    // The field types differ between 32-bit ARM and 64-bit desktop builds.
    #[allow(clippy::unnecessary_cast)]
    Some(stats.f_bavail as u64 * stats.f_frsize as u64)
}

#[cfg(not(feature = "webos"))]
fn free_bytes(_: &Path) -> Option<u64> {
    None
}

fn store_image(
    directory: &Path,
    mut image: image::RgbImage,
    valid: impl Fn() -> bool,
    collection: Option<String>,
    credit: Option<crate::unsplash::Credit>,
    unsplash: Option<String>,
) -> Result<Wallpaper, String> {
    if catalog(directory).len() >= MAX_WALLPAPERS {
        return Err(format!(
            "The wallpaper gallery is full ({MAX_WALLPAPERS} images)"
        ));
    }
    // Never fill the TV's storage with wallpapers (B118): skip the save and say so.
    if let Some(free) = free_bytes(directory)
        && free < MIN_FREE_BYTES
    {
        return Err(format!(
            "The TV is low on storage ({} MB free), so the wallpaper wasn't saved",
            free / 1_000_000
        ));
    }
    let original = jpeg(&image, 94)?;
    let thumbnail =
        image::imageops::resize(&image, 480, 270, image::imageops::FilterType::Triangle);
    shade(&mut image);
    let image = jpeg(&image, 94)?;
    let thumbnail = jpeg(&thumbnail, 82)?;
    if !valid() {
        return Err("Upload session ended".into());
    }
    let parent = directory
        .parent()
        .ok_or("Missing wallpaper directory parent")?;
    if !parent.exists() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if !real_directory(parent) {
        return Err("Invalid wallpaper storage parent".into());
    }
    create_directory(directory)?;
    let id = format!("custom-{}", random_id()?);
    let staging = directory.join(format!(".{id}"));
    create_directory(&staging)?;
    let item = Wallpaper {
        id: id.clone(),
        collection: collection.filter(|id| id != crate::collections::PHOTOS),
        credit,
        unsplash,
        directory: directory.join(&id),
        has_original: true,
    };
    let result = (|| {
        write_new(&staging.join("wallpaper.jpg"), &image)?;
        write_new(&staging.join("original.jpg"), &original)?;
        write_new(&staging.join("thumbnail.jpg"), &thumbnail)?;
        write_new(
            &staging.join("metadata.json"),
            &serde_json::to_vec(&item).map_err(|e| e.to_string())?,
        )?;
        if !valid() {
            return Err("Upload session ended".into());
        }
        fs::rename(&staging, &item.directory).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        // Only the files and fresh directory created by this operation.
        for file in [
            "wallpaper.jpg",
            "original.jpg",
            "thumbnail.jpg",
            "metadata.json",
        ] {
            let _ = fs::remove_file(staging.join(file));
        }
        let _ = fs::remove_dir(&staging);
    }
    result.map(|()| item)
}

fn worker(
    directory: &Path,
    settings: &Path,
    desired: &AtomicU64,
    ready: Ready<'_>,
    sender: &mpsc::SyncSender<Event>,
    updates: &mpsc::Receiver<Command>,
    loopback_only: bool,
) {
    let mut unsplash = crate::unsplash::Client::new(directory.parent().unwrap_or(directory));
    let mut plex = crate::plex::Client::new(directory.parent().unwrap_or(directory));
    ready.unsplash.store(unsplash.has_key(), Ordering::Release);
    ready.plex.store(plex.has_token(), Ordering::Release);
    let posters = directory.parent().unwrap_or(directory).join("spotlight");
    if sender.send(Event::Catalog(catalog(directory))).is_err() {
        return;
    }
    let previews = directory.parent().unwrap_or(directory).join("discover");
    // Each imported image shows in the gallery as soon as it is ready.
    let import = || {
        import_folder(directory, || {
            let _ = sender.send(Event::Catalog(catalog(directory)));
        });
    };
    import();
    let update = |command,
                  unsplash: &mut crate::unsplash::Client,
                  plex: &mut crate::plex::Client| match command {
        // Rows first, with the posters already saved, then again once the rest arrived.
        Command::Spotlight => match plex.rows(&posters) {
            Ok(mut rows) => {
                let shown = sender.send(Event::Spotlight(Ok(rows.clone()))).is_ok();
                plex.fill_posters(&mut rows, &posters);
                shown && sender.send(Event::Spotlight(Ok(rows))).is_ok()
            }
            Err(error) => sender.send(Event::Spotlight(Err(error))).is_ok(),
        },
        Command::Wake => {
            import();
            true
        }
        Command::Suggest { query, page } => {
            let result = suggest(unsplash, &previews, &query, page);
            sender.send(Event::Suggestions(page, result)).is_ok()
        }
        Command::AddSuggestion { id, collection } => {
            let result = unsplash.fetch(&id).and_then(|(bytes, credit)| {
                store_image(
                    directory,
                    decode(&bytes)?,
                    || true,
                    // Already a collection id, chosen on the TV.
                    Some(collection.clone()).filter(|id| !id.is_empty()),
                    Some(credit),
                    Some(id.clone()),
                )
            });
            let added = result.is_ok();
            sender
                .send(Event::SuggestionAdded(id, result.map(|_| ())))
                .is_ok()
                && (!added || sender.send(Event::Catalog(catalog(directory))).is_ok())
        }
        Command::Delete { id, selected } => {
            let result = delete_wallpaper(directory, &id, &selected);
            sender.send(Event::Deleted(id, result)).is_ok()
                && sender.send(Event::Catalog(catalog(directory))).is_ok()
        }
    };
    let mut handled = 0;
    loop {
        // Finish queued mutations before starting another upload session.
        for command in updates.try_iter() {
            if !update(command, &mut unsplash, &mut plex) {
                return;
            }
        }
        let generation = desired.load(Ordering::Acquire);
        if generation == STOP {
            break;
        }
        if generation == 0 || generation == handled {
            match updates.recv() {
                Ok(command) => {
                    if !update(command, &mut unsplash, &mut plex) {
                        break;
                    }
                }
                Err(_) => break,
            }
            continue;
        }
        handled = generation;
        let opened = (|| {
            let listener = listen(loopback_only)?;
            let token = random_token()?;
            let address = listener.local_addr().map_err(|e| e.to_string())?;
            let info = session(address, &token)?;
            Ok::<_, String>((listener, token, address, info))
        })();
        let (listener, token, address, info) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                let _ = sender.send(Event::Session(generation, Err(error)));
                continue;
            }
        };
        if sender.send(Event::Session(generation, Ok(info))).is_err() {
            break;
        }
        let deadline = Instant::now() + SESSION_TIME;
        let mut requests = 0;
        while desired.load(Ordering::Acquire) == generation
            && Instant::now() < deadline
            && requests < SESSION_REQUEST_LIMIT
        {
            match listener.accept() {
                Ok((mut stream, peer)) => {
                    if !local_peer(peer.ip()) {
                        continue;
                    }
                    requests += 1;
                    let valid = || {
                        desired.load(Ordering::Acquire) == generation && Instant::now() < deadline
                    };
                    let restore = |preferences| {
                        let _ = sender.send(Event::Restore(Box::new(preferences)));
                    };
                    match serve(
                        &mut stream,
                        &address.to_string(),
                        &token,
                        directory,
                        Backup {
                            settings,
                            restore: &restore,
                        },
                        &mut unsplash,
                        &mut plex,
                        valid,
                    ) {
                        Ok(true) => {
                            if sender.send(Event::Catalog(catalog(directory))).is_err() {
                                return;
                            }
                        }
                        Ok(false) => {}
                        Err(error) => eprintln!("Luma wallpaper upload: {error}"),
                    }
                    // The phone may have saved or removed the Unsplash key or Plex token.
                    ready.unsplash.store(unsplash.has_key(), Ordering::Release);
                    ready.plex.store(plex.has_token(), Ordering::Release);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25))
                }
                Err(error) => {
                    eprintln!("Luma wallpaper server: {error}");
                    break;
                }
            }
        }
        drop(listener);
        if sender.send(Event::Ended(generation)).is_err() {
            break;
        }
    }
}

/// What the worker tells the main loop about saved keys.
struct Ready<'a> {
    unsplash: &'a std::sync::atomic::AtomicBool,
    plex: &'a std::sync::atomic::AtomicBool,
}

fn request(
    reader: &mut impl Read,
    valid: &impl Fn() -> bool,
    deadline: Instant,
) -> Result<Request, String> {
    crate::companion::request(reader, valid, deadline, MAX_BODY, "Image exceeds 12 MiB")
}

/// The saved preferences file and how a checked restore reaches the main loop.
struct Backup<'a> {
    settings: &'a Path,
    restore: &'a dyn Fn(crate::model::Preferences),
}

/// Largest settings file the phone may restore.
const MAX_SETTINGS: usize = 512 * 1024;

/// A backup the phone sent back: must be a JSON object Luma itself wrote, with the usual keys,
/// read the same way as the preferences file. The Unsplash key is never part of it.
pub fn restorable(bytes: &[u8]) -> Result<crate::model::Preferences, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "That file isn't a Lume settings backup.")?;
    let object = value
        .as_object()
        .ok_or("That file isn't a Lume settings backup.")?;
    if !["language", "background", "favorites"]
        .iter()
        .all(|key| object.contains_key(*key))
    {
        return Err("That file isn't a Lume settings backup.".into());
    }
    let mut preferences: crate::model::Preferences =
        serde_json::from_value(value).map_err(|e| format!("That backup couldn't be read: {e}"))?;
    preferences.sanitize();
    Ok(preferences)
}

#[allow(clippy::too_many_arguments)]
fn serve(
    stream: &mut TcpStream,
    host: &str,
    token: &str,
    directory: &Path,
    backup: Backup<'_>,
    unsplash: &mut crate::unsplash::Client,
    plex: &mut crate::plex::Client,
    valid: impl Fn() -> bool,
) -> Result<bool, String> {
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + REQUEST_TIME;
    let mut reader = BufReader::new(&mut *stream);
    let request = match request(&mut reader, &valid, deadline) {
        Ok(request) => request,
        Err(error) => {
            error_response(stream, "400 Bad Request", &error)?;
            return Ok(false);
        }
    };
    if request.foreign(host) {
        error_response(stream, "403 Forbidden", "Open the address shown on your TV")?;
        return Ok(false);
    }
    if !valid() {
        return Err("Upload session ended".into());
    }
    if request.method == "GET" && request.length == 0 && request.target == format!("/{token}") {
        redirect_to_page(stream, token)?;
        return Ok(false);
    }
    let Some(route) = request.target.strip_prefix(&format!("/{token}/")) else {
        error_response(stream, "404 Not Found", "Upload session not found")?;
        return Ok(false);
    };
    if request.method == "GET" && request.length == 0 {
        if route == "info.json" {
            let items = catalog(directory);
            let free = MAX_WALLPAPERS.saturating_sub(items.len());
            // Collection names for the phone's picker: Photos, custom ones, then holidays.
            let mut custom: Vec<String> = items
                .iter()
                .filter_map(|item| item.collection.as_deref())
                .filter(|id| crate::collections::is_custom(id))
                .map(crate::collections::name)
                .collect();
            custom.sort_by_key(|name| name.to_lowercase());
            custom.dedup();
            let collections: Vec<String> = std::iter::once("Photos".to_string())
                .chain(custom)
                .chain(
                    crate::collections::HOLIDAYS
                        .iter()
                        .map(|(_, name)| (*name).to_string()),
                )
                .collect();
            let body = serde_json::json!({
                "path": directory.to_string_lossy(),
                "free": free,
                "collections": collections,
                "unsplash": unsplash.has_key(),
                "plex": plex.has_token(),
            })
            .to_string();
            response(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                body.as_bytes(),
            )?;
            return Ok(false);
        }
        if route == "settings.json" {
            // The saved file is current: every settings change is saved at once.
            match fs::read(backup.settings) {
                Ok(bytes) => response(stream, "200 OK", "application/json; charset=utf-8", &bytes)?,
                Err(error) => error_response(
                    stream,
                    "404 Not Found",
                    &format!("No saved settings yet ({error})"),
                )?,
            }
            return Ok(false);
        }
        let (kind, body): (&str, &[u8]) = match route {
            "" => (
                "text/html; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/index.html"),
            ),
            "app.css" => (
                "text/css; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/app.css"),
            ),
            "app.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/app.js"),
            ),
            "crop.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/crop.js"),
            ),
            "queue.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/queue.js"),
            ),
            "unsplash.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/unsplash.js"),
            ),
            "settings.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/settings.js"),
            ),
            "plex.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/plex.js"),
            ),
            "storage.js" => (
                "text/javascript; charset=utf-8",
                include_bytes!("../assets/wallpaper-upload/storage.js"),
            ),
            "luma-mark.svg" => ("image/svg+xml", include_bytes!("../assets/app-icon.svg")),
            _ => {
                error_response(stream, "404 Not Found", "Not found")?;
                return Ok(false);
            }
        };
        response(stream, "200 OK", kind, body)?;
        return Ok(false);
    }
    if request.method == "POST"
        && route == "settings/restore"
        && request.headers.get("x-luma-upload").map(String::as_str) == Some(token)
    {
        if request.length > MAX_SETTINGS {
            error_response(
                stream,
                "413 Content Too Large",
                "That file is too large to be settings",
            )?;
            return Ok(false);
        }
        let mut body = vec![0; request.length];
        read_exact(&mut reader, &mut body, &valid, deadline)?;
        drop(reader);
        match restorable(&body) {
            Ok(preferences) => {
                (backup.restore)(preferences);
                response(
                    stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    br#"{"ok":true}"#,
                )?;
            }
            Err(error) => error_response(stream, "422 Unprocessable Content", &error)?,
        }
        return Ok(false);
    }
    if request.method == "POST"
        && route == "plex/token"
        && request.headers.get("x-luma-upload").map(String::as_str) == Some(token)
    {
        if request.length > 1024 {
            error_response(stream, "413 Content Too Large", "Request too large")?;
            return Ok(false);
        }
        let mut body = vec![0; request.length];
        read_exact(&mut reader, &mut body, &valid, deadline)?;
        drop(reader);
        let input: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        let text = |name: &str| input[name].as_str().unwrap_or_default().to_owned();
        // The token is checked with the server and saved; it is never echoed back or logged.
        match plex.set_token(&text("token"), &text("server")) {
            Ok(()) => {
                let body = serde_json::json!({"ok": true, "plex": plex.has_token()}).to_string();
                response(
                    stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    body.as_bytes(),
                )?;
            }
            Err(error) => error_response(stream, "400 Bad Request", &error)?,
        }
        return Ok(false);
    }
    if request.method == "POST"
        && route.starts_with("unsplash/")
        && request.headers.get("x-luma-upload").map(String::as_str) == Some(token)
    {
        if request.length > 4096 {
            error_response(stream, "413 Content Too Large", "Request too large")?;
            return Ok(false);
        }
        let mut body = vec![0; request.length];
        read_exact(&mut reader, &mut body, &valid, deadline)?;
        drop(reader);
        let input: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        let text = |name: &str| input[name].as_str().unwrap_or_default().to_owned();
        let (result, changed) = match route {
            "unsplash/key" => (
                unsplash
                    .set_key(&text("key"))
                    .map(|()| serde_json::json!({"ok": true})),
                false,
            ),
            "unsplash/search" => (
                unsplash
                    .search(&text("query"), input["page"].as_u64().unwrap_or(1) as u32)
                    .map(|photos| {
                        // Photos already saved here, so the phone shows them as added.
                        let owned: serde_json::Map<String, serde_json::Value> = catalog(directory)
                            .into_iter()
                            .filter_map(|item| {
                                let id = item.unsplash?;
                                photos.iter().any(|photo| photo.id == id).then(|| {
                                    let collection = item
                                        .collection
                                        .as_deref()
                                        .unwrap_or(crate::collections::PHOTOS);
                                    (id, crate::collections::name(collection).into())
                                })
                            })
                            .collect();
                        serde_json::json!({"photos": photos, "owned": owned})
                    }),
                false,
            ),
            "unsplash/add" => {
                let collection = crate::collections::custom_id(&text("collection"));
                let photo = text("id");
                let added = unsplash.fetch(&photo).and_then(|(bytes, credit)| {
                    store_image(
                        directory,
                        decode(&bytes)?,
                        &valid,
                        collection,
                        Some(credit),
                        Some(photo.clone()),
                    )
                });
                let ok = added.is_ok();
                (added.map(|item| serde_json::json!({"id": item.id})), ok)
            }
            _ => (Err("Not found".to_string()), false),
        };
        match result {
            Ok(value) => response(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                value.to_string().as_bytes(),
            )?,
            Err(error) => error_response(stream, "422 Unprocessable Content", &error)?,
        }
        return Ok(changed);
    }
    if request.method != "POST"
        || route != "upload"
        || request.headers.get("x-luma-upload").map(String::as_str) != Some(token)
    {
        error_response(stream, "403 Forbidden", "Invalid upload request")?;
        return Ok(false);
    }
    if !request
        .headers
        .get("content-type")
        .is_some_and(|value| matches!(value.as_str(), "image/jpeg" | "image/png" | "image/webp"))
    {
        error_response(
            stream,
            "415 Unsupported Media Type",
            "Choose a JPEG, PNG or WebP image",
        )?;
        return Ok(false);
    }
    let mut body = vec![0; request.length];
    read_exact(&mut reader, &mut body, &valid, deadline)?;
    drop(reader);
    // The phone sends the collection name percent-encoded, as headers are ASCII.
    let collection = request
        .headers
        .get("x-luma-collection")
        .and_then(|name| percent_decode(name))
        .and_then(|name| crate::collections::custom_id(&name));
    match store(directory, &body, valid, collection) {
        Ok(item) => {
            // The commit succeeded even if the phone disconnects before receiving its reply.
            let _ = response(
                stream,
                "201 Created",
                "application/json; charset=utf-8",
                serde_json::json!({"id":item.id}).to_string().as_bytes(),
            );
            Ok(true)
        }
        Err(error) => {
            error_response(stream, "422 Unprocessable Content", &error)?;
            Ok(false)
        }
    }
}

/// Runs a Discover search and saves each result's preview where the TV can draw it.
fn suggest(
    unsplash: &mut crate::unsplash::Client,
    previews: &Path,
    query: &str,
    page: u32,
) -> Result<Vec<Suggestion>, String> {
    if page <= 1 {
        // A new search replaces the previews of the last one.
        if let Ok(entries) = fs::read_dir(previews) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|ext| ext == "jpg") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
    fs::create_dir_all(previews).map_err(|_| "Previews couldn't be saved on the TV")?;
    let photos = unsplash.search(query, page)?;
    Ok(photos
        .into_iter()
        .filter_map(|photo| {
            let thumbnail = previews.join(format!("{}.jpg", photo.id));
            if !thumbnail.exists() {
                let bytes = unsplash.thumbnail(&photo).ok()?;
                fs::write(&thumbnail, bytes).ok()?;
            }
            Some(Suggestion { photo, thumbnail })
        })
        .collect())
}

/// Decodes `encodeURIComponent` output; None for malformed input or invalid UTF-8.
fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
#[path = "wallpapers-tests.rs"]
mod tests;
