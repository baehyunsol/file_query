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
        clearscreen::clear().unwrap();
    }

    let mut print_dir_config = PrintDirConfig::default();
    let mut print_file_config = PrintFileConfig::default();

    print_dir_config.adjust_output_dimension();
    print_file_config.adjust_output_dimension();

    let mut curr_uid = Uid::BASE;

    // Uid::BASE must point to a directory
    print_dir(curr_uid, &print_dir_config);

    unsafe { IS_MASTER_WORKING = false; }

    // TODO: spawn_workers here

    // TODO: use rustyline or reedline
    if is_interactive_mode {
        loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            buffer = buffer.strip_suffix("\n").unwrap().to_string();

            if buffer == ".." && curr_uid != Uid::ROOT {
                let curr_dir = get_file_by_uid(curr_uid).unwrap();
                curr_uid = curr_dir.get_parent_uid();
            }

            else {
                for child in get_file_by_uid(curr_uid).unwrap().get_children(true) {
                    if child.name == buffer {
                        curr_uid = child.uid;
                    }
                }
            }

            print_dir_config.adjust_output_dimension();
            print_file_config.adjust_output_dimension();

            clearscreen::clear().unwrap();

            unsafe { IS_MASTER_WORKING = true; }

            match get_file_by_uid(curr_uid) {
                Some(f) => match f.file_type {
                    FileType::Dir => {
                        print_dir(curr_uid, &print_dir_config);
                    },
                    FileType::File => {
                        print_file(curr_uid, &print_file_config);
                    },
                    FileType::Symlink => {
                        print_link(curr_uid);
                    },
                },
                None => {
                    // TODO: what do I do here?
                },
            }
            unsafe { IS_MASTER_WORKING = false; }
        }
    }
}
