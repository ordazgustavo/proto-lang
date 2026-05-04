use std::{fmt::Debug, ops::Range};

#[derive(Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file: FileId,
}

impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
        // f.debug_struct("Span")
        //     .field("start", &self.start)
        //     .field("end", &self.end)
        //     .field("file", &self.file)
        //     .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u16);

impl Span {
    pub fn new(start: usize, end: usize, file: FileId) -> Self {
        Self { start, end, file }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            file: self.file,
        }
    }

    pub fn range(self) -> Range<usize> {
        self.start..self.end
    }
}
