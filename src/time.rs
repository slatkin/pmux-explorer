//! System-clock helpers shared across hook dispatch, TUI refresh, and the
//! desktop notification dedup store. Each caller previously hand-rolled
//! the same `SystemTime::now().duration_since(UNIX_EPOCH)` incantation,
//! which meant every new call site had to reinvent the clock-skew
//! fallback. Centralising here keeps the fallback uniform (`0` on
//! skew) and makes the "we depend on the wall clock" surface grep-able.

use std::time::{SystemTime, UNIX_EPOCH};

/// Wall-clock seconds since the Unix epoch, with `0` as the fallback
/// when the clock is earlier than the epoch (practically impossible,
/// but the `Result` shape forces us to handle it).
pub fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Wall-clock milliseconds since the Unix epoch, same fallback as
/// [`now_epoch_secs`]. Used for the per-run notification id so that two
/// rapid SessionStart events on the same pane get distinct ids.
pub fn now_epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The `HH:mm:ss` local time of an ISO-8601 UTC timestamp — the `ts`
/// the thinking feed's records carry (`2026-10-27T08:00:07Z`, an
/// optional fractional part allowed). `None` when the value does not
/// parse as a UTC timestamp; the Thought tab draws no time line for it.
pub fn local_hhmmss_from_ts(ts: &str) -> Option<String> {
    let secs = parse_iso_utc_secs(ts)?;
    let t = secs as libc::time_t;
    // SAFETY: `zeroed` is the valid initial state for `libc::tm` — a plain
    // C struct of integers — and every field `localtime_r` does not write
    // stays zero.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: `localtime_r` reads `t` and writes only `tm`; both are valid
    // for the call. It reports failure by returning null, checked here.
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return None;
    }
    Some(format!(
        "{:02}:{:02}:{:02}",
        tm.tm_hour, tm.tm_min, tm.tm_sec
    ))
}

/// The `HH:mm:ss` of an ISO-8601 UTC timestamp in UTC itself — the
/// fixed-zone variant of [`local_hhmmss_from_ts`] and the seam snapshot
/// tests fix instead of touching the process timezone. `None` on the
/// same unparsable input.
pub fn utc_hhmmss_from_ts(ts: &str) -> Option<String> {
    let secs = parse_iso_utc_secs(ts)?;
    let day = secs.rem_euclid(86_400);
    Some(format!(
        "{:02}:{:02}:{:02}",
        day / 3600,
        day % 3600 / 60,
        day % 60
    ))
}

/// Seconds since the Unix epoch of an ISO-8601 UTC timestamp:
/// `YYYY-MM-DD[T ]HH:MM:SS`, an optional `.fff…` fraction, then `Z` —
/// the shape `Date.toISOString()` (with the fraction) and the spec
/// examples (without it) both write. A missing `Z` still reads as UTC;
/// any other trailing text, a wrong separator or a non-UTC offset is
/// unparsable.
fn parse_iso_utc_secs(ts: &str) -> Option<i64> {
    let b = ts.as_bytes();
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let field = ts.get(range)?;
        field
            .bytes()
            .all(|c| c.is_ascii_digit())
            .then_some(())
            .and_then(|()| field.parse().ok())
    };
    let year = digits(0..4)?;
    if b.get(4) != Some(&b'-') {
        return None;
    }
    let month = digits(5..7)?;
    if b.get(7) != Some(&b'-') {
        return None;
    }
    let day = digits(8..10)?;
    if !matches!(b.get(10), Some(b'T') | Some(b' ')) {
        return None;
    }
    let hour = digits(11..13)?;
    if b.get(13) != Some(&b':') {
        return None;
    }
    let min = digits(14..16)?;
    if b.get(16) != Some(&b':') {
        return None;
    }
    let sec = digits(17..19)?;
    let rest = &ts[19..];
    let rest = rest.strip_prefix('.').map_or(rest, |frac| {
        let end = frac
            .bytes()
            .position(|c| !c.is_ascii_digit())
            .unwrap_or(frac.len());
        &frac[end..]
    });
    if !rest.is_empty() && rest != "Z" {
        return None;
    }
    if !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || min > 59
        || sec > 59
    {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + min * 60 + sec)
}

/// Days from 1970-01-01 to a civil date (Howard Hinnant's
/// `days_from_civil`), valid for the whole `i64` year range.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let mp = if month > 2 { month - 3 } else { month + 9 }; // [0, 11]
    let doy = (153 * mp + 2) / 5 + day - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Days in one month of one year, leap Februaries included.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        // A Gregorian leap year: divisible by 4, except centuries not
        // divisible by 400.
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secs_and_millis_reflect_the_same_clock() {
        let secs = now_epoch_secs();
        let ms = now_epoch_millis();
        // Both read the wall clock in quick succession; ms/1000 must sit
        // within a couple of seconds of the secs reading, otherwise one
        // of them is reading a different source.
        assert!(
            ms / 1000 >= secs && ms / 1000 <= secs + 2,
            "secs={secs}, ms={ms}"
        );
    }

    // ─── local_hhmmss_from_ts ──────────────────────────────────

    #[test]
    fn an_iso_utc_ts_parses_to_its_epoch() {
        assert_eq!(
            parse_iso_utc_secs("2026-10-27T08:00:07Z"),
            Some(1_793_088_007)
        );
        // The fraction `Date.toISOString()` writes changes nothing.
        assert_eq!(
            parse_iso_utc_secs("2026-10-27T08:00:07.123Z"),
            Some(1_793_088_007)
        );
        // A missing `Z` still reads as UTC.
        assert_eq!(
            parse_iso_utc_secs("2026-10-27T08:00:07"),
            Some(1_793_088_007)
        );
        // A space separator is accepted beside `T`.
        assert_eq!(
            parse_iso_utc_secs("2026-10-27 08:00:07Z"),
            Some(1_793_088_007)
        );
        assert_eq!(parse_iso_utc_secs("1970-01-01T00:00:00Z"), Some(0));
        // A leap day lands on the right day.
        assert_eq!(
            parse_iso_utc_secs("2000-02-29T23:59:59Z"),
            Some(951_868_799)
        );
    }

    #[test]
    fn an_unparsable_ts_gives_none() {
        for ts in [
            "",
            "t1",
            "2026-10-27",
            "2026-10-27T08:00Z",
            "2026-10-27T08:00:07+02:00",
            "2026-13-27T08:00:07Z",
            "2026-10-32T08:00:07Z",
            "2026/10/27T08:00:07Z",
            "not a timestamp at all",
        ] {
            assert!(parse_iso_utc_secs(ts).is_none(), "{ts:?} must not parse");
        }
    }

    #[test]
    fn an_out_of_range_time_of_day_gives_none() {
        for ts in [
            "2026-10-27T24:00:00Z", // hour 24 (no leap-second handling)
            "2026-10-27T25:99:99Z", // the corrupt shape the review found
            "2026-10-27T23:60:00Z", // minute 60
            "2026-10-27T23:59:60Z", // second 60
        ] {
            assert!(parse_iso_utc_secs(ts).is_none(), "{ts:?} must not parse");
        }
    }

    #[test]
    fn a_day_beyond_the_month_gives_none() {
        for ts in [
            "2026-02-31T08:00:07Z", // February, non-leap year
            "2026-02-29T08:00:07Z", // February 29, non-leap year
            "2026-04-31T08:00:07Z", // April has 30 days
        ] {
            assert!(parse_iso_utc_secs(ts).is_none(), "{ts:?} must not parse");
        }
        // The leap day a leap year actually has; the century rule too.
        assert!(parse_iso_utc_secs("2024-02-29T08:00:07Z").is_some());
        assert!(parse_iso_utc_secs("2000-02-29T00:00:00Z").is_some());
        assert!(parse_iso_utc_secs("1900-02-29T00:00:00Z").is_none());
    }
}
