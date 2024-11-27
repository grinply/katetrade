use chrono::{NaiveDate, NaiveTime, Timelike};
use crate::models::candlestick::TimeUnit;

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
pub fn convert_to_naive_time(time: i32) -> Option<NaiveTime> {
    let time = format_time(time)?;
    let hours = time / 10000;
    let minutes = (time / 100) - (hours * 100);
    let seconds = time % 100;

    NaiveTime::from_hms_opt(hours as u32, minutes as u32, seconds as u32)
}

pub fn increase_time(current_time: &NaiveTime, amount_to_increase: u8, unit: TimeUnit) -> NaiveTime{
    match unit {
        TimeUnit::Seconds => {
            let second_sum = current_time.second() + amount_to_increase as u32;
            if second_sum >= 60 {
                NaiveTime::from_hms_opt(current_time.hour(), current_time.minute() + second_sum/60, second_sum % 60)
                    .unwrap()
            } else{
                NaiveTime::from_hms_opt(current_time.hour(), current_time.minute(), second_sum)
                    .unwrap()
            }
        }
        TimeUnit::Minutes => {
            let minute_sum = current_time.minute() + amount_to_increase as u32;
            if minute_sum >= 60 {
                let mut hour = current_time.hour() + minute_sum / 60;
                if hour >= 24 {
                    hour = hour % 24;
                }
                NaiveTime::from_hms_opt(hour, minute_sum % 60, current_time.second())
                    .unwrap()
            }else{
                NaiveTime::from_hms_opt(current_time.hour(), minute_sum, current_time.second())
                    .unwrap()
            }
        }
        TimeUnit::Hours => {
            let hour_sum = current_time.hour() + amount_to_increase as u32;
            if hour_sum >= 24 {
                NaiveTime::from_hms_opt(hour_sum % 24, current_time.minute(), current_time.second())
                    .unwrap()
            }else{
                NaiveTime::from_hms_opt(hour_sum, current_time.minute(), current_time.second())
                    .unwrap()
            }
        }
    }
}

/// compare if the time defined by hour, minutes and seconds is after current_time
pub fn is_time_after(current_time: NaiveTime, hour: u8, minutes: u8, seconds: u8) -> bool{
    let current = (current_time.hour() * 10000) + (100 * current_time.minute()) + current_time.second();
    let target = (hour as u32 * 10000) + (100 * minutes as u32) + seconds as u32;
    target >= current
}

#[cfg(test)]
mod tests {
    use crate::models::candlestick::TimeUnit::{Hours, Minutes, Seconds};
    use super::*;

    #[test]
    fn should_increase_time_correctly() {
        let tests:Vec<(i32, u8, TimeUnit, i32)> = vec![
            (09_10_00, 15, Seconds, 09_10_15),
            (09_10_15, 2, Seconds, 09_10_17),
            (09_10_15, 30, Minutes, 09_40_15),
            (09_40_15, 10, Hours, 19_40_15),

            // sum will trigger one increment in the next unit of time
            (09_40_15, 50, Seconds, 09_41_05),
            (09_40_15, 50, Minutes, 10_30_15),
            (09_40_15, 15, Hours, 00_40_15),

            // sum will trigger more than one increment in the next unit of time
            (09_40_15, 121, Seconds, 09_42_16),
            (09_40_15, 121, Minutes, 11_41_15),
            (09_40_15, 30, Hours, 15_40_15),

            // simple candlestick cycle of 15 seconds
            (11_00_00, 15, Seconds, 11_00_15),
            (11_00_15, 15, Seconds, 11_00_30),
            (11_00_30, 15, Seconds, 11_00_45),
            (11_00_45, 15, Seconds, 11_01_00),

            // simple candlestick cycle of 5 minutes
            (11_00_00, 5, Minutes, 11_05_00),
            (11_05_00, 5, Minutes, 11_10_00),
            (11_10_00, 5, Minutes, 11_15_00),
            (11_15_00, 5, Minutes, 11_20_00),
            (11_20_00, 5, Minutes, 11_25_00),
            (11_25_00, 5, Minutes, 11_30_00),
            (11_30_00, 5, Minutes, 11_35_00),
            (11_35_00, 5, Minutes, 11_40_00),
            (11_40_00, 5, Minutes, 11_45_00),
            (11_45_00, 5, Minutes, 11_50_00),
            (11_50_00, 5, Minutes, 11_55_00),
            (11_55_00, 5, Minutes, 12_00_00),
        ];

        for (input_time, period, time_unit, expected_time) in tests {
            let input_value = convert_to_naive_time(input_time).unwrap();
            let expected_value = convert_to_naive_time(expected_time).unwrap();
            let result = increase_time(&input_value, period, time_unit);
            assert_eq!(expected_value, result,
                       "given the increment of {} {:?} in the time {}, the expected value is {}, but the result was {}",
                       period, time_unit, input_value, expected_value, result);
        }
    }

    #[test]
    fn should_compare_time_correctly() {
        let current = convert_to_naive_time(09_10_33).unwrap();
        assert_eq!(true, is_time_after(current, 10, 0, 0));
        assert_eq!(true, is_time_after(current, 09,10,33));
        assert_eq!(true, is_time_after(current, 09,10,34));
        assert_eq!(true, is_time_after(current, 09,11,32));
        assert_eq!(false, is_time_after(current, 09,10,32));
        assert_eq!(false, is_time_after(current, 09,00,33));
        assert_eq!(false, is_time_after(current, 0,0,0));
    }

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