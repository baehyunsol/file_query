use super::Alignment;
use crate::utils::SortBy;

pub enum ColumnKind {
    Index,
    Name,
    Size,
    TotalSize,
    Modified,
    FileType,
    FileExt,
}

impl ColumnKind {
    pub fn header_string(&self) -> String {
        match self {
            ColumnKind::Index => "index",
            ColumnKind::Name => "name",
            ColumnKind::Size => "size",
            ColumnKind::TotalSize => "total size",
            ColumnKind::Modified => "modified",
            ColumnKind::FileType => "type",
            ColumnKind::FileExt => "extension",
        }.to_string()
    }

    pub fn alignment(&self) -> Alignment {
        match self {
            ColumnKind::Index => Alignment::Right,
            ColumnKind::Name => Alignment::Left,
            ColumnKind::Size => Alignment::Right,
            ColumnKind::TotalSize => Alignment::Right,
            ColumnKind::Modified => Alignment::Right,
            ColumnKind::FileType => Alignment::Left,
            ColumnKind::FileExt => Alignment::Left,
        }
    }
}

pub struct PrintDirConfig {
    pub max_row: usize,
    pub sort_by: SortBy,
    pub sort_reverse: bool,
    pub show_full_path: bool,
    pub show_hidden_files: bool,
    pub table_max_width: usize,
    pub table_min_width: usize,

    // columns[0] MUST BE ColumnKind::Index
    // columns[1] MUST BE ColumnKind::Name
    // users can set columns[2..]
    pub columns: Vec<ColumnKind>,
}

impl Default for PrintDirConfig {
    fn default() -> Self {
        PrintDirConfig {
            max_row: 60,
            sort_by: SortBy::Name,
            sort_reverse: false,
            show_full_path: false,
            show_hidden_files: false,
            table_max_width: 120,
            table_min_width: 64,
            columns: vec![
                ColumnKind::Index,
                ColumnKind::Name,
                ColumnKind::FileType,
                ColumnKind::FileExt,
                ColumnKind::Modified,
                ColumnKind::Size,
            ],
        }
    }
}
