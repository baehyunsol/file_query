use super::{
    print_error_message,
    print_horizontal_line,
    print_row,
    Alignment,
    COLUMN_MARGIN,
    LineColor,
};
use super::config::PrintLinkConfig;
use super::result::PrintLinkResult;
use super::utils::prettify_size;
use crate::colors;
use crate::uid::Uid;
use crate::utils::{get_file_by_uid, get_path_by_uid};
use std::fs;

// macro_rules! print_to_buffer {
//     ($($arg:tt)*) => {
//         unsafe {
//             SCREEN_BUFFER.push(format!($($arg)*));
//         }
//     };
// }

// macro_rules! println_to_buffer {
//     ($($arg:tt)*) => {
//         print_to_buffer!($($arg)*);
//         print_to_buffer!("\n");
//     };
// }

pub fn print_link(
    uid: Uid,
    config: &PrintLinkConfig,
) -> PrintLinkResult {
    let f_i = match get_file_by_uid(uid) {
        Some(f) => f,
        None => {
            print_error_message(
                None,
                None,
                format!("get_file_by_uid({}) has failed", uid.debug_info()),
                config.min_width,
                config.max_width,
            );
            return PrintLinkResult::error();
        },
    };

    match get_path_by_uid(uid) {
        Some(path) => match fs::read_link(path) {
            Ok(dest) => {
                let dest = dest.display().to_string();
                let table_width = (dest.len() + COLUMN_MARGIN * 2).max(path.len() + 16 + COLUMN_MARGIN * 3).min(config.max_width).max(config.min_width);

                print_horizontal_line(
                    None,
                    table_width,
                    (true, false),
                    (true, true),
                );
                print_row(
                    colors::BLACK,
                    &vec![
                        path.clone(),
                        prettify_size(f_i.size),
                    ],
                    &vec![
                        table_width - 16 - COLUMN_MARGIN * 3,
                        16,
                    ],
                    &vec![
                        Alignment::Left,
                        Alignment::Right,
                    ],
                    &vec![
                        LineColor::All(colors::WHITE),
                        LineColor::All(colors::YELLOW),
                    ],
                    COLUMN_MARGIN,
                    (true, true),
                );
                print_row(
                    colors::BLACK,
                    &vec![
                        dest,
                    ],
                    &vec![
                        table_width - COLUMN_MARGIN * 2,
                    ],
                    &vec![
                        Alignment::Left,
                    ],
                    &vec![
                        LineColor::All(colors::WHITE),
                    ],
                    COLUMN_MARGIN,
                    (true, true),
                );
                print_horizontal_line(
                    None,
                    table_width,
                    (false, true),
                    (true, true),
                );

                PrintLinkResult::success()
            },
            Err(e) => {
                print_error_message(
                    Some(f_i),
                    Some(path.to_string()),
                    format!("{e:?}"),
                    config.min_width,
                    config.max_width,
                );
                PrintLinkResult::error()
            },
        },
        None => {
            print_error_message(
                Some(f_i),
                None,
                format!("get_path_by_uid({}) has failed", uid.debug_info()),
                config.min_width,
                config.max_width,
            );
            PrintLinkResult::error()
        },
    }
}
