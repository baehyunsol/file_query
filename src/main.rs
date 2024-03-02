use hfile::*;
use std::collections::HashMap;
use std::io;

fn main() {
    unsafe { IS_MASTER_WORKING = true; }

    let is_interactive_mode = true;  // TODO: make it configurable

    let mut files = Box::new(HashMap::with_capacity(65536));
    let mut paths = Box::new(HashMap::with_capacity(65536));

    unsafe {
        FILES = files.as_mut() as *mut HashMap<_, _>;
        PATHS = paths.as_mut() as *mut HashMap<_, _>;
    }

    match std::env::current_dir() {
        Ok(dir) => {
            File::new_from_path_buf(dir, Some(Uid::BASE), None);
        },
        Err(e) => {
            println!("{e:?}");
            return;
        },
    }

    if is_interactive_mode {
        // clearscreen::clear().unwrap();
    }

    let mut curr_dir_uid = Uid::BASE;
    print_dir(curr_dir_uid, PrintDirConfig::default());

    unsafe { IS_MASTER_WORKING = false; }

    // TODO: spawn_workers here

    // TODO: use rustyline or reedline
    if is_interactive_mode {
        loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();

            if buffer == "..\n" {
                let curr_dir = get_file_by_uid(curr_dir_uid).unwrap();
                curr_dir_uid = curr_dir.get_parent_uid();
            }

            // clearscreen::clear().unwrap();

            unsafe { IS_MASTER_WORKING = true; }
            print_dir(curr_dir_uid, PrintDirConfig::default());
            unsafe { IS_MASTER_WORKING = false; }
        }
    }
}
