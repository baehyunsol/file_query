use colored::Color;
use crate::colors;
use crate::file::FileType;
use std::time::SystemTime;

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
        String::from("just now     ")
    }

    else if secs <= 99 {
        format!("{} seconds ago  ", secs)
    }

    else if secs <= 60 * 60 {
        format!("{} minutes ago  ", secs / 60)
    }

    else if secs <= 24 * 60 * 60 {
        format!("{} hours ago    ", secs / 3600)
    }

    else if secs <= 99 * 60 * 60 * 24 {
        format!("{} days ago     ", secs / 86400)
    }

    else if secs <= 99 * 60 * 60 * 24 * 7 {
        format!("{} weeks ago    ", secs / 604800)
    }

    // an average month is 2629746 seconds
    // it's okay to use the average value because the duration is long enough (at least 25 months)
    else if secs <= 99 * 2629746 {
        format!("{} months ago   ", secs / 2629746)
    }

    // an average year
    else if secs <= 99 * 31556952 {
        format!("{} years ago    ", secs / 31556952)
    }

    else {
        format!("{} centuries ago", secs / 3155695200)
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
    if size < 1024 {
        colors::GREEN
    }

    else if size < 33554432 {
        colors::WHITE
    }

    else if size < 1073741824 {
        colors::YELLOW
    }

    else {
        colors::RED
    }
}

pub fn colorize_time(now: &SystemTime, time: SystemTime) -> Color {
    let duration = now.duration_since(time).unwrap();
    let secs = duration.as_secs();

    if secs < 10 {
        colors::GREEN
    }

    else if secs < 60 * 60 * 24 * 7 * 3 {
        colors::WHITE
    }

    else if secs < 60 * 60 * 24 * 99 {
        colors::YELLOW
    }

    else {
        colors::RED
    }
}
