use colored::{Color, Colorize};
use crate::colors;
use crate::file::File;
use crate::uid::Uid;
use crate::utils::{
    get_path_by_uid,
    get_file_by_uid,
    sort_files,
};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::time::{Instant, SystemTime};
use syntect::easy::HighlightLines;
use syntect::parsing::SyntaxSet;
use syntect::highlighting::ThemeSet;
use syntect::util::LinesWithEndings;

#[cfg(unix)]
use std::os::unix::fs::FileExt;

#[cfg(not(unix))]
use std::os::windows::fs::FileExt;

mod config;
mod utils;

const COLUMN_MARGIN: usize = 2;

pub use config::{
    ColumnKind,
    PrintDirConfig,
    PrintFileConfig,
};
use utils::{
    colorize_name,
    colorize_size,
    colorize_time,
    colorize_type,
    format_duration,
    prettify_size,
    prettify_time,
    try_extract_utf8_text,
};

lazy_static! {
    static ref SYNTECT_SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
    static ref SYNTECT_THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

#[derive(Clone)]
enum Alignment {
    Left, Center, Right,
}

/// It does NOT check whether the given `uid` is dir or not.
/// It assumes that the given `uid` is valid.
pub fn print_dir(
    uid: Uid,
    config: &PrintDirConfig,
) {
    let started_at = Instant::now();
    let file = get_file_by_uid(uid).unwrap();

    file.init_children();

    let mut children_instances = file.get_children(config.show_hidden_files);

    // num of children BEFORE truncated
    let children_num = children_instances.len();
    let curr_dir_path = get_path_by_uid(uid).unwrap();

    sort_files(&mut children_instances, config.sort_by, config.sort_reverse);

    // it shows contents inside dirs (if there are enough rows)
    let mut nested_levels = vec![];

    if children_num > config.max_row {
        children_instances = children_instances[..config.max_row].to_vec();
        nested_levels = vec![0; config.max_row];
    }

    else if children_num + 4 < config.max_row {
        let (children_instances_, nested_levels_) = add_nested_contents(
            children_instances,
            &config,
        );
        children_instances = children_instances_;
        nested_levels = nested_levels_;
    }

    else {
        nested_levels = vec![0; children_num];
    }

    let now = SystemTime::now();

    let truncated_rows = children_num - nested_levels.iter().filter(|level| **level == 0).count();

    if truncated_rows > 0 {
        children_instances.push(
            // very ugly, but there's no other way than this to fool the borrow checker
            get_file_by_uid(File::message_for_truncated_rows(truncated_rows)).unwrap() as &File
        );
        nested_levels.push(0);
    }

    if children_num == 0 {
        children_instances.push(
            // very ugly, but there's no other way than this to fool the borrow checker
            get_file_by_uid(File::message_from_string(String::from("Empty Directory"))).unwrap() as &File
        );
        nested_levels.push(0);
    }

    debug_assert_eq!(
        children_instances.len(),
        nested_levels.len(),
    );

    let mut table_contents = vec![];
    let mut column_alignments = vec![];
    let mut content_colors = vec![];

    // column names
    table_contents.push(config.columns.iter().map(|col| col.header_string()).collect::<Vec<_>>());
    column_alignments.push(vec![Alignment::Center; table_contents[0].len()]);
    content_colors.push(vec![LineColor::All(colors::WHITE); table_contents[0].len()]);

    let mut table_index = 0;
    let mut table_sub_index = 0;

    for (index, child) in children_instances.iter().enumerate() {
        let nested_level = nested_levels[index];
        let has_to_use_half_arrow = nested_level > 0 && (index == nested_levels.len() - 1 || nested_levels[index + 1] < nested_level);

        if child.is_special_file() {
            let message = render_indented_message(
                nested_level,
                has_to_use_half_arrow,
                &child.name,
            );
            let col2_color = if nested_level > 0 {
                color_arrows(
                    colors::WHITE,  // default color
                    colors::GREEN,  // arrow color
                    &message,
                )
            } else {
                LineColor::All(colors::WHITE)
            };
            table_contents.push(vec![
                String::new(),  // index
                message,
            ]);
            column_alignments.push(vec![
                Alignment::Right,
                Alignment::Left,
            ]);
            content_colors.push(vec![
                LineColor::All(colors::WHITE),
                col2_color,
            ]);

            continue;
        }

        if nested_level == 0 {
            table_index += 1;
            table_sub_index = 0;
        }

        else if nested_level == 1 {
            table_sub_index += 1;
        }

        else {
            unreachable!();
        }

        let table_index_formatted = if table_sub_index == 0 {
            format!("{table_index}   ")
        } else {
            format!(
                "{table_index}-{table_sub_index}{}",
                if table_sub_index < 10 { " " } else { "" },
            )
        };

        let name = if nested_level > 0 {  // nested contents do not show full path
            render_indented_message(
                nested_level,
                has_to_use_half_arrow,
                &child.name,
            )
        } else if config.show_full_path {
            get_path_by_uid(child.uid).unwrap().to_string()
        } else {
            child.name.clone()
        };

        let mut curr_table_contents = vec![];
        let mut curr_column_alignments = vec![];
        let mut curr_content_colors = vec![];

        for column in config.columns.iter() {
            match column {
                ColumnKind::Index => {
                    curr_table_contents.push(table_index_formatted.clone());
                    curr_content_colors.push(LineColor::All(colors::WHITE));
                },
                ColumnKind::Name => {
                    curr_table_contents.push(name.clone());
                    let name_color = colorize_name(child.file_type, child.is_executable);

                    if nested_level > 0 {
                        curr_content_colors.push(color_arrows(
                            name_color,     // default color
                            colors::GREEN,  // arrow color
                            &name,
                        ));
                    }

                    else {
                        curr_content_colors.push(LineColor::All(name_color));
                    }
                },
                ColumnKind::Size => {
                    curr_table_contents.push(prettify_size(child.size));
                    curr_content_colors.push(LineColor::All(colorize_size(child.size)));
                },
                ColumnKind::TotalSize => {
                    curr_table_contents.push(prettify_size(child.get_recursive_size()));
                    curr_content_colors.push(LineColor::All(colorize_size(child.get_recursive_size())));
                },
                ColumnKind::Modified => {
                    curr_table_contents.push(prettify_time(&now, child.last_modified));
                    curr_content_colors.push(LineColor::All(colorize_time(&now, child.last_modified)));
                },
                ColumnKind::FileType => {
                    curr_table_contents.push(child.file_type.to_string());
                    curr_content_colors.push(LineColor::All(colorize_type(child.file_type)));
                },
                ColumnKind::FileExt => {
                    curr_table_contents.push(child.file_ext.clone().unwrap_or(String::new()));
                    curr_content_colors.push(LineColor::All(colors::WHITE));
                },
            }

            curr_column_alignments.push(column.alignment());
        }

        table_contents.push(curr_table_contents);
        column_alignments.push(curr_column_alignments);
        content_colors.push(curr_content_colors);
    }

    let table_column_widths = calc_table_column_widths(
        &table_contents,
        Some(config.max_width),
        Some(config.min_width),
        COLUMN_MARGIN,
    );
    let curr_table_width = {
        let (cols, widths) = table_column_widths.iter().next().unwrap();

        widths.iter().sum::<usize>() + COLUMN_MARGIN * (*cols + 1)
    };

    print_horizontal_line(
        None,  // background
        curr_table_width,
        (true, false),   // (is top, is bottom)
        (true, true),    // (left border, right border)
    );

    // print curr dir
    print_row(
        colors::BLACK,
        &vec![
            curr_dir_path.to_string(),
            format!("{} elements", children_num),
        ],
        &vec![
            curr_table_width - 13 - COLUMN_MARGIN * 3,
            13,
        ],
        &vec![
            Alignment::Left,    // path
            Alignment::Right,   // num of elements
        ],
        &vec![
            LineColor::All(colors::WHITE),  // path
            LineColor::All(colors::YELLOW),  // num of elements
        ],
        COLUMN_MARGIN,
        (true, true),
    );

    print_horizontal_line(
        None,  // background
        curr_table_width,
        (false, false),  // (is top, is bottom)
        (true, true),    // (left border, right border)
    );

    for index in 0..table_contents.len() {
        let background = if index & 1 == 1 { colors::DARK_GRAY } else { colors::BLACK };
        let column_widths = table_column_widths.get(&table_contents[index].len()).unwrap();

        print_row(
            background,
            &table_contents[index],
            column_widths,
            &column_alignments[index],
            &content_colors[index],
            COLUMN_MARGIN,
            (true, true),
        );
    }

    print_horizontal_line(
        None,  // background
        curr_table_width,
        (false, true),   // (is top, is bottom)
        (true, true),    // (left border, right border)
    );
    println!("{}", config.into_sql_string());
    println!("took {}", format_duration(Instant::now().duration_since(started_at)));
}

pub fn print_link(uid: Uid) {
    match get_path_by_uid(uid) {
        Some(path) => {},
        None => {
            // TODO: what do I do here?
        },
    }
}

pub fn print_file(
    uid: Uid,
    config: &PrintFileConfig,
) {
    let started_at = Instant::now();

    match get_path_by_uid(uid) {
        Some(path) => {
            let f_i = get_file_by_uid(uid).unwrap();
            let mut content = vec![];
            let mut truncated = 0;

            match fs::File::open(&path) {
                Ok(mut f) => if f_i.size <= (1 << 18) {
                    if let Err(e) = f.read_to_end(&mut content) {
                        println!("{e:?}");
                        return;
                    }
                } else {
                    let mut buffer = [0u8; (1 << 18)];

                    if let Err(e) = f.read_exact(&mut buffer) {
                        println!("{e:?}");
                        return;
                    }

                    content = buffer.to_vec();
                    truncated = f_i.size - content.len() as u64;
                },
                Err(e) => {
                    println!("{e:?}");
                    return;
                },
            }

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
                                if line_no >= config.offset {
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
                                curr_line_colors.push(Color::TrueColor {
                                    r: style.foreground.r,
                                    g: style.foreground.g,
                                    b: style.foreground.b,
                                });
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

                println!("took {}", format_duration(Instant::now().duration_since(started_at)));
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
                    Ok(mut f) => {
                        #[cfg(unix)]
                        let r = f.read_at(&mut buffer, offset);

                        #[cfg(not(unix))]
                        let r = f.seek_read(&mut buffer, offset);

                        r
                    },
                    Err(e) => {
                        println!("{e:?}");
                        return;
                    },
                };

                let bytes_read = match read_result {
                    Ok(n) => n,
                    Err(e) => {
                        println!("{e:?}");
                        return;
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
                    let offset_fmt = format!("{offset:08x}");
                    let offset_color = if offset & 255 == 0 {
                        LineColor::All(colors::GREEN)
                    } else {
                        LineColor::All(colors::WHITE)
                    };

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

                    // it makes sense because all the rows have the same dimension
                    let column_widths = vec![
                        offset_fmt.len(),
                        bytes_fmt.len(),
                        ascii_fmt.len(),
                    ];

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
                        break;
                    }
                }

                print_horizontal_line(
                    None,
                    total_width,
                    (false, true),
                    (true, true),
                );
            }
        },
        None => {
            // TODO: what do I do here?
        },
    }
}

fn add_nested_contents<'a>(
    contents: Vec<&'a File>,
    config: &PrintDirConfig,
) -> (Vec<&'a File>, Vec<usize>) {
    let mut number_of_children_to_show = HashMap::new();
    let mut remaining_rows = config.max_row - contents.len();

    for content in contents.iter() {
        let children_num = content.get_children_num(config.show_hidden_files);

        if children_num > 0 && remaining_rows > 0 {
            number_of_children_to_show.insert(content.uid, 1);
            remaining_rows -= 1;
        }

        else {
            number_of_children_to_show.insert(content.uid, 0);
        }
    }

    loop {
        if remaining_rows < 4 {
            break;
        }

        let mut added_something = false;

        for content in contents.iter() {
            let children_num = content.get_children_num(config.show_hidden_files);
            let children_to_show = number_of_children_to_show.get_mut(&content.uid).unwrap();

            if remaining_rows > 0 && *children_to_show < children_num {
                *children_to_show += 1;
                remaining_rows -= 1;
                added_something = true;
            }
        }

        if !added_something {
            break;
        }
    }

    // TODO: if there're still remaining rows, show level-2 contents

    let mut new_contents = vec![];
    let mut nested_levels = vec![];

    for content in contents.iter() {
        new_contents.push(content.uid);
        nested_levels.push(0);
        let children_to_show = *number_of_children_to_show.get(&content.uid).unwrap();

        if children_to_show > 0 {
            let mut children = content.get_children(config.show_hidden_files);
            sort_files(&mut children, config.sort_by, config.sort_reverse);

            for child in children[..children_to_show].iter() {
                new_contents.push(child.uid);
                nested_levels.push(1);
            }

            if children.len() > children_to_show {
                new_contents.push(File::message_for_truncated_rows(children.len() - children_to_show));
                nested_levels.push(1);
            }
        }
    }

    (
        new_contents.iter().map(
            |uid| get_file_by_uid(*uid).unwrap() as &File
        ).collect(),
        nested_levels,
    )
}

// you can either
// 1. color the entire line with the same color
// 2. color each character
#[derive(Clone)]
enum LineColor {
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
        print!("│");
    }

    if contents.len() > 0 {
        print!(
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
            print!("{}", part.on_color(background));
        }

        print!(
            "{}",
            " ".repeat(margin).on_color(background),
        );

        curr_table_width += margin + widths[i];
    }

    if borders.1 {
        print!("│");
    }

    print!("\n");
}

fn render_indented_message(
    indent_level: usize,
    use_half_arrow: bool,
    message: &str,
) -> String {
    match indent_level {
        0 => message.to_string(),
        1 if use_half_arrow => format!("╰── {message}"),
        1 => format!("├── {message}"),
        _ => unreachable!(),
    }
}

fn print_horizontal_line(
    background: Option<Color>,
    width: usize,
    vertical_position: (bool, bool),  // (is top, is bottom)
    borders: (bool, bool),  // (left, right)
) {
    if borders.0 {  // left border
        if vertical_position.0 {  // is top
            print!("╭");
        }

        else if vertical_position.1 {  // is bottom
            print!("╰");
        }

        else {
            print!("├");
        }
    }

    if let Some(c) = background {
        print!("{}", "─".repeat(width).on_color(c));
    }

    else {
        print!("{}", "─".repeat(width));
    }

    if borders.1 {  // right border
        if vertical_position.0 {  // is top
            print!("╮");
        }

        else if vertical_position.1 {  // is bottom
            print!("╯");
        }

        else {
            print!("┤");
        }
    }

    print!("\n");
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

// it doesn't check whether `content` has arrows or not
// it always assumes that there is
fn color_arrows(
    default_color: Color,
    arrow_color: Color,
    content: &str,
) -> LineColor {
    let mut result = vec![];
    let mut has_met_non_arrow_char = false;

    for c in content.chars() {
        if has_met_non_arrow_char {
            result.push(default_color);
        }

        else {
            if c == '├' || c == '─' || c == '╰' || c == ' ' {
                result.push(arrow_color);
            }

            else {
                result.push(default_color);
                has_met_non_arrow_char = true;
            }
        }
    }

    LineColor::Each(result)
}
