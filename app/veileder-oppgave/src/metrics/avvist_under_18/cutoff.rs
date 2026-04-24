use chrono::{DateTime, TimeZone, Utc};

/// Data før denne datoen er upålitelig pga. en bug i AvvistUnder18-håndteringen.
pub fn cutoff_date() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap()
}
