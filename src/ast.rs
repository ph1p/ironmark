#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Block {
    Document {
        children: Vec<Block>,
    },
    BlockQuote {
        children: Vec<Block>,
    },
    List {
        kind: ListKind,
        start: u32,
        tight: bool,
        children: Vec<Block>,
    },
    ListItem {
        children: Vec<Block>,
        checked: Option<bool>,
    },
    Paragraph {
        raw: String,
    },
    Heading {
        level: u8,
        raw: String,
    },
    CodeBlock {
        info: String,
        literal: String,
    },
    HtmlBlock {
        literal: String,
    },
    ThematicBreak,
    Table(Box<TableData>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TableData {
    pub alignments: Vec<TableAlignment>,
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    Bullet(u8),  // marker character: b'-', b'*', b'+'
    Ordered(u8), // delimiter: b'.' or b')'
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TableAlignment {
    None,
    Left,
    Center,
    Right,
}
