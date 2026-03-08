use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct SortMetrics {
    pub lines: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub runs: usize,
    pub elapsed_ms: u128,
}

/// Merge sort for large files (external merge sort) where the input contains
/// one integer per line. Implements run generation + k-way merge.
pub fn mergesort_file_external(input: &str) -> io::Result<(String, SortMetrics)> {
    let start = Instant::now();
    let input_path = Path::new(input);
    let parent = input_path.parent().unwrap_or(Path::new("."));
    let out_path = derive_sorted_path(input_path);
    let mut runs: Vec<PathBuf> = Vec::new();
    let mut total_lines: u64 = 0;
    let mut total_bytes_in: u64 = 0;

    let file = File::open(input_path)?;
    let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file);
    let mut buf = String::new();
    let mut chunk: Vec<i64> = Vec::with_capacity(1 << 20);
    let mut chunk_bytes: usize = 0;
    let chunk_limit_bytes: usize = 16 * 1024 * 1024; // 16MB chunks

    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            // flush last chunk
            if !chunk.is_empty() {
                runs.push(write_sorted_run(parent, &mut chunk[..])?);
                chunk.clear();
            }
            break;
        }
        total_bytes_in += n as u64;
        // trim newline
        if let Some('\n') = buf.chars().last() { buf.pop(); }
        if buf.ends_with('\r') { buf.pop(); }
        if !buf.is_empty() {
            if let Ok(v) = buf.trim().parse::<i64>() {
                chunk.push(v);
                chunk_bytes += n;
                total_lines += 1;
            }
        }
        if chunk_bytes >= chunk_limit_bytes {
            runs.push(write_sorted_run(parent, &mut chunk[..])?);
            chunk.clear();
            chunk_bytes = 0;
        }
    }

    // If only one run, just rename/copy
    let mut bytes_out = 0u64;
    if runs.is_empty() {
        // Empty input
        let mut w = BufWriter::new(File::create(&out_path)?);
        w.flush()?;
    } else if runs.len() == 1 {
        fs::copy(&runs[0], &out_path)?;
        let md = fs::metadata(&out_path)?; bytes_out = md.len();
        // cleanup
        let _ = fs::remove_file(&runs[0]);
    } else {
        bytes_out = k_way_merge(&runs, &out_path)?;
        // cleanup runs
        for r in &runs { let _ = fs::remove_file(r); }
    }

    let elapsed_ms = start.elapsed().as_millis();
    Ok((out_path.to_string_lossy().to_string(), SortMetrics {
        lines: total_lines,
        bytes_in: total_bytes_in,
        bytes_out,
        runs: runs.len().max(1),
        elapsed_ms,
    }))
}

fn derive_sorted_path(input: &Path) -> PathBuf {
    let os = input.as_os_str().to_owned();
    let mut s = os.to_string_lossy().to_string();
    s.push_str(".sorted");
    PathBuf::from(s)
}

fn write_sorted_run(dir: &Path, chunk: &mut [i64]) -> io::Result<PathBuf> {
    chunk.sort_unstable();
    let mut path = dir.to_path_buf();
    let fname = format!("run-{}.tmp", nano_time());
    path.push(fname);
    let mut w = BufWriter::with_capacity(8 * 1024 * 1024, File::create(&path)?);
    for &v in chunk.iter() {
        writeln!(w, "{}", v)?;
    }
    w.flush()?;
    Ok(path)
}

fn nano_time() -> u128 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() }

#[derive(Eq)]
struct HeapItem {
    value: i64,
    idx: usize,
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // min-heap via reverse
        other.value.cmp(&self.value).then_with(|| other.idx.cmp(&self.idx))
    }
}
impl PartialOrd for HeapItem { fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) } }
impl PartialEq for HeapItem { fn eq(&self, other: &Self) -> bool { self.value == other.value && self.idx == other.idx } }

fn k_way_merge(runs: &[PathBuf], out_path: &Path) -> io::Result<u64> {
    let mut readers: Vec<BufReader<File>> = Vec::with_capacity(runs.len());
    for p in runs {
        readers.push(BufReader::new(File::open(p)?));
    }
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();
    let mut curr_vals: Vec<Option<i64>> = vec![None; readers.len()];
    let mut line = String::new();
    for (i, r) in readers.iter_mut().enumerate() {
        line.clear();
        if r.read_line(&mut line)? > 0 {
            let v: i64 = line.trim().parse().unwrap_or(0);
            curr_vals[i] = Some(v);
            heap.push(HeapItem { value: v, idx: i });
        }
    }
    let mut out = BufWriter::with_capacity(8 * 1024 * 1024, File::create(out_path)?);
    let mut bytes_out: u64 = 0;
    while let Some(HeapItem { value, idx }) = heap.pop() {
        let s = format!("{}\n", value);
        out.write_all(s.as_bytes())?;
        bytes_out += s.len() as u64;
        // refill from that reader
        line.clear();
        if readers[idx].read_line(&mut line)? > 0 {
            let v: i64 = line.trim().parse().unwrap_or(0);
            curr_vals[idx] = Some(v);
            heap.push(HeapItem { value: v, idx });
        } else {
            curr_vals[idx] = None;
        }
    }
    out.flush()?;
    Ok(bytes_out)
}

/// Quick sort path for medium files. Loads all integers into memory,
/// sorts using quicksort (Rust's sort_unstable), and writes a .sorted file.
pub fn quicksort_file(input: &str) -> io::Result<(String, SortMetrics)> {
    let start = Instant::now();
    let input_path = Path::new(input);
    let out_path = derive_sorted_path(input_path);

    let file = File::open(input_path)?;
    let md = fs::metadata(input_path)?;
    let mut reader = BufReader::with_capacity(8 * 1024 * 1024, file);
    let mut buf = String::new();
    let mut values: Vec<i64> = Vec::new();
    let mut lines: u64 = 0;
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 { break; }
        if let Some('\n') = buf.chars().last() { buf.pop(); }
        if buf.ends_with('\r') { buf.pop(); }
        if !buf.is_empty() {
            if let Ok(v) = buf.trim().parse::<i64>() {
                values.push(v);
                lines += 1;
            }
        }
    }
    values.sort_unstable();

    let mut w = BufWriter::with_capacity(8 * 1024 * 1024, File::create(&out_path)?);
    for v in &values { writeln!(w, "{}", v)?; }
    w.flush()?;
    let bytes_out = fs::metadata(&out_path)?.len();
    Ok((out_path.to_string_lossy().to_string(), SortMetrics{
        lines,
        bytes_in: md.len(),
        bytes_out,
        runs: 1,
        elapsed_ms: start.elapsed().as_millis(),
    }))
}
