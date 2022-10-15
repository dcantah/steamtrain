use unicode_width::UnicodeWidthStr;

/// Formats minutes as "Xh Ym" or "Xm" for short durations.
pub fn format_playtime(minutes: u64) -> String {
    if minutes == 0 {
        return "-".to_string();
    }
    let hours = minutes / 60;
    let mins = minutes % 60;
    if hours == 0 {
        format!("{}m", mins)
    } else if mins == 0 {
        format!("{}h", hours)
    } else {
        format!("{}h {}m", hours, mins)
    }
}

pub fn human_bytes(n: i64) -> String {
    const UNIT: i64 = 1024;
    if n < UNIT {
        return format!("{} B", n);
    }

    let mut div = UNIT;
    let mut exp = 0;
    let mut m = n / UNIT;
    while m >= UNIT {
        m /= UNIT;
        div *= UNIT;
        exp += 1;
    }

    let suffix = ["K", "M", "G", "T", "P", "E"];
    format!("{:.1} {}iB", n as f64 / div as f64, suffix[exp])
}

#[allow(dead_code)]
pub fn truncate_width(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if s.width() <= width {
        return s.to_string();
    }

    let mut result = String::new();
    let mut current_width = 0;

    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > width {
            break;
        }
        result.push(c);
        current_width += char_width;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_bytes() {
        assert_eq!(human_bytes(500), "500 B");
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(1048576), "1.0 MiB");
        assert_eq!(human_bytes(1073741824), "1.0 GiB");
    }

    #[test]
    fn test_truncate_width() {
        assert_eq!(truncate_width("hello", 10), "hello");
        assert_eq!(truncate_width("hello world", 5), "hello");
        assert_eq!(truncate_width("hello", 0), "");
    }

    #[test]
    fn test_format_playtime() {
        assert_eq!(format_playtime(0), "-");
        assert_eq!(format_playtime(30), "30m");
        assert_eq!(format_playtime(60), "1h");
        assert_eq!(format_playtime(90), "1h 30m");
        assert_eq!(format_playtime(150), "2h 30m");
    }
}
