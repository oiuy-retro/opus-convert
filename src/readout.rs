//! The same readouts as Folder Difference Copy: speed, items per second, files left, time left, elapsed.

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        if bytes == 1 {
            return "1 byte".into();
        }
        return format!("{bytes} bytes");
    }
    format_scaled(bytes as f64)
}

pub fn format_rate(bytes_per_sec: f64) -> String {
    let bytes_per_sec = if bytes_per_sec.is_finite() && bytes_per_sec > 0.0 {
        bytes_per_sec
    } else {
        0.0
    };
    if bytes_per_sec < 1024.0 {
        return format!("{} B/s", bytes_per_sec.round() as u64);
    }
    format!("{}/s", format_scaled(bytes_per_sec))
}

fn format_scaled(bytes: f64) -> String {
    let units = ["KB", "MB", "GB", "TB"];
    let mut value = bytes;
    let mut unit = 0;
    if value >= 1024.0 {
        value /= 1024.0;
    }
    while value >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }
    let number = if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };
    format!("{number} {}", units[unit])
}

pub fn format_items(per_second: f64) -> String {
    let per_second = if per_second.is_finite() && per_second > 0.0 {
        per_second
    } else {
        0.0
    };
    if per_second >= 100.0 {
        format!("{per_second:.0}")
    } else if per_second >= 10.0 {
        format!("{per_second:.1}")
    } else {
        format!("{per_second:.2}")
    }
}

pub fn format_eta(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "Calculating...".into();
    }
    if seconds < 4.0 {
        return "A few seconds".into();
    }
    if seconds < 60.0 {
        return format!("{} seconds", seconds.round() as i64);
    }
    if seconds < 3600.0 {
        let mut minutes = (seconds / 60.0).round() as i64;
        if minutes < 1 {
            minutes = 1;
        }
        if minutes == 1 {
            return "About 1 minute".into();
        }
        return format!("About {minutes} minutes");
    }
    let mut hours = (seconds / 3600.0) as i64;
    let mut mins = ((seconds - hours as f64 * 3600.0) / 60.0).round() as i64;
    if mins >= 60 {
        hours += 1;
        mins = 0;
    }
    if mins <= 0 {
        if hours == 1 {
            return "About 1 hour".into();
        }
        return format!("About {hours} hours");
    }
    format!("About {hours} h {mins} min")
}

pub fn format_elapsed(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours >= 1 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

pub fn percent(ratio: f64) -> u32 {
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if ratio >= 1.0 {
        return 100;
    }
    let pct = (ratio * 100.0).round() as u32;
    if pct >= 100 {
        99
    } else {
        pct
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readouts_match_the_copy_program() {
        assert_eq!(format_bytes(1), "1 byte");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_rate(2048.0), "2.00 KB/s");
        assert_eq!(format_items(1.2), "1.20");
        assert_eq!(format_eta(2.0), "A few seconds");
        assert_eq!(format_eta(90.0), "About 2 minutes");
        assert_eq!(format_elapsed(75), "1:15");
        assert_eq!(format_elapsed(3661), "1:01:01");
        assert_eq!(percent(0.999), 99);
        assert_eq!(percent(1.0), 100);
    }
}
