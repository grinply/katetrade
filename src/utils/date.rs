use chrono::NaiveDate;

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
}