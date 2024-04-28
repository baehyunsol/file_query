use hfile::*;
use regex::Regex;
use std::{fs, thread, time};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};

fn main() {
    unsafe { IS_MASTER_WORKING = true; }

    let is_interactive_mode = true;  // TODO: make it configurable

    let mut files = Box::new(HashMap::with_capacity(65536));
    let mut paths = Box::new(HashMap::with_capacity(65536));

    let mut print_dir_config = PrintDirConfig::default();
    let mut print_file_config = PrintFileConfig::default();
    let mut print_link_config = PrintLinkConfig::default();

    // TODO: it's inefficient to handle 3 (almost) identical configs
    print_dir_config.adjust_output_dimension();
    print_file_config.adjust_output_dimension();
    print_link_config.adjust_output_dimension();

    while print_dir_config.max_width < 40 {
        println!("Your terminal is too small to run FileQuery. Please resize your terminal and try again.");

        if !is_interactive_mode {
            return;
        }

        thread::sleep(time::Duration::from_millis(300));

        print_dir_config.adjust_output_dimension();
        print_file_config.adjust_output_dimension();
        print_link_config.adjust_output_dimension();
        clearscreen::clear().unwrap();
    }

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
                    // TODO: better parsing... or Rusty Line!
                    let mut buffer = String::new();
                    io::stdin().read_line(&mut buffer).unwrap();
                    print_dir_config.reset_alert();

                    buffer = buffer.strip_suffix("\n").unwrap().to_string();

                    let mut paths = buffer.split('/').map(|p| p.to_string()).collect::<Vec<_>>();

                    // `../../Music/` -> `../../Music`
                    // TODO: what if `Music` is a file, not a directory?
                    // TODO: it doesn't work if the path starts with `/`
                    if paths.last() == Some(&String::new()) {
                        paths.pop().unwrap();
                    }

                    let chars = buffer.chars().collect::<Vec<char>>();

                    match chars.get(0) {
                        Some('~') => {
                            curr_uid = Uid::BASE;
                            curr_instance = get_file_by_uid(curr_uid).unwrap();
                        },
                        // TODO: duplicate code
                        Some(';') => match chars.get(1) {  // special commands
                            Some('j') => match chars.get(2) {
                                Some('j') => match chars.get(3) {
                                    Some('j') => {
                                        print_dir_config.offset += 100;
                                    },
                                    _ => {
                                        print_dir_config.offset += 10;
                                    },
                                },
                                Some(c) if '0' <= *c && *c <= '9' => {
                                    let n = parse_int_from(&chars[2..]);
                                    print_dir_config.offset += n as usize;
                                },
                                _ => {
                                    print_dir_config.offset += 1;
                                },
                            },
                            // TODO
                            Some('k') => match chars.get(2) {
                                Some('k') => {},
                                Some(c) if '0' <= *c && *c <= '9' => {},
                                _ => {},
                            },
                            Some(c) if '0' <= *c && *c <= '9' => {
                                let n = parse_int_from(&chars[1..]);
                                print_dir_config.offset = n as usize;
                            },
                            // TODO: GOTO nth file, not just moving the offset
                            _ => {},
                        },
                        _ => if let Some(uid) = iterate_paths(curr_uid, &paths) {
                            curr_uid = uid;
                            curr_instance = get_file_by_uid(curr_uid).unwrap();
                            print_dir_config.offset = 0;
                        }

                        else if let Some(uid) = search_by_prefix(curr_uid, &paths) {
                            curr_uid = uid;
                            curr_instance = get_file_by_uid(curr_uid).unwrap();
                            print_dir_config.offset = 0;
                        }

                        else {
                            print_dir_config.alert = format!("{buffer:?} file not found");
                        },
                    }
                },
                // TODO: what does it do in Symlink mode?
                FileType::Symlink
                | FileType::File => {
                    // TODO: better parsing...
                    let mut buffer = String::new();
                    io::stdin().read_line(&mut buffer).unwrap();
                    print_file_config.reset_alert();
                    print_link_config.reset_alert();

                    let jump_by = match previous_print_file_result.viewer_kind {
                        // a line is a line (for texts and images)
                        ViewerKind::Text
                        | ViewerKind::Image => 1,

                        // a line is multiple bytes
                        ViewerKind::Hex => previous_print_file_result.width,
                    };

                    let mut has_changed_path = false;
                    let chars = buffer.strip_suffix("\n").unwrap().to_string().chars().collect::<Vec<char>>();

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
                            Some(c) if '0' <= *c && *c <= '9' => {
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
                            Some(c) if '0' <= *c && *c <= '9' => {
                                let n = parse_int_from(&chars[1..]) as usize;
                                print_file_config.offset = print_file_config.offset.max(n * jump_by) - n * jump_by;
                            },
                            _ => {  // k
                                print_file_config.offset = print_file_config.offset.max(jump_by) - jump_by;
                            },
                        },
                        Some('n') => match chars.get(1) {
                            Some('o') => match chars.get(2) {
                                Some('h') => {
                                    print_file_config.highlights = vec![];
                                },
                                _ => {},
                            },
                            _ => {
                                if print_file_config.highlights.len() > 0 {
                                    let new_highlight_index = match print_file_config.highlights.binary_search(&print_file_config.offset) {
                                        Ok(n) => (n + 1) % print_file_config.highlights.len(),
                                        Err(n) => n % print_file_config.highlights.len(),
                                    };
    
                                    print_file_config.offset = print_file_config.highlights[new_highlight_index];
                                    print_file_config.alert = format!("search result {}/{}", new_highlight_index + 1, print_file_config.highlights.len());
                                }
                            },
                        },
                        Some('N') if print_file_config.highlights.len() > 0 => {
                            let new_highlight_index = match print_file_config.highlights.binary_search(&print_file_config.offset) {
                                Ok(n) => (n + print_file_config.highlights.len() - 1) % print_file_config.highlights.len(),
                                Err(n) => (n + print_file_config.highlights.len() - 1) % print_file_config.highlights.len(),
                            };

                            print_file_config.offset = print_file_config.highlights[new_highlight_index];
                            print_file_config.alert = format!("search result {}/{}", new_highlight_index + 1, print_file_config.highlights.len());
                        },
                        Some('G') => {
                            match previous_print_file_result.viewer_kind {
                                ViewerKind::Text
                                | ViewerKind::Image => {
                                    print_file_config.offset = previous_print_file_result.last_line.unwrap_or(1).max(1) - 1;
                                },
                                ViewerKind::Hex => {
                                    print_file_config.offset = (curr_instance.size as usize).max(1) - 1;
                                },
                            }
                        },
                        Some('g') => match chars.get(1) {
                            Some('g') => {
                                print_file_config.offset = 0;
                            },
                            _ => {},
                        },
                        Some('0') => match chars.get(1) {
                            Some('x') | Some('X') if chars.len() > 2 => {
                                let n = parse_hex_from(&chars[2..]);
                                print_file_config.offset = n as usize;
                            },
                            _ => {
                                let n = parse_int_from(&chars[0..]);
                                print_file_config.offset = n as usize;
                            },
                        },
                        Some(c) if '1' <= *c && *c <= '9' => {
                            let n = parse_int_from(&chars[0..]);
                            print_file_config.offset = n as usize;
                        },
                        Some('q') => {
                            has_changed_path = true;
                            curr_uid = curr_instance.get_parent_uid();
                            curr_instance = get_file_by_uid(curr_uid).unwrap();
                        },
                        // TODO: search feature in hex viewer
                        Some('/') => {  // TODO: it's very naive implementation
                            let mut matched_lines = vec![];
                            let mut search_error = true;

                            if chars.len() > 2 {
                                // [1..] excludes '/'
                                if let Ok(re) = Regex::new(&chars[1..].iter().collect::<String>()) {
                                    if let Some(path) = get_path_by_uid(curr_uid) {
                                        if let Ok(file) = fs::File::open(path) {
                                            let line_reader = BufReader::new(file);
                                            search_error = false;

                                            for (index, line) in line_reader.lines().enumerate() {
                                                if let Ok(line) = &line {
                                                    if re.is_match(line) {
                                                        matched_lines.push(index);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if search_error {
                                print_file_config.alert = String::from("search failed");
                            }

                            else {
                                print_file_config.alert = format!("found {} results", matched_lines.len());
                            }

                            print_file_config.highlights = matched_lines;
                        },
                        Some('.') => match chars.get(1) {
                            Some('.') => {  // for convenience, `..` is an alias for `q`
                                print_file_config.offset = 0;

                                for ch in chars[1..].iter() {
                                    if *ch == '.' && curr_uid != Uid::ROOT {
                                        has_changed_path = true;
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

                    if has_changed_path {
                        print_file_config.offset = 0;
                        print_file_config.highlights = vec![];
                        print_file_config.read_mode = FileReadMode::default();
                    }

                    else {
                        if let Some(line_no) = previous_print_file_result.last_line {
                            if print_file_config.offset >= line_no {
                                print_file_config.offset = line_no.max(1) - 1;
                            }
                        }
                    }
                },
            }

            print_dir_config.adjust_output_dimension();
            print_file_config.adjust_output_dimension();
            print_link_config.adjust_output_dimension();

            while print_dir_config.max_width < 40 {
                println!("Your terminal is too small to run FileQuery. Please resize your terminal and try again.");
                thread::sleep(time::Duration::from_millis(300));

                print_dir_config.adjust_output_dimension();
                print_file_config.adjust_output_dimension();
                print_link_config.adjust_output_dimension();
                clearscreen::clear().unwrap();
            }

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

// TODO: these should not belong to `main.rs`
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

fn parse_hex_from(chars: &[char]) -> u64 {
    let mut result = 0;

    for c in chars {
        let n = if '0' <= *c && *c <= '9' {
            *c as u8 - b'0'
        } else if 'A' <= *c && *c <= 'Z' {
            *c as u8 + 10 - b'A'
        } else if 'a' <= *c && *c <= 'z' {
            *c as u8 + 10 - b'a'
        } else {
            return result;
        };

        result <<= 4;
        result += n as u64;

        // let's leave before it overflows
        if result > 0xffff_ffff_ffff {
            return result;
        }
    }

    result
}
