use std::fs::File;
use std::io::{self, Read, Write, BufReader, BufWriter};
use std::time::Instant;
use std::str::FromStr;

pub struct CompressMetrics {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub elapsed_ms: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImplHint {
    Auto,
    Lib,
    Pure,
}

impl FromStr for ImplHint {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "lib" => ImplHint::Lib,
            "pure" => ImplHint::Pure,
            _ => ImplHint::Auto,
        })
    }
}

// CRC32 (IEEE) table
fn crc32_init() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xEDB88320u32 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        *slot = c;
    }
    table
}

fn crc32_update(crc: u32, table: &[u32; 256], buf: &[u8]) -> u32 {
    let mut c = !crc;
    for &b in buf {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

/// Gzip (store-only) implementation without libs.
pub fn compress_gzip_pure(input: &str, output: &str) -> io::Result<CompressMetrics> {
    let start = Instant::now();
    let mut infile = BufReader::new(File::open(input)?);
    let mut outfile = BufWriter::new(File::create(output)?);

    // Write gzip header: ID1 ID2 CM FLG MTIME XFL OS
    outfile.write_all(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff])?;

    let table = crc32_init();
    let mut crc: u32 = 0;
    let mut total_in: u64 = 0;
    let mut buf = [0u8; 64 * 1024];

    loop {
        let n = infile.read(&mut buf)?;
        if n == 0 { break; }
        total_in += n as u64;
        crc = crc32_update(crc, &table, &buf[..n]);

        let mut offset = 0;
        while offset < n {
            let remaining = n - offset;
            let block_len = remaining.min(65535);
            let is_last_block = if offset + block_len >= n { 1u8 } else { 0u8 };
            outfile.write_all(&[is_last_block])?;
            let len = block_len as u16;
            let nlen = !len;
            outfile.write_all(&len.to_le_bytes())?;
            outfile.write_all(&nlen.to_le_bytes())?;
            outfile.write_all(&buf[offset..offset + block_len])?;
            offset += block_len;
        }
    }

    outfile.write_all(&crc.to_le_bytes())?;
    outfile.write_all(&(total_in as u32).to_le_bytes())?;
    outfile.flush()?;

    let bytes_out = std::fs::metadata(output)?.len();
    Ok(CompressMetrics { bytes_in: total_in, bytes_out, elapsed_ms: start.elapsed().as_millis() })
}

pub fn compress_gzip_select(input: &str, impl_hint: ImplHint) -> io::Result<(String, CompressMetrics)> {
    let out_path = format!("{}.gz", input);
    match impl_hint {
        ImplHint::Pure => {
            let m = compress_gzip_pure(input, &out_path)?;
            Ok((out_path, m))
        }
        ImplHint::Lib => {
            let m = compress_gzip_lib(input, &out_path)?;
            Ok((out_path, m))
        }
        ImplHint::Auto => {
            match compress_gzip_lib(input, &out_path) {
                Ok(m) => Ok((out_path, m)),
                Err(_) => {
                    let m = compress_gzip_pure(input, &out_path)?;
                    Ok((out_path, m))
                }
            }
        }
    }
}

pub fn compress_xz_select(input: &str, impl_hint: ImplHint) -> io::Result<(String, CompressMetrics)> {
    let out_path = format!("{}.xz", input);
    match impl_hint {
        ImplHint::Pure => Err(io::Error::other("xz pure impl not available")),
        ImplHint::Lib | ImplHint::Auto => {
            let m = compress_xz_lib(input, &out_path)?;
            Ok((out_path, m))
        }
    }
}

// Library-backed gzip
fn compress_gzip_lib(input: &str, output: &str) -> io::Result<CompressMetrics> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    let start = Instant::now();
    let mut infile = BufReader::new(File::open(input)?);
    let outfile = BufWriter::new(File::create(output)?);
    let mut enc = GzEncoder::new(outfile, Compression::default());
    let mut buf = [0u8; 64 * 1024];
    let mut total_in: u64 = 0;
    loop {
        let n = infile.read(&mut buf)?;
        if n == 0 { break; }
        total_in += n as u64;
        enc.write_all(&buf[..n])?;
    }
    let mut out = enc.finish()?;
    out.flush()?;
    let bytes_out = std::fs::metadata(output)?.len();
    Ok(CompressMetrics { bytes_in: total_in, bytes_out, elapsed_ms: start.elapsed().as_millis() })
}

// Library-backed xz
fn compress_xz_lib(input: &str, output: &str) -> io::Result<CompressMetrics> {
    use xz2::write::XzEncoder;
    let start = Instant::now();
    let mut infile = BufReader::new(File::open(input)?);
    let outfile = BufWriter::new(File::create(output)?);
    let mut enc = XzEncoder::new(outfile, 6);
    let mut buf = [0u8; 64 * 1024];
    let mut total_in: u64 = 0;
    loop {
        let n = infile.read(&mut buf)?;
        if n == 0 { break; }
        total_in += n as u64;
        enc.write_all(&buf[..n])?;
    }
    let mut out = enc.finish()?;
    out.flush()?;
    let bytes_out = std::fs::metadata(output)?.len();
    Ok(CompressMetrics { bytes_in: total_in, bytes_out, elapsed_ms: start.elapsed().as_millis() })
}
