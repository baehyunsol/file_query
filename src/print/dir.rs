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
use super::config::{ColumnKind, PrintDirConfig};
use super::result::PrintDirResult;
use super::utils::{
    colorize_name,
    colorize_size,
    colorize_time,
    colorize_type,
    format_duration,
    prettify_size,
    prettify_time,
};
use colored::Color;
use crate::colors;
use crate::file::File;
use crate::uid::Uid;
use crate::utils::{
    get_file_by_uid,
    get_path_by_uid,
    sort_files,
};
use std::collections::HashMap;
use std::time::{Instant, SystemTime};

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

/// It does NOT check whether the given `uid` is dir or not.
/// It assumes that the given `uid` is valid.
pub fn print_dir(
    uid: Uid,
    config: &PrintDirConfig,
) -> PrintDirResult {
    let started_at = Instant::now();
    let file = get_file_by_uid(uid).unwrap();

    file.init_children();

    let mut children_instances = file.get_children(config.show_hidden_files);

    // num of children BEFORE truncated
    let children_num = children_instances.len();
    let curr_dir_path = match get_path_by_uid(uid) {
        Some(path) => path,
        None => {
            print_error_message(
                Some(file),
                None,
                format!("get_path_by_uid({}) has failed", uid.debug_info()),
                config.min_width,
                config.max_width,
            );
            return PrintDirResult::error();
        },
    };

    sort_files(&mut children_instances, config.sort_by, config.sort_reverse);

    // it shows contents inside dirs (if there are enough rows)
    let mut nested_levels;

    if config.offset > 0 {
        children_instances = children_instances[config.offset.min(children_instances.len().max(1) - 1)..].to_vec();
    }

    if children_instances.len() > config.max_row {
        children_instances = children_instances[..config.max_row].to_vec();
        nested_levels = vec![0; config.max_row];
    }

    else if children_instances.len() + 4 < config.max_row {
        let (children_instances_, nested_levels_) = add_nested_contents(
            children_instances,
            &config,
        );
        children_instances = children_instances_;
        nested_levels = nested_levels_;
    }

    else {
        nested_levels = vec![0; children_instances.len()];
    }

    let now = SystemTime::now();

    // we don't called offseted rows 'truncated'
    let shown_rows = nested_levels.iter().filter(|level| **level == 0).count();
    let mut truncated_rows = children_num.max(shown_rows + config.offset) - shown_rows - config.offset;

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

    let mut table_index = config.offset;
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
    println_to_buffer!("{}", config.into_sql_string());
    println_to_buffer!("took {}", format_duration(Instant::now().duration_since(started_at)));

    PrintDirResult::success()
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
