//! The config's schema version and the migrations between versions.

use super::*;

/// Read the `version` key from a parsed config table.
///
/// - absent → `1` (every config written before 0.9 is schema v1);
/// - a positive integer → that version;
/// - anything else (string, float, negative, zero) → treated as
///   [`CURRENT_SCHEMA_VERSION`] with a warning. A malformed version
///   field must never be a reason to wipe a user's config — we'd
///   rather skip migration than run the wrong one.
pub(super) fn detect_schema_version(table: &toml::Table) -> u32 {
    match table.get("version") {
        None => 1,
        // Bounded conversion. A value that doesn't fit u32 (e.g.
        // `version = 4294967296`) must never wrap to a bogus low
        // version and then drive migration off a cliff — the old
        // `*n as u32` turned 2^32 into 0 and `migrate_table` hit
        // `unreachable!`, so a hand-typed number could *panic* the
        // loader. `try_from` rejects overflow and negatives; the
        // `>= 1` guard rejects zero. All of them fall through to the
        // "treat as current, skip migration" contract below.
        Some(toml::Value::Integer(n)) => match u32::try_from(*n) {
            Ok(v) if v >= 1 => v,
            _ => {
                tracing::warn!(
                    "config `version` = {n} is out of range; treating as current ({CURRENT_SCHEMA_VERSION}) and skipping migration"
                );
                CURRENT_SCHEMA_VERSION
            }
        },
        Some(other) => {
            tracing::warn!(
                "config `version` is malformed ({other:?}); treating as current ({CURRENT_SCHEMA_VERSION}) and skipping migration"
            );
            CURRENT_SCHEMA_VERSION
        }
    }
}

/// Bring a parsed config table up to [`CURRENT_SCHEMA_VERSION`] in
/// place, then stamp the version. Pure: no IO, no deserialisation.
/// Returns the version it started from (for backup naming / logging).
///
/// The chain is ordered and idempotent: re-running it on an
/// already-current table is a no-op apart from re-stamping the same
/// version.
pub(super) fn migrate_table(table: &mut toml::Table) -> u32 {
    let from = detect_schema_version(table);
    if from > CURRENT_SCHEMA_VERSION {
        // Config written by a newer build than this one. There is no
        // downgrade path, and re-stamping it to CURRENT would silently
        // strip fields this build can't model. Leave the table — version
        // marker and all — untouched and let `load` deserialise what it
        // understands without persisting, so the richer on-disk file is
        // never downgraded just by being opened by an older binary.
        tracing::warn!(
            "config schema v{from} is newer than this build (v{CURRENT_SCHEMA_VERSION}); \
             loading read-only and leaving the file untouched"
        );
        return from;
    }
    let mut v = from;
    while v < CURRENT_SCHEMA_VERSION {
        match v {
            1 => migrate_v1_v2(table),
            // Unreachable while CURRENT_SCHEMA_VERSION == 2; the arm is
            // here so adding a v2→v3 migration is a one-line change and
            // a missing arm is a loud panic, not a silent skip.
            other => unreachable!("no migration registered for schema v{other}"),
        }
        v += 1;
    }
    table.insert(
        "version".to_string(),
        toml::Value::Integer(CURRENT_SCHEMA_VERSION as i64),
    );
    from
}

/// v1 → v2: structurally identical. Every config change from 0.2
/// through 0.8 was additive and is handled by serde `default`s, so
/// there is nothing to rewrite — this migration only exists to stamp
/// the version and to prove the chain runs end to end. The first
/// genuinely non-additive change (a renamed/removed/retyped key) lands
/// here as real rewrite logic.
pub(super) fn migrate_v1_v2(_table: &mut toml::Table) {}
