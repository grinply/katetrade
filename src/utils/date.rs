use chrono::{NaiveDate, NaiveTime};

/// Convert a number in the format yyyyMMdd into a NaiveDate
pub fn convert_to_naive_date(date: u32) -> Result<NaiveDate, anyhow::Error> {
    if date < 1000_00_00 || date > 9999_99_99 {
        anyhow::bail!("invalid date, the provided value doest contain 8 digits, it must be in the format yyyyMMdd");
    }

    let year = date / 10000;
    let month = (date / 100) - (year * 100);
    let day = date % 100;

    if year < 1000 || year >= 9999 || day == 0 || day > 31 || month == 0 || month > 12 {
        anyhow::bail!("invalid date provided, it must be in the format yyyyMMdd");
    }

    let naive_date = NaiveDate::from_ymd_opt(year as i32, month, day);

    if naive_date.is_none() {
        anyhow::bail!("failed to convert date, invalid date provided, it must be in the format yyyyMMdd");
    }

    Ok(naive_date.unwrap())
}

/// convert the time to the format hhMMss
/// any additional precision will be removed.
///
/// invalid values will be colapsed to the nearest valid value when possible:
/// for example minutes or seconds values of 60 will be changed to 00
pub fn format_time(time: i32) -> Option<i32>{
    let hours = time / 10000;
    let mut minutes = (time / 100) - (hours * 100);
    let mut seconds = time % 100;

    if minutes == 60 {
        minutes = 00;
    }

    if seconds == 60 {
        seconds = 00;
    }

    Some((hours * 10000) + (100 * minutes) + seconds )
}

/// converts the provided time into [NaiveTime]
pub fn parse_time(time: i32) -> Option<NaiveTime> {
    let time = format_time(time)?;
    let hours = time / 10000;
    let minutes = (time / 100) - (hours * 100);
    let seconds = time % 100;

    NaiveTime::from_hms_opt(hours as u32, minutes as u32, seconds as u32)

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_convert_to_naive_date_correctly() {
        assert!(convert_to_naive_date(2024_10_28).is_ok());
        assert!(convert_to_naive_date(2020_12_01).is_ok());
        assert!(convert_to_naive_date(3001_01_31).is_ok());
    }

    #[test]
    fn should_fail_convert_to_naive_date() {
        assert!(convert_to_naive_date(999999999).is_err());
        assert!(convert_to_naive_date(100_10_28).is_err());
        assert!(convert_to_naive_date(2024_00_28).is_err());
        assert!(convert_to_naive_date(0000_01_00).is_err());
        assert!(convert_to_naive_date(2024_01_00).is_err());
        assert!(convert_to_naive_date(2020_13_01).is_err());
        assert!(convert_to_naive_date(3001_01_32).is_err());
    }

    #[test]
    fn should_format_time_correctly() {
        assert_eq!(9_00_00, format_time(9_60_60).unwrap());
        assert_eq!(9_00_10, format_time(9_60_10).unwrap());
        assert_eq!(9_05_00, format_time(9_05_60).unwrap());
        assert_eq!(9_22_33, format_time(9_22_33).unwrap());
        assert_eq!(14_38_57, format_time(14_38_57).unwrap());
        assert_eq!(14_38_00, format_time(14_38_60).unwrap());
        assert_eq!(14_00_57, format_time(14_60_57).unwrap());
    }
}