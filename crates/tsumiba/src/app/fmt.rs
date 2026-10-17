//! Small display formatters: how long ago, how big.

use crate::i18n::Lang;

const MINUTE: u64 = 60_000;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// "now", "5m", "3h", "2d", and a date after a week.
pub fn relative_time(now_ms: u64, then_ms: u64, lang: Lang) -> String {
    let age = now_ms.saturating_sub(then_ms);
    match (age, lang) {
        (a, Lang::En) if a < 45_000 => "now".into(),
        (a, Lang::Ja) if a < 45_000 => "たった今".into(),
        (a, Lang::En) if a < HOUR => format!("{}m", (a / MINUTE).max(1)),
        (a, Lang::Ja) if a < HOUR => format!("{}分前", (a / MINUTE).max(1)),
        (a, Lang::En) if a < DAY => format!("{}h", a / HOUR),
        (a, Lang::Ja) if a < DAY => format!("{}時間前", a / HOUR),
        (a, Lang::En) if a < 7 * DAY => format!("{}d", a / DAY),
        (a, Lang::Ja) if a < 7 * DAY => format!("{}日前", a / DAY),
        _ => date(then_ms),
    }
}

/// `YYYY-MM-DD` in the local time zone.
pub fn date(ms: u64) -> String {
    chrono::DateTime::from_timestamp_millis(ms as i64)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default()
}

/// `512 B`, `1.4 KB`, `38 MB`: binary multiples, one decimal under 10.
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value < 10.0 {
        format!("{value:.1} {}", UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_800_000_000_000;

    #[test]
    fn recent_times_are_short() {
        assert_eq!(relative_time(NOW, NOW, Lang::En), "now");
        assert_eq!(relative_time(NOW, NOW - 44_000, Lang::En), "now");
        assert_eq!(relative_time(NOW, NOW - 50_000, Lang::En), "1m");
        assert_eq!(relative_time(NOW, NOW - 5 * MINUTE, Lang::En), "5m");
        assert_eq!(relative_time(NOW, NOW - 59 * MINUTE, Lang::En), "59m");
        assert_eq!(relative_time(NOW, NOW - 3 * HOUR, Lang::En), "3h");
        assert_eq!(relative_time(NOW, NOW - 2 * DAY, Lang::En), "2d");
    }

    #[test]
    fn japanese_uses_japanese_units() {
        assert_eq!(relative_time(NOW, NOW, Lang::Ja), "たった今");
        assert_eq!(relative_time(NOW, NOW - 5 * MINUTE, Lang::Ja), "5分前");
        assert_eq!(relative_time(NOW, NOW - 3 * HOUR, Lang::Ja), "3時間前");
        assert_eq!(relative_time(NOW, NOW - 2 * DAY, Lang::Ja), "2日前");
    }

    #[test]
    fn old_times_become_dates() {
        let s = relative_time(NOW, NOW - 30 * DAY, Lang::En);
        assert_eq!(s.len(), 10, "{s}");
        assert_eq!(&s[4..5], "-");
    }

    #[test]
    fn future_times_do_not_underflow() {
        assert_eq!(relative_time(NOW, NOW + 10 * DAY, Lang::En), "now");
    }

    #[test]
    fn sizes() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1024), "1.0 KB");
        assert_eq!(size(1536), "1.5 KB");
        assert_eq!(size(10 * 1024), "10 KB");
        assert_eq!(size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(size(3 * 1024 * 1024 * 1024), "3.0 GB");
        assert!(size(u64::MAX).ends_with("TB"));
    }
}
