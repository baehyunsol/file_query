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
use std::collections::HashMap;
use std::time::SystemTime;

mod config;
mod utils;

const COLUMN_MARGIN: usize = 2;

pub use config::PrintDirConfig;
use utils::{
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

            // print curr dir
            print_row(
                colors::BLACK,
                vec![
                    curr_dir_path,
                    &format!("{} elements", children_num),
                ],
                &vec![  // TODO: make it configurable (according to the size of the term)
                    64,
                    18,  // num of elements
                ],
                &vec![
                    Alignment::Left,   // path
                    Alignment::Left,   // num of elements
                ],
                &vec![
                    colors::WHITE,  // path
                    colors::BLUE,   // num of elements
                ],
                COLUMN_MARGIN,
                Some(config.table_width),
            );

            print_horizontal_line(
                None,  // background
                config.table_width,
            );

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
            let column_widths = vec![
                if config.max_row < 100 { 5 } else { 8 },   // index
                32,  // name
                6,   // file type
                18,  // last modified
                9,   // size
            ];
            let column_alignments = vec![
                Alignment::Right,  // index
                Alignment::Left,   // name
                Alignment::Left,   // file type
                Alignment::Right,  // last modified
                Alignment::Right,  // size
            ];

            // column names
            print_row(
                colors::GRAY,
                vec![
                    "index",
                    "name",
                    "type",
                    "modified",
                    "size",
                ],
                &column_widths,
                &vec![Alignment::Center; column_widths.len()],
                &vec![colors::WHITE; column_widths.len()],
                COLUMN_MARGIN,
                Some(config.table_width),
            );

            let truncated_rows = children_num - nested_levels.iter().filter(|level| **level == 0).count();

            if truncated_rows > 0 {
                children_instances.push(
                    // very ugly, but there's no other way than this to fool the borrow checker
                    get_file_by_uid(File::message_for_truncated_rows(truncated_rows)).unwrap() as &File
                );
                nested_levels.push(0);
            }

            debug_assert_eq!(
                children_instances.len(),
                nested_levels.len(),
            );

            let mut table_index = 0;
            let mut table_sub_index = 0;

            for (index, child) in children_instances.iter().enumerate() {
                let background = if index & 1 == 1 { colors::GRAY } else { colors::BLACK };
                let nested_level = nested_levels[index];
                let has_to_use_half_arrow = nested_level > 0 && (index == nested_levels.len() - 1 || nested_levels[index + 1] < nested_level);

                if child.is_special_file() {
                    let message = render_indented_message(
                        nested_level,
                        has_to_use_half_arrow,
                        &child.name,
                    );

                    print_row(
                        background,
                        vec![
                            "",  // index
                            &message,
                        ],
                        &vec![
                            column_widths[0],
                            column_widths[1..].iter().sum::<usize>(),
                        ],
                        &vec![Alignment::Right, Alignment::Left],
                        &vec![colors::WHITE; 2],
                        COLUMN_MARGIN,
                        Some(config.table_width),
                    );

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

                print_row(
                    background,
                    vec![
                        &table_index_formatted,
                        &name,
                        &child.file_type.to_string(),
                        &prettify_time(&now, child.last_modified),
                        &prettify_size(child.size),
                    ],
                    &column_widths,
                    &column_alignments,
                    &vec![
                        colors::WHITE,
                        colors::WHITE,
                        colorize_type(child.file_type),
                        colorize_time(&now, child.last_modified),
                        colorize_size(child.size),
                    ],
                    COLUMN_MARGIN,
                    Some(config.table_width),
                );
            }
        },
        None => {
            // TODO: what do I do here?
        },
    }
}

pub fn print_file(uid: Uid) {}

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

// TODO: colorize '├──'
// for that, I have to make sure that the file names never contain non-ascii chars
fn print_row(
    background: Color,
    contents: Vec<&str>,
    widths: &Vec<usize>,
    alignments: &Vec<Alignment>,
    colors: &Vec<Color>,
    margin: usize,
    fill_width: Option<usize>,
) {
    debug_assert_eq!(contents.len(), widths.len());
    debug_assert_eq!(contents.len(), alignments.len());
    debug_assert_eq!(contents.len(), colors.len());
    let mut curr_table_width = 0;

    if contents.len() > 0 {
        print!(
            "{}",
            " ".repeat(margin).on_color(background),
        );

        curr_table_width += margin;
    }

    for i in 0..contents.len() {
        let curr_content_len = contents[i].chars().count();

        if curr_content_len <= widths[i] {
            let left_margin = match alignments[i] {
                Alignment::Left => 0,
                Alignment::Center => (widths[i] - curr_content_len) >> 1,
                Alignment::Right => widths[i] - curr_content_len,
            };
            let right_margin = widths[i] - curr_content_len - left_margin;

            let line = format!(
                "{}{}{}",
                " ".repeat(left_margin),
                contents[i].color(colors[i]),
                " ".repeat(right_margin),
            );

            print!("{}", line.on_color(background));
        }

        else {
            // TODO: how do I make sure that widths[i] >= 3?
            let first_half = (widths[i] - 3) >> 1;
            let last_half = widths[i] - 3 - first_half;

            let line = format!(
                "{}...{}",
                // TODO: it has to make sure that all the chars are single-byte
                contents[i].get(..first_half).unwrap().color(colors[i]),
                contents[i].get((curr_content_len - last_half)..).unwrap().color(colors[i]),
            );

            print!("{}", line.on_color(background));
        }

        print!(
            "{}",
            " ".repeat(margin).on_color(background),
        );

        curr_table_width += margin + widths[i];
    }

    if let Some(width) = fill_width {
        if curr_table_width < width {
            print!(
                "{}",
                " ".repeat(width - curr_table_width).on_color(background),
            );
        }
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
) {
    if let Some(c) = background {
        println!("{}", "─".repeat(width).on_color(c));
    }

    else {
        println!("{}", "─".repeat(width));
    }
}
