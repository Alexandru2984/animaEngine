//! What `mp4` 0.14 cannot be trusted with, checked before it is asked.
//!
//! The crate parses a container on the faith of its own numbers. Against a
//! file the user merely dropped, that faith hangs, panics or allocates
//! (SEC-003):
//!
//! - inside a container, a child box of size 0 makes its loop skip zero
//!   bytes and read the same header again, forever (the top level alone
//!   stops on it); a size below 8 walks it backwards;
//! - an iTunes `data` box shorter than its own fixed fields underflows
//!   `start + size - current`, and an `emsg` box whose strings outgrow it
//!   underflows `size - …` into an allocation of nearly 2⁶⁴ bytes;
//! - `track.sample_count()` sums fragment sample counts with `expect`;
//! - `read_sample` allocates the sample's declared size before reading it,
//!   then unwraps its time from `stts`, and computes a fragmented sample's
//!   time as `(id - 1) × duration` in u32 — and sums a chunk's sample sizes
//!   in u32 too.
//!
//! [`check_box_tree`] walks the boxes the crate will walk, before it does;
//! [`checked_samples`] checks a parsed track's tables, before a sample is
//! read, and says how many samples can be read safely. Swapping the
//! demuxer would be the other way out; these checks keep the crate, whose
//! behaviour on valid files is known, and add no dependency.

use crate::error::{AnimaError, Result};
use std::io::{Read, Seek, SeekFrom};

/// Boxes `mp4` 0.14 reads as a list of child boxes.
const CONTAINERS: [[u8; 4]; 11] = [
    *b"moov", *b"trak", *b"edts", *b"mdia", *b"minf", *b"dinf", *b"stbl", *b"udta", *b"mvex",
    *b"moof", *b"traf",
];

/// The iTunes metadata items `mp4` 0.14 reads (title, year, poster,
/// summary): lists of boxes, a `data` box among them.
const ILST_ITEMS: [[u8; 4]; 4] = [
    [0xA9, b'n', b'a', b'm'],
    [0xA9, b'd', b'a', b'y'],
    *b"covr",
    *b"desc",
];

/// Deeper than any real file nests (`moov/udta/meta/ilst/©nam/data` is 6).
const MAX_DEPTH: u32 = 16;

/// More boxes than a real clip of the length we decode has — a fragmented
/// hour is some 20 000 — and few enough to walk in milliseconds.
const MAX_BOXES: u32 = 100_000;

/// A `data` box's fixed fields after its header: type and locale.
const DATA_FIELDS: u64 = 8;

/// What the children of a box are, for the walk.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    /// The file itself: a box of size 0 runs to its end, where the crate
    /// stops reading.
    Top,
    /// A list of boxes, as the crate reads it.
    Children,
    /// `ilst`: items, some of which the crate opens.
    Items,
    /// An item the crate opens: its `data` box is read.
    Item,
}

fn refused(why: &str) -> AnimaError {
    AnimaError::VideoDecode(format!("malformed MP4 container: {why}"))
}

/// Walk the boxes `mp4` 0.14 will walk in the `size` bytes `r` holds,
/// refusing the shapes that hang, underflow or allocate in it.
pub fn check_box_tree<R: Read + Seek>(r: &mut R, size: u64) -> Result<()> {
    let mut boxes = 0;
    walk(r, 0, size, Level::Top, 0, &mut boxes)?;
    r.seek(SeekFrom::Start(0))?;
    Ok(())
}

fn walk<R: Read + Seek>(
    r: &mut R,
    start: u64,
    end: u64,
    level: Level,
    depth: u32,
    boxes: &mut u32,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(refused("boxes nested too deep"));
    }
    let mut at = start;
    while at < end {
        if end - at < 8 {
            return Err(refused("a box header runs past its parent"));
        }
        *boxes += 1;
        if *boxes > MAX_BOXES {
            return Err(refused("too many boxes"));
        }
        r.seek(SeekFrom::Start(at))?;
        let mut header = [0u8; 8];
        r.read_exact(&mut header)?;
        let declared = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
        let kind = [header[4], header[5], header[6], header[7]];
        let (len, header_len) = match declared {
            // To the end of the file: the crate stops reading there, at
            // the top level; inside a container it loops forever.
            0 if level == Level::Top => return Ok(()),
            0 => return Err(refused("a box of size zero inside a container")),
            1 => {
                if end - at < 16 {
                    return Err(refused("a 64-bit box header runs past its parent"));
                }
                let mut large = [0u8; 8];
                r.read_exact(&mut large)?;
                let len = u64::from_be_bytes(large);
                if len < 16 {
                    return Err(refused("a 64-bit box size smaller than its header"));
                }
                (len, 16)
            }
            n if n < 8 => return Err(refused("a box size smaller than its header")),
            n => (u64::from(n), 8),
        };
        let box_end = at
            .checked_add(len)
            .filter(|&e| e <= end)
            .ok_or_else(|| refused("a box runs past its parent"))?;
        let body = at + header_len;

        match (&kind, level) {
            (b"emsg", _) => return Err(refused("event message boxes are not supported")),
            (k, _) if CONTAINERS.contains(k) => {
                walk(r, body, box_end, Level::Children, depth + 1, boxes)?;
            }
            (b"meta", _) => {
                // A full box, or QuickTime's without the version and
                // flags — the crate tells them apart the same way.
                let children = if box_end - body >= 4 {
                    r.seek(SeekFrom::Start(body))?;
                    let mut first = [0u8; 4];
                    r.read_exact(&mut first)?;
                    if first == [0; 4] {
                        body + 4
                    } else {
                        body
                    }
                } else {
                    body
                };
                walk(r, children, box_end, Level::Children, depth + 1, boxes)?;
            }
            (b"ilst", _) => walk(r, body, box_end, Level::Items, depth + 1, boxes)?,
            (k, Level::Items) if ILST_ITEMS.contains(k) => {
                walk(r, body, box_end, Level::Item, depth + 1, boxes)?;
            }
            (b"data", Level::Item) if len < header_len + DATA_FIELDS => {
                return Err(refused("a metadata box shorter than its fields"));
            }
            _ => {}
        }
        at = box_end;
    }
    Ok(())
}

/// How many of `track`'s samples, from the first, can be read from a file
/// of `file_size` bytes without `mp4` 0.14 overflowing, panicking or
/// allocating more than the file holds — at most `limit`. Errors when the
/// track's tables cannot be trusted at all.
pub fn checked_samples(track: &mp4::Mp4Track, file_size: u64, limit: u32) -> Result<u32> {
    let mut count = if track.trafs.is_empty() {
        track.trak.mdia.minf.stbl.stsz.sample_count
    } else {
        // What `sample_count()` sums with `expect`.
        track
            .trafs
            .iter()
            .filter_map(|traf| traf.trun.as_ref())
            .try_fold(0u32, |sum, trun| sum.checked_add(trun.sample_count))
            .ok_or_else(|| refused("fragment sample counts overflow"))?
    };
    count = count.min(limit);

    if track.trafs.is_empty() {
        // `read_sample` unwraps the sample's time from `stts`: read only
        // the samples it covers, adding as the crate adds.
        let mut covered: u32 = 0;
        let mut elapsed: u64 = 0;
        for entry in &track.trak.mdia.minf.stbl.stts.entries {
            if covered >= count {
                break;
            }
            covered = covered
                .checked_add(entry.sample_count)
                .ok_or_else(|| refused("time table sample counts overflow"))?;
            elapsed = u64::from(entry.sample_count)
                .checked_mul(u64::from(entry.sample_delta))
                .and_then(|span| elapsed.checked_add(span))
                .ok_or_else(|| refused("time table durations overflow"))?;
        }
        count = count.min(covered);

        // Each sample is allocated at its declared size, and a chunk's
        // sizes are summed in u32: all of them together must fit in the
        // file, as the samples of a real file do.
        let stsz = &track.trak.mdia.minf.stbl.stsz;
        let total = if stsz.sample_size > 0 {
            u64::from(stsz.sample_size) * u64::from(count)
        } else {
            stsz.sample_sizes
                .iter()
                .take(count as usize)
                .map(|&n| u64::from(n))
                .sum()
        };
        if total > file_size {
            return Err(refused("sample sizes add up to more than the file"));
        }
    } else {
        // A fragment's sample time is `(id - 1) × duration` in u32.
        if count > 1
            && (count - 1)
                .checked_mul(track.default_sample_duration)
                .is_none()
        {
            return Err(refused("fragment sample times overflow"));
        }
        let total: u64 = track
            .trafs
            .iter()
            .filter_map(|traf| traf.trun.as_ref())
            .flat_map(|trun| trun.sample_sizes.iter())
            .map(|&n| u64::from(n))
            .sum();
        if total > file_size {
            return Err(refused("sample sizes add up to more than the file"));
        }
    }

    if count == 0 {
        return Err(refused("no sample has a time"));
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn mp4_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut b = (8 + payload.len() as u32).to_be_bytes().to_vec();
        b.extend_from_slice(kind);
        b.extend_from_slice(payload);
        b
    }

    fn check(bytes: &[u8]) -> Result<()> {
        check_box_tree(&mut Cursor::new(bytes), bytes.len() as u64)
    }

    fn ftyp() -> Vec<u8> {
        mp4_box(b"ftyp", b"isom\0\0\0\0")
    }

    #[test]
    fn a_well_formed_tree_passes_and_the_reader_is_rewound() {
        let mut bytes = ftyp();
        let ilst = mp4_box(
            b"ilst",
            &mp4_box(
                &[0xA9, b'n', b'a', b'm'],
                &mp4_box(b"data", &[0, 0, 0, 1, 0, 0, 0, 0, b'x']),
            ),
        );
        let meta = mp4_box(b"meta", &[vec![0, 0, 0, 0], ilst].concat());
        bytes.extend(mp4_box(b"moov", &mp4_box(b"udta", &meta)));
        let mut cursor = Cursor::new(&bytes);
        check_box_tree(&mut cursor, bytes.len() as u64).expect("well formed");
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    fn a_zero_sized_box_ends_the_file_at_the_top_only() {
        let mut top = ftyp();
        top.extend([0, 0, 0, 0, b'm', b'd', b'a', b't', 1, 2, 3]);
        assert!(check(&top).is_ok(), "size 0 at the top runs to the end");
        let mut inner = ftyp();
        inner.extend(mp4_box(b"moov", &[0, 0, 0, 0, b'f', b'r', b'e', b'e']));
        assert!(check(&inner).is_err());
    }

    #[test]
    fn sizes_below_a_header_or_past_the_parent_are_refused() {
        let mut small = ftyp();
        small.extend(mp4_box(b"moov", &[0, 0, 0, 4, b'f', b'r', b'e', b'e']));
        assert!(check(&small).is_err());
        let mut past = ftyp();
        past.extend(mp4_box(b"moov", &[0, 0, 0, 64, b'f', b'r', b'e', b'e']));
        assert!(check(&past).is_err());
        let mut large = ftyp();
        // 64-bit size of 12: smaller than the 16-byte header it is in.
        large.extend([0, 0, 0, 1, b'f', b'r', b'e', b'e', 0, 0, 0, 0, 0, 0, 0, 12]);
        assert!(check(&large).is_err());
    }

    #[test]
    fn a_short_metadata_box_and_event_messages_are_refused() {
        let short_data = mp4_box(b"data", &[0, 0, 0, 1]);
        let ilst = mp4_box(b"ilst", &mp4_box(&[0xA9, b'n', b'a', b'm'], &short_data));
        let meta = mp4_box(b"meta", &[vec![0, 0, 0, 0], ilst].concat());
        let mut bytes = ftyp();
        bytes.extend(mp4_box(b"moov", &mp4_box(b"udta", &meta)));
        assert!(check(&bytes).is_err());

        let mut emsg = ftyp();
        emsg.extend(mp4_box(b"emsg", &[0; 16]));
        assert!(check(&emsg).is_err());
    }

    #[test]
    fn nesting_and_box_counts_are_bounded() {
        let mut nested = mp4_box(b"free", &[]);
        for _ in 0..=MAX_DEPTH + 1 {
            nested = mp4_box(b"moov", &nested);
        }
        let mut bytes = ftyp();
        bytes.extend(nested);
        assert!(check(&bytes).is_err());

        let mut many = ftyp();
        for _ in 0..=MAX_BOXES {
            many.extend(mp4_box(b"free", &[]));
        }
        assert!(check(&many).is_err());
    }
}
