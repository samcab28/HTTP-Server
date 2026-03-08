//! Streaming file processing (wordcount, grep)

use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug, Clone, Copy, Default)]
pub struct WordCount {
    pub lines: u64,
    pub words: u64,
    pub bytes: u64,
}

pub fn word_count(path: &str) -> io::Result<WordCount> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut count = WordCount::default();
    let mut in_word = false;
    loop {
        // Limit the scope of the immutable borrow from fill_buf
        let len = {
            let buf = reader.fill_buf()?;
            if buf.is_empty() { 0 } else {
                count.bytes += buf.len() as u64;
                for &b in buf {
                    if b == b'\n' { count.lines += 1; }
                    let is_ws = matches!(b, b' ' | b'\n' | b'\t' | b'\r' | 0x0c | 0x0b);
                    if is_ws {
                        if in_word { count.words += 1; in_word = false; }
                    } else {
                        in_word = true;
                    }
                }
                buf.len()
            }
        };
        if len == 0 { break; }
        reader.consume(len);
    }
    if in_word { count.words += 1; }
    Ok(count)
}

#[derive(Debug, Default)]
pub struct GrepResult {
    pub matches: u64,
    pub first_lines: Vec<String>,
}

/// Grep with options: case-insensitive and overlapping control.
pub fn grep_file_opts(
    path: &str,
    pattern: &str,
    max_preview: usize,
    icase: bool,
    overlap: bool,
) -> io::Result<GrepResult> {
    if pattern.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "pattern cannot be empty"));
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut res = GrepResult { matches: 0, first_lines: Vec::new() };
    let pat_norm = if icase { pattern.to_lowercase() } else { pattern.to_string() };
    for line in reader.lines() {
        let line = line?;
        let line_norm_owned;
        let line_norm: &str = if icase {
            line_norm_owned = line.to_lowercase();
            &line_norm_owned
        } else {
            &line
        };

        let mut occ = 0u64;
        let mut pos = 0usize;
        while let Some(idx) = line_norm[pos..].find(&pat_norm) {
            occ += 1;
            pos += idx + if overlap { 1 } else { pat_norm.len() };
        }
        if occ > 0 {
            res.matches += occ;
            if res.first_lines.len() < max_preview {
                res.first_lines.push(line);
            }
        }
    }
    Ok(res)
}

/// Backward-compatible: case-sensitive, non-overlapping
pub fn grep_file(path: &str, pattern: &str, max_preview: usize) -> io::Result<GrepResult> {
    grep_file_opts(path, pattern, max_preview, false, false)
}
