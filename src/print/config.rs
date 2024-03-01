use crate::utils::SortBy;

pub struct PrintDirConfig {
    pub max_row: usize,
    pub sort_by: SortBy,
    pub sort_reverse: bool,
    pub show_full_path: bool,
    pub show_hidden_files: bool,
    pub table_width: usize,
}

impl Default for PrintDirConfig {
    fn default() -> Self {
        PrintDirConfig {
            max_row: 60,
            sort_by: SortBy::Name,
            sort_reverse: false,
            show_full_path: false,
            show_hidden_files: false,
            table_width: 96,
        }
    }
}
