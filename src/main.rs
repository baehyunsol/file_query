use hfile::*;
use std::collections::HashMap;
use std::io;

fn main() {
    unsafe { IS_MASTER_WORKING = true; }

    let is_interactive_mode = true;  // TODO: make it configurable

    let mut files = Box::new(HashMap::with_capacity(65536));
    let mut paths = Box::new(HashMap::with_capacity(65536));

    let mut print_dir_config = PrintDirConfig::default();
    let mut print_file_config = PrintFileConfig::default();
    let mut print_link_config = PrintLinkConfig::default();

    print_dir_config.adjust_output_dimension();
    print_file_config.adjust_output_dimension();
    print_link_config.adjust_output_dimension();

    unsafe {
        FILES = files.as_mut() as *mut HashMap<_, _>;
        PATHS = paths.as_mut() as *mut HashMap<_, _>;
    }

    match std::env::current_dir() {
        Ok(dir) => {
            File::new_from_path_buf(dir, Some(Uid::BASE), None);
        },
        Err(e) => {
            print_error_message(
                None,
                None,
                format!("{e:?}"),
                print_dir_config.min_width,
                print_dir_config.max_width,
            );
            return;
        },
    }

    let mut curr_uid = Uid::BASE;
    let mut curr_instance = get_file_by_uid(curr_uid).unwrap();
    let mut curr_mode = FileType::Dir;

    let mut previous_print_dir_result = PrintDirResult::dummy();
    let mut previous_print_file_result = PrintFileResult::dummy();
    let mut previous_print_link_result = PrintLinkResult::dummy();

    // Uid::BASE must point to a directory
    print_dir(curr_uid, &print_dir_config);
    flip_buffer(is_interactive_mode);

    unsafe { IS_MASTER_WORKING = false; }

    // TODO: spawn_workers here

    // TODO: use rustyline or reedline
    if is_interactive_mode {
        loop {
            match curr_mode {
                FileType::Dir => {
                    let mut buffer = String::new();
                    io::stdin().read_line(&mut buffer).unwrap();
                    buffer = buffer.strip_suffix("\n").unwrap().to_string();

                    if buffer.starts_with("..") {
                        for c in buffer.get(1..).unwrap().chars() {
                            if c == '.' && curr_uid != Uid::ROOT {
                                curr_uid = curr_instance.get_parent_uid();
                                curr_instance = get_file_by_uid(curr_uid).unwrap();
                            }

                            else {
                                break;
                            }
                        }
                    }

                    else {
                        for child in get_file_by_uid(curr_uid).unwrap().get_children(true) {
                            if child.name == buffer {
                                curr_uid = child.uid;
                                curr_instance = get_file_by_uid(curr_uid).unwrap();
                            }
                        }
                    }
                },
                // TODO: what does it do in Symlink mode?
                FileType::Symlink
                | FileType::File => {
                    // TODO: better parsing...
                    let mut buffer = String::new();
                    io::stdin().read_line(&mut buffer).unwrap();

                    let jump_by = match previous_print_file_result.viewer_kind {
                        // a line is a line (for texts and images)
                        ViewerKind::Text
                        | ViewerKind::Image => 1,

                        // a line is multiple bytes
                        ViewerKind::Hex => previous_print_file_result.width,
                    };

                    let chars = buffer.chars().collect::<Vec<char>>();

                    match chars.get(0) {
                        Some('j') => match chars.get(1) {
                            Some('j') => match chars.get(2) {
                                Some('j') => {  // jjj
                                    print_file_config.offset += 100 * jump_by;
                                },
                                _ => {  // jj
                                    print_file_config.offset += 10 * jump_by;
                                },
                            },
                            Some('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9') => {
                                let n = parse_int_from(&chars[1..]) as usize;
                                print_file_config.offset += n * jump_by;
                            },
                            _ => {  // j
                                print_file_config.offset += jump_by;
                            },
                        },
                        Some('k') => match chars.get(1) {
                            Some('k') => match chars.get(2) {
                                Some('k') => {  // kkk
                                    print_file_config.offset = print_file_config.offset.max(100 * jump_by) - 100 * jump_by;
                                },
                                _ => {  // kk
                                    print_file_config.offset = print_file_config.offset.max(10 * jump_by) - 10 * jump_by;
                                },
                            },
                            Some('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9') => {
                                let n = parse_int_from(&chars[1..]) as usize;
                                print_file_config.offset = print_file_config.offset.max(n * jump_by) - n * jump_by;
                            },
                            _ => {  // k
                                print_file_config.offset = print_file_config.offset.max(jump_by) - jump_by;
                            },
                        },
                        Some('n') => {
                            // next highlighted line
                        },
                        Some('N') => {
                            // previous highlighted line
                        },
                        Some('G') => {
                            match previous_print_file_result.viewer_kind {
                                ViewerKind::Hex => {
                                    print_file_config.offset = curr_instance.size as usize;
                                },
                                ViewerKind::Image => {
                                    print_file_config.offset = previous_print_file_result.last_line.unwrap_or(8).max(8) - 8;
                                },
                                _ => { /* TODO */ },
                            }
                        },
                        Some('g') => match chars.get(1) {
                            Some('g') => {
                                print_file_config.offset = 0;
                            },
                            _ => {},
                        },
                        Some('0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9') => {
                            let n = parse_int_from(&chars[0..]);
                            print_file_config.offset = n as usize;
                        },
                        Some('q') => {
                            print_file_config.offset = 0;
                            curr_uid = curr_instance.get_parent_uid();
                            curr_instance = get_file_by_uid(curr_uid).unwrap();
                        },
                        Some('.') => match chars.get(1) {
                            Some('.') => {  // for convenience, `..` is an alias for `q`
                                print_file_config.offset = 0;

                                for ch in chars[1..].iter() {
                                    if *ch == '.' && curr_uid != Uid::ROOT {
                                        curr_uid = curr_instance.get_parent_uid();
                                        curr_instance = get_file_by_uid(curr_uid).unwrap();
                                    }

                                    else {
                                        break;
                                    }
                                }
                            },
                            _ => {},
                        },
                        _ => {},
                    }

                    if let Some(line_no) = previous_print_file_result.last_line {
                        print_file_config.offset = print_file_config.offset.min(line_no);
                    }
                },
            }

            print_dir_config.adjust_output_dimension();
            print_file_config.adjust_output_dimension();
            print_link_config.adjust_output_dimension();

            unsafe { IS_MASTER_WORKING = true; }

            match get_file_by_uid(curr_uid) {
                Some(f) => match f.file_type {
                    FileType::Dir => {
                        previous_print_dir_result = print_dir(curr_uid, &print_dir_config);
                        curr_mode = FileType::Dir;
                    },
                    FileType::File => {
                        previous_print_file_result = print_file(curr_uid, &print_file_config);
                        curr_mode = FileType::File;
                    },
                    FileType::Symlink => {
                        previous_print_link_result = print_link(curr_uid, &print_link_config);
                        curr_mode = FileType::Symlink;
                    },
                },
                None => {
                    print_error_message(
                        Some(curr_instance),
                        None,
                        format!("get_file_by_uid({}) has failed", curr_uid.debug_info()),
                        print_dir_config.min_width,
                        print_dir_config.max_width,
                    );
                },
            }

            flip_buffer(is_interactive_mode);
            unsafe { IS_MASTER_WORKING = false; }
        }
    }
}

// TODO: it should not belong to `main.rs`
fn parse_int_from(chars: &[char]) -> u64 {
    let mut result = 0;

    for c in chars {
        if *c < '0' || *c > '9' {
            return result;
        }

        result *= 10;
        result += (*c as u8 - b'0') as u64;

        // let's leave before it overflows
        if result > 0xffff_ffff_ffff {
            return result;
        }
    }

    result
}
