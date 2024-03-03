use super::print_error_message;
use super::config::PrintLinkConfig;
use super::result::PrintLinkResult;
use crate::uid::Uid;
use crate::utils::get_path_by_uid;

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
    match get_path_by_uid(uid) {
        Some(path) => PrintLinkResult::success(),
        None => {
            print_error_message(
                None,
                None,
                format!("get_path_by_uid({}) has failed", uid.debug_info()),
                config.min_width,
                config.max_width,
            );
            PrintLinkResult::error()
        },
    }
}
