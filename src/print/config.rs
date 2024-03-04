use super::Alignment;
use super::result::ViewerKind;
use terminal_size::{self as ts, terminal_size};

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
    pub max_width: usize,
    pub min_width: usize,

    // every index is 0-based
    pub offset: usize,

    pub prompt: String,
    pub show_elapsed_time: bool,

    // columns[0] MUST BE ColumnKind::Index
    // columns[1] MUST BE ColumnKind::Name
    // users can set columns[2..]
    pub columns: Vec<ColumnKind>,
}

impl PrintDirConfig {
    pub fn adjust_output_dimension(&mut self) {
        if let Some((ts::Width(w), ts::Height(h))) = terminal_size() {
            let w = w as usize;
            let h = h as usize;
            self.max_width = w.max(36) - 4;
            self.min_width = self.max_width >> 2;
            self.max_row = h.max(28).min(168) - 8;
        }
    }

    pub fn reset_prompt(&mut self) {
        self.prompt = String::new();
        self.show_elapsed_time = true;
    }

    pub fn into_sql_string(&self) -> String {
        format!(
            "SELECT {} FROM cwd{} ORDER BY {}{} LIMIT {}{};",
            self.columns[1..].iter().map(|col| col.col_name()).collect::<Vec<_>>().join(", "),
            if !self.show_hidden_files { " WHERE is_hidden=false" } else { "" },
            self.sort_by.col_name(),
            if self.sort_reverse { " DESC" } else { "" },
            self.max_row,
            if self.offset != 0 { format!(" OFFSET {}", self.offset) } else { String::new() },
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
            max_width: 120,
            min_width: 64,
            offset: 0,
            prompt: String::new(),
            show_elapsed_time: true,
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

pub enum FileReadMode {
    Infer,
    Force(ViewerKind),
}

impl Default for FileReadMode {
    fn default() -> Self {
        FileReadMode::Infer
    }
}

pub struct PrintFileConfig {
    pub max_row: usize,
    pub max_width: usize,
    pub min_width: usize,

    // for text files, it's a line offset
    // for hex files, it's a byte offset
    // for image files, it's a row offset
    pub offset: usize,

    pub prompt: String,
    pub show_elapsed_time: bool,

    // every index is 0-based
    // for text files, it's a line offset
    // for hex files, it's a byte offset
    // for image files, it does nothing
    // make sure that it's sorted
    pub highlights: Vec<usize>,

    pub read_mode: FileReadMode,
}

impl PrintFileConfig {
    pub fn adjust_output_dimension(&mut self) {
        if let Some((ts::Width(w), ts::Height(h))) = terminal_size() {
            let w = w as usize;
            let h = h as usize;
            self.max_width = w.max(36) - 4;
            self.min_width = self.max_width >> 2;
            self.max_row = h.max(28).min(168) - 8;
        }
    }

    pub fn reset_prompt(&mut self) {
        self.prompt = String::new();
        self.show_elapsed_time = true;
    }
}

impl Default for PrintFileConfig {
    fn default() -> Self {
        PrintFileConfig {
            max_row: 60,
            max_width: 120,
            min_width: 64,
            offset: 0,
            prompt: String::new(),
            show_elapsed_time: true,
            highlights: vec![],
            read_mode: FileReadMode::Infer,
        }
    }
}

pub struct PrintLinkConfig {
    pub max_row: usize,
    pub max_width: usize,
    pub min_width: usize,
    pub prompt: String,
    pub show_elapsed_time: bool,
}

impl PrintLinkConfig {
    pub fn adjust_output_dimension(&mut self) {
        if let Some((ts::Width(w), ts::Height(h))) = terminal_size() {
            let w = w as usize;
            let h = h as usize;
            self.max_width = w.max(36) - 4;
            self.min_width = self.max_width >> 2;
            self.max_row = h.max(28).min(168) - 8;
        }
    }

    pub fn reset_prompt(&mut self) {
        self.prompt = String::new();
        self.show_elapsed_time = true;
    }
}

impl Default for PrintLinkConfig {
    fn default() -> Self {
        PrintLinkConfig {
            max_row: 60,
            max_width: 120,
            min_width: 64,
            prompt: String::new(),
            show_elapsed_time: true,
        }
    }
}
