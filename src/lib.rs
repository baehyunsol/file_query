use std::collections::HashMap;

mod colors;
mod file;
mod print;
mod uid;
mod utils;

pub use file::File;
pub use print::{print_dir, PrintDirConfig};
pub use uid::Uid;
pub use utils::get_file_by_uid;

pub static mut IS_MASTER_WORKING: bool = false;
pub static mut FILES: *mut HashMap<Uid, File> = std::ptr::null_mut();
pub static mut PATHS: *mut HashMap<Uid, Path> = std::ptr::null_mut();

type Path = String;
