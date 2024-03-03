use colored::{Color, Colorize};
use crate::colors;
use crate::file::File;
use std::collections::{HashMap, HashSet};

mod config;
mod dir;
mod file;
mod link;
mod result;
mod utils;

const COLUMN_MARGIN: usize = 2;

pub use config::{
    ColumnKind,
    FileReadMode,
    PrintDirConfig,
    PrintFileConfig,
    PrintLinkConfig,
};
pub use dir::print_dir;
pub use file::print_file;
pub use link::print_link;
pub use result::{
    PrintDirResult,
    PrintFileResult,
    PrintLinkResult,
    ViewerKind,
};
use utils::split_long_str;

static mut SCREEN_BUFFER: Vec<String> = Vec::new();

macro_rules! print_to_buffer {
    ($($arg:tt)*) => {
        unsafe {
            SCREEN_BUFFER.push(format!($($arg)*));
        }
    };
}

// macro_rules! println_to_buffer {
//     ($($arg:tt)*) => {
//         print_to_buffer!($($arg)*);
//         print_to_buffer!("\n");
//     };
// }

#[derive(Clone)]
pub enum Alignment {
    Left, Center, Right,
}

pub fn print_error_message(
    file: Option<&File>,
    path: Option<String>,
    message: String,
    min_width: usize,
    max_width: usize,
) {
    let mut rows = vec![];

    if let Some(f) = file {
        let f_fmt = f.debug_info();

        for (index, line) in split_long_str(f_fmt).into_iter().enumerate() {
            rows.push(vec![
                if index == 0 { String::from("instance") } else { String::new() },
                String::from("│"),
                line,
            ]);
        }
    }

    if let Some(path) = path {
        for (index, line) in split_long_str(path).into_iter().enumerate() {
            rows.push(vec![
                if index == 0 { String::from("path") } else { String::new() },
                String::from("│"),
                line,
            ]);
        }
    }

    for (index, line) in split_long_str(message).into_iter().enumerate() {
        rows.push(vec![
            if index == 0 { String::from("message") } else { String::new() },
            String::from("│"),
            line,
        ]);
    }

    let column_widths = calc_table_column_widths(
        &rows,
        Some(max_width),
        Some(min_width),
        COLUMN_MARGIN,
    );
    let table_width = column_widths.get(&3).unwrap().iter().sum::<usize>() + COLUMN_MARGIN * 2;

    print_horizontal_line(
        None,
        table_width + COLUMN_MARGIN * 2,
        (true, false),
        (true, true),
    );
    print_row(
        colors::BLACK,
        &vec![String::from("error")],
        &vec![table_width],
        &vec![Alignment::Center],
        &vec![LineColor::All(colors::WHITE)],
        COLUMN_MARGIN,
        (true, true),
    );
    print_horizontal_line(
        None,
        table_width + COLUMN_MARGIN * 2,
        (false, false),
        (true, true),
    );

    for row in rows.iter() {
        print_row(
            colors::BLACK,
            row,
            column_widths.get(&row.len()).unwrap(),
            &vec![Alignment::Center, Alignment::Left, Alignment::Left],
            &vec![LineColor::All(colors::WHITE); 3],
            COLUMN_MARGIN,
            (true, true),
        );
    }

    print_horizontal_line(
        None,
        table_width + COLUMN_MARGIN * 2,
        (false, true),
        (true, true),
    );
}

// you can either
// 1. color the entire line with the same color
// 2. color each character
#[derive(Clone)]
pub enum LineColor {
    All(Color),
    Each(Vec<Color>),
}

fn print_row(
    background: Color,
    contents: &Vec<String>,
    widths: &Vec<usize>,
    alignments: &Vec<Alignment>,
    colors: &Vec<LineColor>,
    margin: usize,
    borders: (bool, bool),  // (left, right)
) {
    debug_assert_eq!(contents.len(), widths.len());
    debug_assert_eq!(contents.len(), alignments.len());
    debug_assert_eq!(contents.len(), colors.len());
    let mut curr_table_width = 0;

    if borders.0 {
        print_to_buffer!("│");
    }

    if contents.len() > 0 {
        print_to_buffer!(
            "{}",
            " ".repeat(margin).on_color(background),
        );

        curr_table_width += margin;
    }

    for i in 0..contents.len() {
        let curr_content_len = contents[i].chars().count();
        let mut parts = vec![];

        if curr_content_len <= widths[i] {
            let left_margin = match alignments[i] {
                Alignment::Left => 0,
                Alignment::Center => (widths[i] - curr_content_len) >> 1,
                Alignment::Right => widths[i] - curr_content_len,
            };
            let right_margin = widths[i] - curr_content_len - left_margin;

            match &colors[i] {
                LineColor::All(c) => {
                    parts.push(" ".repeat(left_margin).color(*c));
                    parts.push(contents[i].color(*c));
                    parts.push(" ".repeat(right_margin).color(*c));
                },
                LineColor::Each(colors) => {
                    debug_assert_eq!(
                        curr_content_len,
                        colors.len(),
                    );

                    // default color
                    parts.push(" ".repeat(left_margin).color(colors::WHITE));

                    for (idx, ch) in contents[i].chars().enumerate() {
                        parts.push(ch.to_string().color(colors[idx]));
                    }

                    // default color
                    parts.push(" ".repeat(right_margin).color(colors::WHITE));
                },
            }
        }

        else {
            // TODO: how do I make sure that widths[i] >= 3?
            let first_half = (widths[i] - 3) >> 1;
            let last_half = widths[i] - 3 - first_half;

            let prefix = &contents[i].chars().collect::<Vec<_>>()[..first_half];
            let suffix = &contents[i].chars().collect::<Vec<_>>()[(curr_content_len - last_half)..];

            match &colors[i] {
                LineColor::All(c) => {
                    parts.push(prefix.iter().collect::<String>().color(*c));
                    parts.push("...".color(colors::WHITE));
                    parts.push(suffix.iter().collect::<String>().color(*c));
                },
                LineColor::Each(colors) => {
                    debug_assert_eq!(
                        curr_content_len,
                        colors.len(),
                    );

                    let prefix_colors = colors[..first_half].to_vec();
                    let suffix_colors = colors[(curr_content_len - last_half)..].to_vec();

                    for i in 0..prefix.len() {
                        parts.push(prefix[i].to_string().color(prefix_colors[i]));
                    }

                    parts.push("...".color(colors::WHITE));

                    for i in 0..suffix.len() {
                        parts.push(suffix[i].to_string().color(suffix_colors[i]));
                    }
                },
            }
        }

        for part in parts.into_iter() {
            print_to_buffer!("{}", part.on_color(background));
        }

        print_to_buffer!(
            "{}",
            " ".repeat(margin).on_color(background),
        );

        curr_table_width += margin + widths[i];
    }

    if borders.1 {
        print_to_buffer!("│");
    }

    print_to_buffer!("\n");
}

fn print_horizontal_line(
    background: Option<Color>,
    width: usize,
    vertical_position: (bool, bool),  // (is top, is bottom)
    borders: (bool, bool),  // (left, right)
) {
    if borders.0 {  // left border
        if vertical_position.0 {  // is top
            print_to_buffer!("╭");
        }

        else if vertical_position.1 {  // is bottom
            print_to_buffer!("╰");
        }

        else {
            print_to_buffer!("├");
        }
    }

    if let Some(c) = background {
        print_to_buffer!("{}", "─".repeat(width).on_color(c));
    }

    else {
        print_to_buffer!("{}", "─".repeat(width));
    }

    if borders.1 {  // right border
        if vertical_position.0 {  // is top
            print_to_buffer!("╮");
        }

        else if vertical_position.1 {  // is bottom
            print_to_buffer!("╯");
        }

        else {
            print_to_buffer!("┤");
        }
    }

    print_to_buffer!("\n");
}

// it has some odd rules to follow...
// Let's say a row has 1 ~ M columns (1 <= M).
// 1. The first row must have M columns.
// 2. The other rows can have any number (1 ~ M) of columns.
// 3. If a row has N columns (N < M), the last column has rowspan (M - N + 1), and the other columns have rowspan 1.
fn calc_table_column_widths(
    table_contents: &Vec<Vec<String>>,
    max_width: Option<usize>,
    min_width: Option<usize>,
    column_margin: usize,
) -> HashMap<usize, Vec<usize>> {
    if let (Some(t), Some(m)) = (max_width, min_width) {
        assert!(t >= m);
    }

    let mut max_column_widths = table_contents[0].iter().map(|c| c.chars().count()).collect::<Vec<_>>();
    let mut col_counts = HashSet::new();
    col_counts.insert(table_contents[0].len());

    for row in table_contents[1..].iter() {
        let curr_row_widths = row.iter().map(|c| c.chars().count()).collect::<Vec<_>>();
        col_counts.insert(row.len());

        if curr_row_widths.len() == max_column_widths.len() {
            for i in 0..curr_row_widths.len() {
                max_column_widths[i] = max_column_widths[i].max(curr_row_widths[i]);
            }
        }

        else {
            for i in 0..(curr_row_widths.len() - 1) {
                max_column_widths[i] = max_column_widths[i].max(curr_row_widths[i]);
            }
        }
    }

    let mut max_total_width = max_column_widths.iter().sum::<usize>() + column_margin * (max_column_widths.len() + 1);

    if let Some(width) = max_width {
        if width < max_total_width {
            let mut diff = max_total_width - width;

            while diff > 0 {
                let mut did_something = false;

                for w in max_column_widths.iter_mut() {
                    if *w > 16 && diff > 0 {
                        *w -= 1;
                        diff -= 1;
                        did_something = true;
                    }
                }

                // I'd rather break the ui than showing too small columns
                if !did_something {
                    break;
                }
            }

            max_total_width = max_column_widths.iter().sum::<usize>() + column_margin * (max_column_widths.len() + 1);
        }
    }

    if let Some(width) = min_width {
        if width > max_total_width {
            let d = (width - max_total_width) / max_column_widths.len() + 1;

            for w in max_column_widths.iter_mut() {
                *w += d;
            }

            max_total_width = max_column_widths.iter().sum::<usize>() + column_margin * (max_column_widths.len() + 1);
        }
    }

    let mut result = HashMap::with_capacity(col_counts.len());

    for col_count in col_counts.into_iter() {
        let mut widths = Vec::with_capacity(col_count);
        let mut curr_total_width = 0;

        for i in 0..(col_count - 1) {
            widths.push(max_column_widths[i]);
            curr_total_width += max_column_widths[i];
        }

        widths.push(max_total_width - curr_total_width - column_margin * (col_count + 1));

        result.insert(
            col_count,
            widths
        );
    }

    result
}

pub fn flip_buffer(clear_screen: bool) {
    if clear_screen {
        clearscreen::clear().unwrap();
    }

    unsafe {
        for s in SCREEN_BUFFER.iter() {
            print!("{s}");
        }

        SCREEN_BUFFER.clear();
    }
}
