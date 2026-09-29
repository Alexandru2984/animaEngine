//! The little of the zip format a scene file needs: entries stored as
//! they are — images and videos are compressed already — with no
//! encryption and no zip64. Written here, and read back strictly,
//! refusing everything else, so the reader stays small enough to trust
//! with a file from someone else.
//!
//! What the reader guards against: names and sizes pointing outside the
//! file, a CRC that does not match, and entries that share their bytes.
//! Stored entries cannot expand, so with no two overlapping, what comes
//! out is never more than the file itself — the zip-bomb trick of many
//! entries over one run of data does not get through.

use std::io::{self, Write};

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
const LOCAL_LEN: usize = 30;
const CENTRAL_LEN: usize = 46;
const END_LEN: usize = 22;
/// Version 2.0: stored entries in folders.
const VERSION: u16 = 20;
/// General-purpose flags: bit 0 encrypted, bit 11 names in UTF-8.
const ENCRYPTED: u16 = 1;
const UTF8: u16 = 1 << 11;
/// 1980-01-01, the format's first day: a scene file carries no dates.
const DATE: u16 = (1 << 5) | 1;

/// Writes a zip of stored entries to `out`.
pub struct Writer<W: Write> {
    out: W,
    offset: u64,
    central: Vec<u8>,
    count: u16,
}

impl<W: Write> Writer<W> {
    pub fn new(out: W) -> Self {
        Self {
            out,
            offset: 0,
            central: Vec::new(),
            count: 0,
        }
    }

    /// Add `data` under `name`.
    pub fn add(&mut self, name: &str, data: &[u8]) -> io::Result<()> {
        let too_big = || io::Error::other("the scene is too big for one file");
        let size = u32::try_from(data.len()).map_err(|_| too_big())?;
        let name_len = u16::try_from(name.len()).map_err(|_| too_big())?;
        let offset = u32::try_from(self.offset).map_err(|_| too_big())?;
        self.count = self.count.checked_add(1).ok_or_else(too_big)?;
        let crc = crc32(data);

        let mut local = Vec::with_capacity(LOCAL_LEN + name.len());
        put32(&mut local, LOCAL);
        for v in [VERSION, UTF8, 0, 0, DATE] {
            put16(&mut local, v);
        }
        for v in [crc, size, size] {
            put32(&mut local, v);
        }
        put16(&mut local, name_len);
        put16(&mut local, 0);
        local.extend_from_slice(name.as_bytes());
        self.out.write_all(&local)?;
        self.out.write_all(data)?;

        let c = &mut self.central;
        put32(c, CENTRAL);
        // Made by Unix, so the permissions below mean something.
        for v in [(3 << 8) | VERSION, VERSION, UTF8, 0, 0, DATE] {
            put16(c, v);
        }
        for v in [crc, size, size] {
            put32(c, v);
        }
        for v in [name_len, 0, 0, 0, 0] {
            put16(c, v);
        }
        put32(c, 0o100_644 << 16);
        put32(c, offset);
        c.extend_from_slice(name.as_bytes());

        self.offset += (local.len() + data.len()) as u64;
        Ok(())
    }

    /// Write the directory and hand back `out`.
    pub fn finish(mut self) -> io::Result<W> {
        let too_big = || io::Error::other("the scene is too big for one file");
        let offset = u32::try_from(self.offset).map_err(|_| too_big())?;
        let size = u32::try_from(self.central.len()).map_err(|_| too_big())?;
        self.out.write_all(&self.central)?;
        let mut end = Vec::with_capacity(END_LEN);
        put32(&mut end, END);
        for v in [0, 0, self.count, self.count] {
            put16(&mut end, v);
        }
        put32(&mut end, size);
        put32(&mut end, offset);
        put16(&mut end, 0);
        self.out.write_all(&end)?;
        Ok(self.out)
    }
}

/// One entry read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<'a> {
    pub name: &'a str,
    pub data: &'a [u8],
}

/// The entries of the zip in `bytes`, at most `max_entries` of them, or
/// why it is not one this reader takes.
pub fn read(bytes: &[u8], max_entries: usize) -> Result<Vec<Entry<'_>>, String> {
    let end = find_end(bytes).ok_or("not a zip file")?;
    let (disk, cd_disk) = (u16_at(bytes, end + 4)?, u16_at(bytes, end + 6)?);
    let (here, total) = (u16_at(bytes, end + 8)?, u16_at(bytes, end + 10)?);
    if disk != 0 || cd_disk != 0 || here != total {
        return Err("split across several files".into());
    }
    let total = usize::from(total);
    if total > max_entries {
        return Err(format!("{total} files inside; at most {max_entries}"));
    }
    let cd_size = u32_at(bytes, end + 12)? as usize;
    let cd_start = u32_at(bytes, end + 16)? as usize;
    if cd_start.checked_add(cd_size) != Some(end) {
        return Err("its directory is not where it says".into());
    }

    let mut entries = Vec::with_capacity(total);
    // (start of the local header, end of the data), to check for overlap.
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(total);
    let mut at = cd_start;
    for _ in 0..total {
        if u32_at(bytes, at)? != CENTRAL {
            return Err("damaged directory".into());
        }
        let flags = u16_at(bytes, at + 8)?;
        let method = u16_at(bytes, at + 10)?;
        let crc = u32_at(bytes, at + 16)?;
        let packed = u32_at(bytes, at + 20)? as usize;
        let size = u32_at(bytes, at + 24)? as usize;
        let name_len = usize::from(u16_at(bytes, at + 28)?);
        let extra_len = usize::from(u16_at(bytes, at + 30)?);
        let comment_len = usize::from(u16_at(bytes, at + 32)?);
        let local = u32_at(bytes, at + 42)? as usize;
        let name = slice(bytes, at + CENTRAL_LEN, name_len)?;
        at += CENTRAL_LEN + name_len + extra_len + comment_len;
        if at > end {
            return Err("damaged directory".into());
        }
        if flags & ENCRYPTED != 0 {
            return Err("encrypted".into());
        }
        if method != 0 || packed != size {
            return Err("compressed; only files as animaEngine writes them are read".into());
        }
        let name = std::str::from_utf8(name).map_err(|_| "a name that is not UTF-8")?;

        if u32_at(bytes, local)? != LOCAL {
            return Err(format!("damaged entry: {name}"));
        }
        let local_name_len = usize::from(u16_at(bytes, local + 26)?);
        let local_extra_len = usize::from(u16_at(bytes, local + 28)?);
        if slice(bytes, local + LOCAL_LEN, local_name_len)? != name.as_bytes() {
            return Err(format!("damaged entry: {name}"));
        }
        let start = local + LOCAL_LEN + local_name_len + local_extra_len;
        let data = slice(bytes, start, size)?;
        if start + size > cd_start {
            return Err(format!("damaged entry: {name}"));
        }
        if crc32(data) != crc {
            return Err(format!("damaged entry: {name}"));
        }
        if entries.iter().any(|e: &Entry<'_>| e.name == name) {
            return Err(format!("{name} is in it twice"));
        }
        spans.push((local, start + size));
        entries.push(Entry { name, data });
    }
    spans.sort_unstable();
    if spans.windows(2).any(|w| w[1].0 < w[0].1) {
        return Err("entries share their data".into());
    }
    Ok(entries)
}

/// Where the end record starts: looked for from the back, past a comment
/// of at most 64 KiB, and taken only if the comment runs exactly to the
/// end of the file.
fn find_end(bytes: &[u8]) -> Option<usize> {
    let last = bytes.len().checked_sub(END_LEN)?;
    let first = last.saturating_sub(usize::from(u16::MAX));
    (first..=last).rev().find(|&i| {
        u32_at(bytes, i).ok() == Some(END)
            && u16_at(bytes, i + 20).ok().map(usize::from) == Some(bytes.len() - i - END_LEN)
    })
}

fn slice(bytes: &[u8], at: usize, len: usize) -> Result<&[u8], String> {
    at.checked_add(len)
        .and_then(|end| bytes.get(at..end))
        .ok_or_else(|| "cut short".to_string())
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    let b = slice(bytes, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    let b = slice(bytes, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn put16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// CRC-32 (IEEE), as zip checks its entries.
pub fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut table = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            table[i] = c;
            i += 1;
        }
        table
    };
    !data.iter().fold(!0u32, |c, &b| {
        TABLE[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = Writer::new(Vec::new());
        for (name, data) in files {
            w.add(name, data).unwrap();
        }
        w.finish().unwrap()
    }

    #[test]
    fn crc_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn what_is_written_reads_back() {
        let bytes = zip_of(&[
            ("scene.toml", b"name = \"A\"\n"),
            ("assets/0/asset.png", &[1, 2, 3]),
        ]);
        let entries = read(&bytes, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "scene.toml");
        assert_eq!(entries[1].data, &[1, 2, 3]);
        assert!(read(&bytes, 1).is_err(), "more entries than allowed");
    }

    /// The system's own `unzip`, when there is one, agrees it is a zip.
    #[test]
    fn unzip_reads_it() {
        let Ok(out) = std::process::Command::new("unzip").arg("-v").output() else {
            return;
        };
        if !out.status.success() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("anima-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.zip");
        std::fs::write(&file, zip_of(&[("a/b.txt", b"hello")])).unwrap();
        let test = std::process::Command::new("unzip")
            .arg("-tq")
            .arg(&file)
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            test.status.success(),
            "{}",
            String::from_utf8_lossy(&test.stdout)
        );
    }

    #[test]
    fn damage_is_refused() {
        let good = zip_of(&[("a.png", b"abcdef")]);
        assert!(read(&good[..good.len() - 1], 10).is_err(), "cut short");
        let mut flipped = good.clone();
        flipped[LOCAL_LEN + 5 + 2] ^= 0xFF;
        assert!(read(&flipped, 10).unwrap_err().contains("damaged"), "CRC");
        assert!(read(b"not a zip at all, not even close", 10).is_err());
        assert!(read(&[], 10).is_err());
    }

    /// A second entry whose header and data sit inside the first's data:
    /// the zip-bomb shape, many names over one run of bytes.
    #[test]
    fn entries_sharing_data_are_refused() {
        let payload = b"0123456789";
        // What "b.png" alone looks like, header and data.
        let alone = zip_of(&[("b.png", payload)]);
        let inner = &alone[..LOCAL_LEN + 5 + payload.len()];
        let alone_end = find_end(&alone).unwrap();
        let alone_cd = u32_at(&alone, alone_end + 16).unwrap() as usize;
        let mut b_record = alone[alone_cd..alone_end].to_vec();

        // "a.png" carries it as its data; point b's record at it.
        let mut bytes = zip_of(&[("a.png", inner)]);
        let end = find_end(&bytes).unwrap();
        let b_offset = (LOCAL_LEN + 5) as u32;
        b_record[42..46].copy_from_slice(&b_offset.to_le_bytes());
        bytes.splice(end..end, b_record.iter().copied());
        let end = end + b_record.len();
        let count = 2u16.to_le_bytes();
        bytes[end + 8..end + 10].copy_from_slice(&count);
        bytes[end + 10..end + 12].copy_from_slice(&count);
        let cd_start = u32_at(&bytes, end + 16).unwrap() as usize;
        let cd_size = ((end - cd_start) as u32).to_le_bytes();
        bytes[end + 12..end + 16].copy_from_slice(&cd_size);

        assert_eq!(read(&bytes, 10).unwrap_err(), "entries share their data");
    }

    #[test]
    fn compressed_and_encrypted_entries_are_refused() {
        let good = zip_of(&[("a.png", b"abcdef")]);
        let end = find_end(&good).unwrap();
        let cd = u32_at(&good, end + 16).unwrap() as usize;
        let mut deflated = good.clone();
        deflated[cd + 10] = 8;
        assert!(read(&deflated, 10).unwrap_err().contains("compressed"));
        let mut locked = good.clone();
        locked[cd + 8] |= 1;
        assert!(read(&locked, 10).unwrap_err().contains("encrypted"));
    }
}
