use colored::{Color, Colorize};
use crate::colors;
use crate::file::File;
use crate::uid::Uid;
use crate::utils::{
    get_path_by_uid,
    get_file_by_uid,
    sort_files,
    SortBy,
};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

mod config;
mod utils;

const COLUMN_MARGIN: usize = 2;

pub use config::{ColumnKind, PrintDirConfig};
use utils::{
    colorize_name,
    colorize_size,
    colorize_time,
    colorize_type,
    prettify_size,
    prettify_time,
};

#[derive(Clone)]
enum Alignment {
    Left, Center, Right,
}

pub fn print_dir(
    uid: Uid,
    config: PrintDirConfig,
) {
    match get_file_by_uid(uid) {
        Some(file) if file.is_file() => {
            print_file(uid);
        },
        Some(file) => {
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
            content_colors.push(vec![colors::WHITE; table_contents[0].len()]);

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
                    table_contents.push(vec![
                        String::new(),  // index
                        message,
                    ]);
                    column_alignments.push(vec![
                        Alignment::Right,
                        Alignment::Left,
                    ]);
                    content_colors.push(vec![
                        colors::WHITE,
                        colors::WHITE,
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
                            curr_content_colors.push(colors::WHITE);
                        },
                        ColumnKind::Name => {
                            curr_table_contents.push(name.clone());
                            curr_content_colors.push(colorize_name(child.file_type, child.is_executable));
                        },
                        ColumnKind::Size => {
                            curr_table_contents.push(prettify_size(child.size));
                            curr_content_colors.push(colorize_size(child.size));
                        },
                        ColumnKind::TotalSize => {
                            curr_table_contents.push(prettify_size(child.get_recursive_size()));
                            curr_content_colors.push(colorize_size(child.get_recursive_size()));
                        },
                        ColumnKind::Modified => {
                            curr_table_contents.push(prettify_time(&now, child.last_modified));
                            curr_content_colors.push(colorize_time(&now, child.last_modified));
                        },
                        ColumnKind::FileType => {
                            curr_table_contents.push(child.file_type.to_string());
                            curr_content_colors.push(colorize_type(child.file_type));
                        },
                        ColumnKind::FileExt => {
                            curr_table_contents.push(child.file_ext.clone().unwrap_or(String::new()));
                            curr_content_colors.push(colors::WHITE);
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
                Some(config.table_max_width),
                Some(config.table_min_width),
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
                    Alignment::Left,   // path
                    Alignment::Left,   // num of elements
                ],
                &vec![
                    colors::WHITE,  // path
                    colors::YELLOW,  // num of elements
                ],
                COLUMN_MARGIN,
                (true, true),
                false,  // color arrows
            );

            print_horizontal_line(
                None,  // background
                curr_table_width,
                (false, false),  // (is top, is bottom)
                (true, true),    // (left border, right border)
            );

            for index in 0..table_contents.len() {
                let background = if index & 1 == 1 { colors::GRAY } else { colors::BLACK };
                let column_widths = table_column_widths.get(&table_contents[index].len()).unwrap();

                print_row(
                    background,
                    &table_contents[index],
                    column_widths,
                    &column_alignments[index],
                    &content_colors[index],
                    COLUMN_MARGIN,
                    (true, true),
                    true,  // color arrows
                );
            }

            print_horizontal_line(
                None,  // background
                curr_table_width,
                (false, true),   // (is top, is bottom)
                (true, true),    // (left border, right border)
            );
        },
        None => {
            // TODO: what do I do here?
        },
    }
}

pub fn print_file(uid: Uid) {
    // TODO...
    // how about embedding `bat` and `viu` here?
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

fn print_row(
    background: Color,
    contents: &Vec<String>,
    widths: &Vec<usize>,
    alignments: &Vec<Alignment>,
    colors: &Vec<Color>,
    margin: usize,
    borders: (bool, bool),  // (left, right)
    color_arrows: bool,
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

            parts.push(" ".repeat(left_margin).color(colors[i]));

            // TODO: if it's level-2 or more, an arrow might start with a whitespace
            // TODO: what if a file name starts with "├"?
            if color_arrows && (contents[i].starts_with("├") || contents[i].starts_with("╰")) {
                // TODO: it has to be fixed when level-2 contents are implemented
                parts.push(contents[i].get(..9).unwrap().color(colors::GREEN));
                parts.push(contents[i].get(9..).unwrap().color(colors[i]));
            }

            else {
                parts.push(contents[i].color(colors[i]));
            }

            parts.push(" ".repeat(right_margin).color(colors[i]));
        }

        else {
            // TODO: how do I make sure that widths[i] >= 3?
            let first_half = (widths[i] - 3) >> 1;
            let last_half = widths[i] - 3 - first_half;

            let prefix = contents[i].chars().collect::<Vec<_>>()[..first_half].iter().collect::<String>();

            if color_arrows && (prefix.starts_with("├") || prefix.starts_with("╰")) {
                parts.push(prefix.get(..9).unwrap().color(colors::GREEN));
                parts.push(prefix.get(9..).unwrap().color(colors[i]));
            }

            else {
                parts.push(prefix.color(colors[i]));
            }

            parts.push(String::from("...").color(colors[i]));

            let suffix = contents[i].chars().collect::<Vec<_>>()[(curr_content_len - last_half)..].iter().collect::<String>();
            parts.push(suffix.color(colors[i]));
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

    let mut max_column_widths = table_contents[0].iter().map(|c| c.len()).collect::<Vec<_>>();
    let mut col_counts = HashSet::new();
    col_counts.insert(table_contents[0].len());

    for row in table_contents[1..].iter() {
        let curr_row_widths = row.iter().map(|c| c.len()).collect::<Vec<_>>();
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
