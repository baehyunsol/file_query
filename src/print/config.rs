use super::Alignment;

#[derive(Clone, Copy)]
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

    pub fn col_name(&self) -> String {
        match self {
            ColumnKind::Index => "index",
            ColumnKind::Name => "name",
            ColumnKind::Size => "size",
            ColumnKind::TotalSize => "total_size",
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
    pub sort_by: ColumnKind,
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

impl PrintDirConfig {
    pub fn into_sql_string(&self) -> String {
        format!(
            "SELECT {} FROM cwd{} ORDER BY {}{} LIMIT {};",
            self.columns[1..].iter().map(|col| col.col_name()).collect::<Vec<_>>().join(", "),
            if !self.show_hidden_files { " WHERE is_hidden=false" } else { "" },
            self.sort_by.col_name(),
            if self.sort_reverse { " DESC" } else { "" },
            self.max_row,
        )
    }
}

impl Default for PrintDirConfig {
    fn default() -> Self {
        PrintDirConfig {
            max_row: 60,
            sort_by: ColumnKind::Name,
            sort_reverse: false,
            show_full_path: false,
            show_hidden_files: false,
            table_max_width: 120,
            table_min_width: 64,
            columns: vec![
                ColumnKind::Index,
                ColumnKind::Name,
                ColumnKind::FileType,
                ColumnKind::Modified,
                ColumnKind::Size,
            ],
        }
    }
}

pub struct PrintFileConfig {
    pub max_row: usize,
    pub max_width: usize,
    pub min_width: usize,

    // read from nth line
    pub line_offset: usize,
}

impl Default for PrintFileConfig {
    fn default() -> Self {
        PrintFileConfig {
            max_row: 60,
            max_width: 120,
            min_width: 64,
            line_offset: 0,
        }
    }
}
