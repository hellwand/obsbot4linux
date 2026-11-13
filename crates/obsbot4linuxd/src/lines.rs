// SPDX-License-Identifier: GPL-3.0-or-later

//! The bounded line reader of a connection (4.1): one JSON object per line; a request line
//! longer than `MAX_REQUEST_LINE` (64 KiB) closes the connection. Never `BufRead::lines`,
//! which would buffer a line of any length.

use std::io::{self, BufRead};

/// One line from a client.
#[derive(Debug, PartialEq, Eq)]
pub enum Line {
    /// A line without its `\n`. A last line without `\n` before the end of the stream counts
    /// as a line.
    Text(String),
    /// The line is longer than the cap: the caller closes the connection. The rest of the
    /// line is not read.
    TooLong,
    /// The line is not UTF-8: answered like a line that is not JSON.
    NotUtf8,
    /// The client closed its end.
    Eof,
}

/// Reads one line of at most `cap` bytes (the `\n` not counted).
pub fn read_line(reader: &mut impl BufRead, cap: usize) -> io::Result<Line> {
    let mut line: Vec<u8> = Vec::new();
    loop {
        let chunk = match reader.fill_buf() {
            Ok(chunk) => chunk,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if chunk.is_empty() {
            if line.is_empty() {
                return Ok(Line::Eof);
            }
            return Ok(text(line));
        }
        match chunk.iter().position(|b| *b == b'\n') {
            Some(at) => {
                if line.len() + at > cap {
                    return Ok(Line::TooLong);
                }
                line.extend_from_slice(&chunk[..at]);
                reader.consume(at + 1);
                return Ok(text(line));
            }
            None => {
                let n = chunk.len();
                if line.len() + n > cap {
                    return Ok(Line::TooLong);
                }
                line.extend_from_slice(chunk);
                reader.consume(n);
            }
        }
    }
}

fn text(bytes: Vec<u8>) -> Line {
    match String::from_utf8(bytes) {
        Ok(s) => Line::Text(s),
        Err(_) => Line::NotUtf8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    fn all(input: &[u8], cap: usize) -> Vec<Line> {
        // A small buffer, so that lines cross buffer boundaries.
        let mut r = BufReader::with_capacity(7, Cursor::new(input.to_vec()));
        let mut out = Vec::new();
        loop {
            let line = read_line(&mut r, cap).unwrap();
            let end = matches!(line, Line::Eof | Line::TooLong);
            out.push(line);
            if end {
                return out;
            }
        }
    }

    #[test]
    fn lines_are_split_at_newlines_and_a_last_line_needs_none() {
        assert_eq!(
            all(b"{\"a\":1}\n\n{\"b\":2}", 100),
            [
                Line::Text("{\"a\":1}".into()),
                Line::Text(String::new()),
                Line::Text("{\"b\":2}".into()),
                Line::Eof
            ]
        );
    }

    /// 4.1: a request line longer than 64 KiB closes the connection; exactly 64 KiB is a line.
    #[test]
    fn the_cap_allows_exactly_its_length_and_refuses_one_byte_more() {
        let cap = obsbot_proto::MAX_REQUEST_LINE;
        let mut at = vec![b'x'; cap];
        at.push(b'\n');
        assert_eq!(all(&at, cap)[0], Line::Text("x".repeat(cap)));
        let mut over = vec![b'x'; cap + 1];
        over.push(b'\n');
        assert_eq!(all(&over, cap), [Line::TooLong]);
        let no_newline = vec![b'x'; cap + 1];
        assert_eq!(
            all(&no_newline, cap),
            [Line::TooLong],
            "also without a newline"
        );
    }

    #[test]
    fn a_line_that_is_not_utf8_is_reported_and_the_next_line_still_reads() {
        assert_eq!(
            all(b"\xff\xfe\n{}\n", 100),
            [Line::NotUtf8, Line::Text("{}".into()), Line::Eof]
        );
    }

    #[test]
    fn an_empty_stream_is_the_end() {
        assert_eq!(all(b"", 100), [Line::Eof]);
    }
}
