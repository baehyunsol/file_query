use colored::Color;
use crate::colors;
use crate::file::{File, FileType};
use crate::uid::Uid;
use crate::utils::get_path_by_uid;
use image::RgbImage;
use image::io::{Reader as ImageReader};
use std::time::{Duration, SystemTime};
use syntect::highlighting::Color as SyColor;

// the result must be right-aligned
pub fn prettify_size(size: u64) -> String {
    if size <= 9999 {
        format!("{size} B  ")
    }

    else if size <= 9999 << 10 {
        format!("{} KiB", size >> 10)
    }

    else if size <= 9999 << 20 {
        format!("{} MiB", size >> 20)
    }

    else if size <= 9999 << 30 {
        format!("{} GiB", size >> 30)
    }

    else {
        format!("{} TiB", size >> 40)
    }
}

pub fn prettify_time(now: &SystemTime, time: SystemTime) -> String {
    let duration = now.duration_since(time).unwrap();
    let secs = duration.as_secs();

    if secs < 5 {
        String::from("just now   ")
    }

    else if secs <= 99 {
        format!("{} seconds ago", secs)
    }

    else if secs <= 60 * 60 {
        format!("{} minutes ago", secs / 60)
    }

    else if secs <= 24 * 60 * 60 {
        format!("{} hours ago  ", secs / 3600)
    }

    else if secs <= 99 * 60 * 60 * 24 {
        format!("{} days ago   ", secs / 86400)
    }

    else if secs <= 99 * 60 * 60 * 24 * 7 {
        format!("{} weeks ago  ", secs / 604800)
    }

    // an average month is 2629746 seconds
    // it's okay to use the average value because the duration is long enough (at least 25 months)
    else if secs <= 99 * 2629746 {
        format!("{} months ago ", secs / 2629746)
    }

    // an average year
    else {
        format!("{} years ago  ", secs / 31556952)
    }
}

pub fn colorize_name(_: FileType, is_executable: bool) -> Color {
    if is_executable {
        colors::YELLOW
    }

    else {
        colors::WHITE
    }
}

pub fn colorize_type(ty: FileType) -> Color {
    match ty {
        FileType::File => colors::WHITE,
        FileType::Dir => colors::GREEN,
        FileType::Symlink => colors::YELLOW,
    }
}

pub fn colorize_size(size: u64) -> Color {
    if size < 9999 {
        colors::GREEN
    }

    else if size < 9999 << 10 {
        colors::WHITE
    }

    else if size < 9999 << 20 {
        colors::YELLOW
    }

    else {
        colors::RED
    }
}

pub fn colorize_time(now: &SystemTime, time: SystemTime) -> Color {
    let duration = now.duration_since(time).unwrap();
    let secs = duration.as_secs();

    if secs < 99 {
        colors::GREEN
    }

    else if secs < 24 * 60 * 60 {
        colors::WHITE
    }

    else if secs < 99 * 60 * 60 * 24 {
        colors::YELLOW
    }

    else {
        colors::RED
    }
}

pub fn try_extract_utf8_text(content: &[u8]) -> Option<String> {
    if content.len() < 6 {
        String::from_utf8(content.to_vec()).ok()
    }

    else if let Ok(s) = String::from_utf8(content.to_vec()) {
        Some(s)
    }

    else if let Ok(s) = String::from_utf8(content[..(content.len() - 1)].to_vec()) {
        Some(s)
    }

    else if let Ok(s) = String::from_utf8(content[..(content.len() - 2)].to_vec()) {
        Some(s)
    }

    // a valid utf-8 char uses at most 4 bytes
    else if let Ok(s) = String::from_utf8(content[..(content.len() - 3)].to_vec()) {
        Some(s)
    }

    else {
        None
    }
}

pub fn try_read_image(file: &File) -> Option<&CachedImage> {
    for (uid_, img) in unsafe { IMAGE_CACHE.iter() } {
        if *uid_ == file.uid {
            return Some(img);
        }
    }

    let path = if let Some(p) = get_path_by_uid(file.uid) {
        p
    } else {
        return None;
    };

    let image = if let Ok(img) = ImageReader::open(path) {
        img
    } else {
        return None;
    };

    let image = if let Ok(reader) = image.with_guessed_format() {
        reader
    } else {
        return None;
    };

    if let Ok(image) = image.decode() {
        let decoded_image = image.to_rgb8();
        let (w, h) = decoded_image.dimensions();

        // registers the image to the cache
        // if it's already registered, it does nothing
        register_image_to_cache(&decoded_image, file.uid);
        let cached_img = get_image_from_cache(file.uid);

        Some(cached_img)
    } else {
        None
    }
}

pub struct CachedImage {
    pub w: usize,
    pub h: usize,
    data: Vec<Color>,
}

impl CachedImage {
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        self.data[x + (y << 9)]
    }
}

// It can store up to 8 images
static mut IMAGE_CACHE: [(Uid, CachedImage); 8] = [
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
    (Uid::DUMMY, CachedImage { w: 0, h: 0, data: Vec::new() }),
];

// new image goes to here
static mut IMAGE_CACHE_CURSOR: usize = 0;

fn register_image_to_cache(img: &RgbImage, uid: Uid) {
    for (uid_, _) in unsafe { IMAGE_CACHE.iter() } {
        if *uid_ == uid {
            return;
        }
    }

    let mut buffer = Vec::with_capacity(1 << 18);
    let (real_w, real_h) = img.dimensions();

    for y in 0..512 {
        for x in 0..512 {
            let [r, g, b] = img.get_pixel(
                (x * real_w) >> 9,
                (y * real_h) >> 9,
            ).0;

            buffer.push(Color::TrueColor { r, g, b });
        }
    }

    unsafe {
        IMAGE_CACHE[IMAGE_CACHE_CURSOR] = (
            uid,
            CachedImage {
                w: real_w as usize,
                h: real_h as usize,
                data: buffer,
            },
        );
        IMAGE_CACHE_CURSOR = (IMAGE_CACHE_CURSOR + 1) & 7;
    }
}

// It panics if the image is not in the cache
fn get_image_from_cache<'a>(uid: Uid) -> &'a CachedImage {
    for (uid_, img) in unsafe { IMAGE_CACHE.iter() } {
        if *uid_ == uid {
            return img;
        }
    }

    panic!();
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs == 0 {
        format!("{} µs", duration.subsec_micros())
    }

    else if secs < 10 {
        let millis = duration.subsec_millis();

        format!("{secs}.{millis:03} seconds")
    }

    else {
        format!("{secs} seconds")
    }
}

pub fn convert_ocean_dark_color(c: SyColor) -> Color {
    if c.r > 190 && c.g > 190 && c.b > 190 {
        colors::WHITE
    }

    // not visible on my color scheme
    else if c.r < 60 && c.g < 60 && c.b < 60 {
        colors::YELLOW
    }

    else {
        // println!("r: {}, g: {}, b: {}", c.r, c.g, c.b);
        Color::TrueColor { r: c.r, g: c.g, b: c.b }
    }
}

// TODO: better implementation
pub fn split_long_str(s: String) -> Vec<String> {
    if s.len() < 60 {
        vec![s]
    }

    else {
        let char_count = s.chars().count();

        vec![
            vec![s.chars().collect::<Vec<char>>()[..(char_count >> 1)].iter().collect::<String>()],
            vec![s.chars().collect::<Vec<char>>()[(char_count >> 1)..].iter().collect::<String>()],
        ].concat()
    }
}
