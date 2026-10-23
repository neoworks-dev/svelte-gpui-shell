//! Splitting a byte stream into lines, for process output and sockets.

use std::io::{ErrorKind, Read};

/// A line longer than this is delivered in pieces, so a stream that never sends a newline
/// cannot grow the buffer without end.
pub const MAX_LINE: usize = 1 << 20;

/// Collects bytes and hands back the lines they complete. A line ends at `\n`, and a `\r`
/// in front of it is dropped. Text that is not UTF-8 is read with replacement characters.
#[derive(Default)]
pub struct LineSplitter {
    pending: Vec<u8>,
}

impl LineSplitter {
    /// Adds a chunk, and returns the lines it completed.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        let mut rest = chunk;
        while let Some(end) = rest.iter().position(|&byte| byte == b'\n') {
            self.pending.extend_from_slice(&rest[..end]);
            lines.push(self.take_line());
            rest = &rest[end + 1..];
        }
        self.pending.extend_from_slice(rest);
        while self.pending.len() > MAX_LINE {
            let tail = self.pending.split_off(MAX_LINE);
            lines.push(self.take_line());
            self.pending = tail;
        }
        lines
    }

    /// The text after the last newline, when the stream ended without one.
    pub fn finish(&mut self) -> Option<String> {
        if self.pending.is_empty() { None } else { Some(self.take_line()) }
    }

    fn take_line(&mut self) -> String {
        if self.pending.last() == Some(&b'\r') {
            self.pending.pop();
        }
        let line = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        line
    }
}

/// Reads `reader` to its end, calling `line` for each line as soon as it is complete.
pub fn for_each_line(mut reader: impl Read, mut line: impl FnMut(String)) {
    let mut splitter = LineSplitter::default();
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => splitter.push(&buffer[..count]).into_iter().for_each(&mut line),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    if let Some(last) = splitter.finish() {
        line(last);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_on_newlines_across_chunks() {
        let mut splitter = LineSplitter::default();
        assert_eq!(splitter.push(b"one\ntw"), vec!["one"]);
        assert_eq!(splitter.push(b"o\nthree\n"), vec!["two", "three"]);
        assert_eq!(splitter.finish(), None);
    }

    #[test]
    fn keeps_empty_lines_and_drops_carriage_returns() {
        let mut splitter = LineSplitter::default();
        assert_eq!(splitter.push(b"a\r\n\nb\r\n"), vec!["a", "", "b"]);
    }

    #[test]
    fn the_last_line_needs_no_newline() {
        let mut splitter = LineSplitter::default();
        assert!(splitter.push(b"tail").is_empty());
        assert_eq!(splitter.finish(), Some("tail".to_string()));
        assert_eq!(splitter.finish(), None);
    }

    #[test]
    fn a_character_split_between_chunks_survives() {
        let bytes = "é\n".as_bytes();
        let mut splitter = LineSplitter::default();
        assert!(splitter.push(&bytes[..1]).is_empty());
        assert_eq!(splitter.push(&bytes[1..]), vec!["é"]);
    }

    #[test]
    fn a_very_long_line_comes_out_in_pieces() {
        let mut splitter = LineSplitter::default();
        let lines = splitter.push(&vec![b'x'; MAX_LINE * 2 + 5]);
        assert_eq!(lines.len(), 2);
        assert_eq!(splitter.finish().map(|rest| rest.len()), Some(5));
    }

    #[test]
    fn reads_a_stream_to_its_end() {
        let mut seen = Vec::new();
        for_each_line(&b"1\n2\n3"[..], |line| seen.push(line));
        assert_eq!(seen, vec!["1", "2", "3"]);
    }
}
