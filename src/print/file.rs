use super::{
    calc_table_column_widths,
    print_error_message,
    print_horizontal_line,
    print_row,
    Alignment,
    COLUMN_MARGIN,
    LineColor,
    SCREEN_BUFFER,
};
use super::config::PrintFileConfig;
use super::result::PrintFileResult;
use super::utils::{
    convert_ocean_dark_color,
    format_duration,
    prettify_size,
    try_extract_utf8_text,
    try_read_image,
};
use crate::colors;
use crate::uid::Uid;
use crate::utils::{
    get_path_by_uid,
    get_file_by_uid,
};
use lazy_static::lazy_static;
use std::fs;
use std::io::Read;
use std::time::Instant;
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::util::LinesWithEndings;

#[cfg(unix)]
use std::os::unix::fs::FileExt;

#[cfg(not(unix))]
use std::os::windows::fs::FileExt;

macro_rules! print_to_buffer {
    ($($arg:tt)*) => {
        unsafe {
            SCREEN_BUFFER.push(format!($($arg)*));
        }
    };
}

macro_rules! println_to_buffer {
    ($($arg:tt)*) => {
        print_to_buffer!($($arg)*);
        print_to_buffer!("\n");
    };
}

lazy_static! {
    static ref SYNTECT_SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref SYNTECT_THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

pub fn print_file(
    uid: Uid,
    config: &PrintFileConfig,
) -> PrintFileResult {
    let started_at = Instant::now();

    match get_path_by_uid(uid) {
        Some(path) => {
            let f_i = get_file_by_uid(uid).unwrap();
            let mut content = vec![];
            let mut truncated = 0;

            match fs::File::open(&path) {
                Ok(mut f) => if f_i.size <= (1 << 18) {
                    if let Err(e) = f.read_to_end(&mut content) {
                        print_error_message(
                            Some(f_i),
                            Some(path.to_string()),
                            format!("{e:?}"),
                            config.min_width,
                            config.max_width,
                        );
                        return PrintFileResult::error();
                    }
                } else {
                    let mut buffer = [0u8; (1 << 18)];

                    if let Err(e) = f.read_exact(&mut buffer) {
                        print_error_message(
                            Some(f_i),
                            Some(path.to_string()),
                            format!("{e:?}"),
                            config.min_width,
                            config.max_width,
                        );
                        return PrintFileResult::error();
                    }

                    content = buffer.to_vec();
                    truncated = f_i.size - content.len() as u64;
                },
                Err(e) => {
                    print_error_message(
                        Some(f_i),
                        Some(path.to_string()),
                        format!("{e:?}"),
                        config.min_width,
                        config.max_width,
                    );
                    return PrintFileResult::error();
                },
            }

            let mut highlights = &config.highlights[..];

            if let Some(text) = try_extract_utf8_text(&content) {
                let mut lines = vec![
                    vec![
                        String::from("line"),
                        String::new(),  // border
                        String::from("content"),
                    ],
                ];
                let mut alignments = vec![
                    vec![Alignment::Center; 3],
                ];

                let mut colors = vec![
                    vec![LineColor::All(colors::WHITE); 3],
                ];

                let syntax = if let Some(ext) = &f_i.file_ext {
                    SYNTECT_SYNTAX_SET.find_syntax_by_extension(ext).unwrap_or_else(|| SYNTECT_SYNTAX_SET.find_syntax_plain_text())
                } else {
                    SYNTECT_SYNTAX_SET.find_syntax_plain_text()
                };
                let mut h = HighlightLines::new(syntax, &SYNTECT_THEME_SET.themes["base16-ocean.dark"]);
                let mut curr_line_chars = vec![];
                let mut curr_line_colors = vec![];
                let mut line_no = 1;
                let mut ch_count = 0;

                'top_loop: for line in LinesWithEndings::from(&text) {
                    let parts = h.highlight_line(line, &SYNTECT_SYNTAX_SET).unwrap();

                    for (style, content) in parts.iter() {
                        for ch in content.chars() {
                            ch_count += 1;

                            if ch == '\n' {
                                if line_no > config.offset {
                                    let (line_no_fmt, line_no_colors) = if highlights.get(0) == Some(&line_no) {
                                        let line_no_fmt = format!(">>> {line_no}");
                                        let line_no_colors = LineColor::Each(vec![
                                            vec![colors::RED; 3],
                                            vec![colors::WHITE; line_no_fmt.len() - 3],
                                        ].concat());

                                        highlights = &highlights[1..];

                                        (line_no_fmt, line_no_colors)
                                    } else {
                                        (line_no.to_string(), LineColor::All(colors::WHITE))
                                    };

                                    lines.push(vec![
                                        line_no_fmt,
                                        String::from("│"),
                                        curr_line_chars.iter().collect::<String>(),
                                    ]);
                                    alignments.push(vec![
                                        Alignment::Right,  // line no
                                        Alignment::Left,   // border
                                        Alignment::Left,   // content
                                    ]);
                                    colors.push(vec![
                                        line_no_colors,
                                        LineColor::All(colors::WHITE),  // border
                                        LineColor::Each(curr_line_colors),
                                    ]);
                                }

                                curr_line_chars = vec![];
                                curr_line_colors = vec![];
                                line_no += 1;

                                if line_no == config.max_row + config.offset {
                                    truncated = f_i.size - ch_count;
                                    break 'top_loop;
                                }
                            }

                            else {
                                // tmp hack: it cannot render '\r' characters properly
                                curr_line_chars.push(if ch == '\r' { ' ' } else { ch });
                                curr_line_colors.push(convert_ocean_dark_color(style.foreground));
                            }
                        }
                    }

                    if !curr_line_chars.is_empty() {
                        lines.push(vec![
                            format!("{line_no}"),
                            String::from("│"),
                            curr_line_chars.iter().collect::<String>(),
                        ]);
                        alignments.push(vec![
                            Alignment::Right,  // line no
                            Alignment::Left,   // border
                            Alignment::Left,   // content
                        ]);
                        colors.push(vec![
                            LineColor::All(colors::WHITE),
                            LineColor::All(colors::WHITE),  // border
                            LineColor::Each(curr_line_colors.clone()),
                        ]);
                    }
                }

                if truncated > 0 {
                    lines.push(vec![format!("... (truncated {})", prettify_size(truncated).trim())]);
                    alignments.push(vec![Alignment::Left]);
                    colors.push(vec![LineColor::All(colors::WHITE)]);
                }

                let table_column_widths = calc_table_column_widths(
                    &lines,
                    Some(config.max_width),
                    Some(config.min_width),
                    COLUMN_MARGIN,
                );
                let curr_table_width = {
                    let (cols, widths) = table_column_widths.iter().next().unwrap();

                    widths.iter().sum::<usize>() + COLUMN_MARGIN * (*cols + 1)
                };

                print_horizontal_line(
                    None,
                    curr_table_width,
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
                        curr_table_width - 16 - COLUMN_MARGIN * 3,
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

                print_horizontal_line(
                    None,
                    curr_table_width,
                    (false, false),
                    (true, true),
                );

                for (index, line) in lines.iter().enumerate() {
                    let column_widths = table_column_widths.get(&line.len()).unwrap();

                    print_row(
                        colors::BLACK,
                        &line,
                        column_widths,
                        &alignments[index],
                        &colors[index],
                        COLUMN_MARGIN,
                        (true, true),
                    );
                }

                print_horizontal_line(
                    None,
                    curr_table_width,
                    (false, true),
                    (true, true),
                );

                println_to_buffer!("took {}", format_duration(Instant::now().duration_since(started_at)));

                PrintFileResult::text_success(0, None)  // TODO
            }

            else if let Some(img) = try_read_image(f_i) {
                todo!()
            }

            // hex viewer
            else {
                // I want the offset to be multiple of 8
                let mut offset = (config.offset - (config.offset & 7)) as u64;

                // I want the offset to be less than f_i.size - 32
                offset = (offset + 32).min(f_i.size).max(32) - 32;

                // There's no point in reading more than 16KiB
                let mut buffer = [0; 16384];

                let read_result = match fs::File::open(&path) {
                    Ok(f) => {
                        #[cfg(unix)]
                        let r = f.read_at(&mut buffer, offset);

                        #[cfg(not(unix))]
                        let r = f.seek_read(&mut buffer, offset);

                        r
                    },
                    Err(e) => {
                        print_error_message(
                            Some(f_i),
                            Some(path.to_string()),
                            format!("{e:?}"),
                            config.min_width,
                            config.max_width,
                        );
                        return PrintFileResult::error();
                    },
                };

                let mut truncated_bytes = 0;

                let bytes_read = match read_result {
                    Ok(n) => n,
                    Err(e) => {
                        print_error_message(
                            Some(f_i),
                            Some(path.to_string()),
                            format!("{e:?}"),
                            config.min_width,
                            config.max_width,
                        );
                        return PrintFileResult::error();
                    },
                };

                let buffer = buffer[..bytes_read].to_vec();

                let (
                    bytes_per_row,
                    total_width,
                    col1_width,
                    col2_width,
                    col3_width,
                ) = calc_hex_viewer_row_width(
                    config.min_width,
                    config.max_width,
                );

                let column_widths = vec![
                    col1_width,
                    col2_width,
                    col3_width,
                ];

                print_horizontal_line(
                    None,
                    total_width,
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
                        total_width - 16 - COLUMN_MARGIN * 3,
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

                print_horizontal_line(
                    None,
                    total_width,
                    (false, false),
                    (true, true),
                );

                print_row(
                    colors::BLACK,
                    &vec![
                        "offset".to_string(),
                        "hex".to_string(),
                        "ascii".to_string(),
                    ],
                    &vec![
                        col1_width,
                        col2_width,
                        col3_width,
                    ],
                    &vec![Alignment::Center; 3],
                    &vec![LineColor::All(colors::WHITE); 3],
                    COLUMN_MARGIN,
                    (true, true),
                );

                for (line_no, bytes) in buffer.chunks(bytes_per_row).enumerate() {
                    let mut offset_fmt = format!("{offset:08x}");
                    let mut offset_color = if offset & 255 == 0 {
                        LineColor::All(colors::GREEN)
                    } else {
                        LineColor::All(colors::WHITE)
                    };

                    if let Some(highlight_offset) = highlights.get(0) {
                        let highlight_offset = *highlight_offset as u64;

                        if offset <= highlight_offset && highlight_offset < offset + bytes_per_row as u64 {
                            offset_fmt = String::from(">>>>>>>>");
                            offset_color = LineColor::All(colors::RED);
                        }

                        while let Some(highlight_offset) = highlights.get(0) {
                            let highlight_offset = *highlight_offset as u64;

                            if offset <= highlight_offset && highlight_offset < offset + bytes_per_row as u64 {
                                highlights = &highlights[1..];
                            }

                            else {
                                break;
                            }
                        }
                    }

                    let mut bytes_fmt = vec![];
                    let mut bytes_colors = vec![];
                    let mut ascii_fmt = vec![];
                    let mut ascii_colors = vec![];

                    for (index, byte) in bytes.iter().enumerate() {
                        bytes_fmt.push(format!("{byte:02x}"));

                        if *byte == 0 {
                            bytes_colors.push(colors::GRAY);
                            bytes_colors.push(colors::GRAY);
                        }

                        else {
                            bytes_colors.push(colors::YELLOW);
                            bytes_colors.push(colors::YELLOW);
                        }

                        if b' ' <= *byte && *byte <= b'~' {
                            ascii_fmt.push((*byte as char).to_string());
                            ascii_colors.push(colors::YELLOW);
                        }

                        else {
                            ascii_fmt.push(".".to_string());
                            ascii_colors.push(colors::GRAY);
                        }

                        if index == bytes.len() - 1 {
                            // nop
                        }

                        else if index & 7 == 7 {
                            bytes_fmt.push("  ".to_string());
                            bytes_colors.push(colors::WHITE);
                            bytes_colors.push(colors::WHITE);

                            ascii_fmt.push("  ".to_string());
                            ascii_colors.push(colors::WHITE);
                            ascii_colors.push(colors::WHITE);
                        }

                        else {
                            bytes_fmt.push(" ".to_string());
                            bytes_colors.push(colors::WHITE);
                        }
                    }

                    let bytes_fmt = bytes_fmt.concat();
                    let ascii_fmt = ascii_fmt.concat();

                    print_row(
                        colors::BLACK,
                        &vec![
                            offset_fmt,
                            bytes_fmt,
                            ascii_fmt,
                        ],
                        &column_widths,
                        &vec![Alignment::Right, Alignment::Left, Alignment::Left],
                        &vec![
                            offset_color,
                            LineColor::Each(bytes_colors),
                            LineColor::Each(ascii_colors),
                        ],
                        COLUMN_MARGIN,
                        (true, true),
                    );

                    offset += bytes_per_row as u64;

                    if line_no == config.max_row {
                        // there's no need to add bytes_per_row, it's already added!
                        truncated_bytes = f_i.size.max(offset) - offset;
                        break;
                    }
                }


                if truncated_bytes > 0 {
                    print_row(
                        colors::BLACK,
                        &vec![format!("... (truncated {})", prettify_size(truncated_bytes).trim())],
                        &vec![total_width - COLUMN_MARGIN * 2],
                        &vec![Alignment::Left],
                        &vec![LineColor::All(colors::WHITE)],
                        COLUMN_MARGIN,
                        (true, true),
                    );
                }

                print_horizontal_line(
                    None,
                    total_width,
                    (false, true),
                    (true, true),
                );
                println_to_buffer!("took {}", format_duration(Instant::now().duration_since(started_at)));

                PrintFileResult::hex_success(bytes_per_row)
            }
        },
        None => {
            print_error_message(
                None,
                None,
                format!("get_path_by_uid({}) has failed", uid.debug_info()),
                config.min_width,
                config.max_width,
            );

            PrintFileResult::error()
        },
    }
}

// '  00000000  7f 45 4c 46  .ELF  '
const HEX_VIEWER_4_BYTES: usize = 23 + 4 * COLUMN_MARGIN;

// '  00000000  7f 45 4c 46 02 01 01 00  .ELF....  '
const HEX_VIEWER_8_BYTES: usize = 39 + 4 * COLUMN_MARGIN;

// '  00000000  7f 45 4c 46 02 01 01 00  00 00 00 00 00 00 00 00  .ELF....  ........  '
const HEX_VIEWER_16_BYTES: usize = 74 + 4 * COLUMN_MARGIN;

// '  00000000  7f 45 4c 46 02 01 01 00  00 00 00 00 00 00 00 00  03 00 3e 00 01 00 00 00  a0 a1 03 00 00 00 00 00  .ELF....  ........  ..>.....  ........  '
const HEX_VIEWER_32_BYTES: usize = 144 + 4 * COLUMN_MARGIN;

fn calc_hex_viewer_row_width(
    min_width: usize,
    max_width: usize,
) -> (
    usize,  // bytes per row
    usize,  // total width
    usize,  // col1 width
    usize,  // col2 width
    usize,  // col3 width
) {
    if max_width < HEX_VIEWER_8_BYTES {
        (4, HEX_VIEWER_4_BYTES, 8, 11, 4)
    }

    else if max_width < HEX_VIEWER_16_BYTES {
        (8, HEX_VIEWER_8_BYTES, 8, 23, 8)
    }

    else if max_width < HEX_VIEWER_32_BYTES {
        (16, HEX_VIEWER_16_BYTES, 8, 48, 18)
    }

    else {
        (32, HEX_VIEWER_32_BYTES, 8, 98, 38)
    }
}
