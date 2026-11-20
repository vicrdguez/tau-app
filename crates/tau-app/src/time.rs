//! Times and durations in the copy's words (`spec.md` §8 "Ages and times"): clock times in local
//! time (`16:41`, `Nov 14 16:41` on another day), and one way to say how long (`4.2 s`, `12 s`,
//! `17 m`, `1 h 5 m`, `4 d 15 h`, `12 d`).
//!
//! How it could fail:
//! - a clock time in UTC, or with seconds, or without its leading zero (`9:05`);
//! - a duration under 10 s without its decimal, or a minute printed as `60 s`;
//! - an hour printed as `60 m`, or an hour and a half as `1.5 h`;
//! - a negative span (a clock that went back) printed with a minus sign;
//! - an age past a day printed in hours (`111 h 34 m`), or a day printed as `24 h`;
//! - a span of a week or more printed with hours (`12 d 3 h`);
//! - a clock time from another day printed without its date, so it reads as today.

use std::time::Duration;

use tau_client::Wall;

/// `16:41`: the wall time in this machine's time zone (`TZ` when set).
pub fn clock(at: Wall) -> String {
    let tm = local(at);
    format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
}

/// `16:41` on the day of `now`, `Nov 14 16:41` on any other day.
pub fn stamp(at: Wall, now: Wall) -> String {
    let (date, today) = (local(at), local(now));
    if date.tm_year == today.tm_year && date.tm_yday == today.tm_yday {
        return clock(at);
    }
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][date.tm_mon as usize];
    format!("{month} {} {}", date.tm_mday, clock(at))
}

fn local(at: Wall) -> libc::tm {
    let secs = at.0.div_euclid(1000) as libc::time_t;
    // SAFETY: `tm` is plain data that `localtime_r` fills; `secs` outlives the call.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&secs, &mut tm);
        tm
    }
}

/// How long, as a running count or an age: one decimal under 10 s, whole seconds under a minute,
/// minutes under an hour, hours and minutes under a day, days and hours under a week, then days.
pub fn span(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 10_000 {
        return format!("{}.{} s", ms / 1000, ms % 1000 / 100);
    }
    let s = d.as_secs();
    match s {
        0..60 => format!("{s} s"),
        60..3600 => format!("{} m", s / 60),
        3600..86_400 => format!("{} h {} m", s / 3600, s % 3600 / 60),
        86_400..604_800 => format!("{} d {} h", s / 86_400, s % 86_400 / 3600),
        _ => format!("{} d", s / 86_400),
    }
}

/// The span from `from` to `to`, zero when `to` is earlier.
pub fn between(from: Wall, to: Wall) -> Duration {
    Duration::from_millis(to.0.saturating_sub(from.0).max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_read_as_the_copy_says() {
        let s = |ms| span(Duration::from_millis(ms));
        assert_eq!(s(4_230), "4.2 s");
        assert_eq!(s(0), "0.0 s");
        assert_eq!(s(12_900), "12 s");
        assert_eq!(s(59_999), "59 s");
        assert_eq!(s(60_000), "1 m");
        assert_eq!(s(17 * 60_000 + 5_000), "17 m");
        assert_eq!(s(3_600_000), "1 h 0 m");
        assert_eq!(s(3_900_000), "1 h 5 m");
        assert_eq!(s((2 * 3600 + 52 * 60) * 1000), "2 h 52 m");
    }

    #[test]
    fn spans_past_a_day_read_in_days() {
        let h = |h: u64, m: u64| span(Duration::from_secs(h * 3600 + m * 60));
        assert_eq!(h(23, 59), "23 h 59 m");
        assert_eq!(h(24, 0), "1 d 0 h");
        assert_eq!(h(24, 59), "1 d 0 h");
        assert_eq!(h(4 * 24 + 15, 34), "4 d 15 h");
        assert_eq!(h(6 * 24 + 23, 59), "6 d 23 h");
        assert_eq!(h(7 * 24, 0), "7 d");
        assert_eq!(h(12 * 24 + 23, 0), "12 d");
    }

    #[test]
    fn a_stamp_carries_its_date_only_on_another_day() {
        let now = Wall(1_794_674_400_000);
        assert_eq!(stamp(now, now), clock(now));
        let old = Wall(now.0 - 5 * 86_400_000);
        let s = stamp(old, now);
        let (date, time) = s.rsplit_once(' ').unwrap();
        assert_eq!(time, clock(old));
        let (month, day) = date.split_once(' ').unwrap();
        assert!(
            [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
            ]
            .contains(&month),
            "{s}"
        );
        assert!(
            day.parse::<u8>().is_ok_and(|d| (1..=31).contains(&d)),
            "{s}"
        );
    }

    #[test]
    fn a_clock_that_went_back_is_no_time() {
        assert_eq!(between(Wall(10_000), Wall(4_000)), Duration::ZERO);
        assert_eq!(between(Wall(4_000), Wall(10_000)), Duration::from_secs(6));
    }

    #[test]
    fn clock_times_are_hours_and_minutes_with_zeros() {
        let t = clock(Wall(1_794_674_400_000));
        assert_eq!(t.len(), 5, "{t}");
        assert_eq!(&t[2..3], ":");
        assert!(t.chars().filter(char::is_ascii_digit).count() == 4, "{t}");
    }
}
