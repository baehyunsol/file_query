pub struct PrintDirResult {}

impl PrintDirResult {
    pub fn success() -> Self {
        PrintDirResult {}
    }

    // you MUST NOT read any of these value
    pub fn dummy() -> Self {
        PrintDirResult {}
    }

    pub fn error() -> Self {
        PrintDirResult {}
    }
}

pub enum ViewerKind {
    Text,
    Hex,
    Image,  // TODO
}

pub struct PrintFileResult {
    // I'm too lazy to use Option<PrintFileResult>
    pub is_error: bool,

    // for texts, it's width of the `contents` column
    // for hexes, it's width of the `hex` column, (number of bytes, not the printed characters)
    // for images, it's the number of columns (in characters)
    pub width: usize,

    pub viewer_kind: ViewerKind,

    // for texts, it's the last line number (if available)
    // for hexes, it's None
    // for images, it's the number of rows (always available)
    pub last_line: Option<usize>,
}

impl PrintFileResult {
    pub fn hex_success(width: usize) -> Self {
        PrintFileResult {
            is_error: false,
            width,
            viewer_kind: ViewerKind::Hex,
            last_line: None,
        }
    }

    pub fn text_success(width: usize, last_line: Option<usize>) -> Self {
        PrintFileResult {
            is_error: false,
            width,
            viewer_kind: ViewerKind::Text,
            last_line,
        }
    }

    // you MUST NOT read any of these value
    pub fn dummy() -> Self {
        PrintFileResult {
            is_error: false,
            width: 0,
            viewer_kind: ViewerKind::Text,
            last_line: None,
        }
    }

    pub fn error() -> Self {
        PrintFileResult {
            is_error: true,
            ..PrintFileResult::dummy()
        }
    }
}

pub struct PrintLinkResult {}

impl PrintLinkResult {
    pub fn success() -> Self {
        PrintLinkResult {}
    }

    // you MUST NOT read any of these value
    pub fn dummy() -> Self {
        PrintLinkResult {}
    }

    pub fn error() -> Self {
        PrintLinkResult {}
    }
}
