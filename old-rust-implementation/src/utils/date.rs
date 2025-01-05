#![allow(dead_code)]
#![allow(unused_variables)]
use chrono::{NaiveDate, NaiveTime, Timelike};
use crate::models::candlestick::TimeUnit;

/// Convert a [NaiveTime] into a [i32] with the format hhMMss
pub fn convert_to_formatted_time(naive_time: NaiveTime) -> i32 {
    ((naive_time.hour() * 10000) + (100 * naive_time.minute()) + naive_time.second()) as i32
}

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
/// invalid values will be collapsed to the nearest valid value when possible:
/// for example minutes or seconds values of 60 will be changed to 00
pub fn format_time(time: i32) -> Option<i32> {
    if time > 99_99_99_999 {
        return None;
    }

    let time = if time < 99_99_99 {
        time
    }else{
        time / 1000
    };

    let hours = time / 10000;
    let mut minutes = (time / 100) - (hours * 100);
    let mut seconds = time % 100;

    if minutes == 60 {
        minutes = 00;
    }

    if seconds == 60 {
        seconds = 00;
    }

    Some((hours * 10000) + (100 * minutes) + seconds)
}

/// converts the provided time into [NaiveTime]
/// the only accepted formats are hhMMss or hhMMssmmm
pub fn convert_to_naive_time(time: i32) -> Option<NaiveTime> {
    if time > 99_99_99_999 {
        return None;
    }

    let mut milli = 0;

    let time = if time > 99_99_99 {
        milli = (time % 1000) as u32;
        time / 1000
    } else {
        time
    };

    let time = format_time(time)?;
    let hours = time / 10000;
    let minutes = (time / 100) - (hours * 100);
    let seconds = time % 100;

    NaiveTime::from_hms_milli_opt(hours as u32, minutes as u32, seconds as u32, milli)
}


/// Decreases a given amount of time from the current_time
///
/// **important:** when using seconds or minutes as unit you cannot decrease more than 60 units.
/// when using hours you cannot decrease more than 24 hours. **If you do the code will panic !!**
///
/// for example: 18:00:00 decreased of 5 minutes will result in 17:55:00
#[allow(unused)]
pub fn decrease_time(current_time: &NaiveTime, amount_to_decrease: u8, unit: TimeUnit) -> NaiveTime {
    match unit {
        TimeUnit::Seconds => {
            assert!(amount_to_decrease <= 60, "the maximum amount of seconds allowed to be decreased is 60");
            let offset_seconds = current_time.second() as i32 - amount_to_decrease as i32;
            if offset_seconds < 0 {
                let mut minutes = current_time.minute();
                let mut hours = current_time.hour();

                if minutes == 0 {
                    minutes = 59;

                    if hours == 0 {
                        hours = 23;
                    }else{
                        hours = hours -1;
                    }
                }else{
                    minutes = minutes -1;
                }

                NaiveTime::from_hms_opt(hours, minutes, 60 - offset_seconds.abs() as u32)
                    .unwrap_or_else(|| panic!("should be able to decrease {amount_to_decrease} {:?} from {current_time}", unit))
            } else {
                NaiveTime::from_hms_opt(current_time.hour(), current_time.minute(), offset_seconds as u32)
                    .unwrap_or_else(|| panic!("should be able to decrease {amount_to_decrease} {:?} from {current_time}", unit))
            }
        }
        TimeUnit::Minutes => {
            assert!(amount_to_decrease <= 60, "the maximum amount of minutes allowed to be decreased is 60");
            let offset_minutes = current_time.minute() as i32 - amount_to_decrease as i32;
            if offset_minutes < 0 {
                let mut hours = current_time.hour();

                if hours == 0 {
                   hours = 23;
                }else{
                    hours = hours - 1;
                }

                NaiveTime::from_hms_opt(hours, 60 - offset_minutes.abs() as u32, current_time.second())
                    .unwrap_or_else(|| panic!("should be able to decrease {amount_to_decrease} {:?} from {current_time}", unit))
            } else {
                NaiveTime::from_hms_opt(current_time.hour(), offset_minutes as u32, current_time.second())
                    .unwrap_or_else(|| panic!("should be able to decrease {amount_to_decrease} {:?} from {current_time}", unit))
            }
        }
        TimeUnit::Hours => {
            assert!(amount_to_decrease <= 24, "the maximum amount of hours allowed to be decreased is 24");
            let hours_offset = current_time.hour() as i32 - amount_to_decrease as i32;
            if hours_offset < 0 {
                NaiveTime::from_hms_opt(24 - hours_offset.abs() as u32, current_time.minute(), current_time.second())
                    .unwrap_or_else(|| panic!("should be able to decrease {amount_to_decrease} {:?} from {current_time}", unit))
            } else {
                NaiveTime::from_hms_opt(hours_offset as u32, current_time.minute(), current_time.second())
                    .unwrap_or_else(|| panic!("should be able to decrease {amount_to_decrease} {:?} from {current_time}", unit))
            }
        }
    }
}

/// Increases a given amount of time from the current_time
///
/// **important:** when using seconds or minutes as unit you cannot increase more than 60 units.
/// when using hours you cannot increase more than 24 hours. **If you do the code will panic !!**
///
/// for example: 17:55:00 decreased of 5 minutes will result in 18:00:00
pub fn increase_time(current_time: &NaiveTime, amount_to_increase: u8, unit: TimeUnit) -> NaiveTime {
    match unit {
        TimeUnit::Seconds => {
            let second_sum = current_time.second() + amount_to_increase as u32;
            if second_sum >= 60 {
                let mut hours = current_time.hour();
                let mut actual_minutes = current_time.minute() + second_sum / 60;

                if actual_minutes >= 60 {
                    actual_minutes = 0;
                    hours+=1;
                }

                if hours >= 24 {
                    hours = 0;
                }

                NaiveTime::from_hms_opt(hours, actual_minutes, second_sum % 60)
                    .unwrap_or_else(|| panic!("should be able to increment {amount_to_increase} {:?} from {current_time}", unit))
            } else {
                NaiveTime::from_hms_opt(current_time.hour(), current_time.minute(), second_sum)
                    .unwrap_or_else(|| panic!("should be able to increment {amount_to_increase} {:?} from {current_time}", unit))
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
                    .unwrap_or_else(|| panic!("should be able to increment {amount_to_increase} {:?} from {current_time}", unit))
            } else {
                NaiveTime::from_hms_opt(current_time.hour(), minute_sum, current_time.second())
                    .unwrap_or_else(|| panic!("should be able to increment {amount_to_increase} {:?} from {current_time}", unit))
            }
        }
        TimeUnit::Hours => {
            let hour_sum = current_time.hour() + amount_to_increase as u32;
            if hour_sum >= 24 {
                NaiveTime::from_hms_opt(hour_sum % 24, current_time.minute(), current_time.second())
                    .unwrap_or_else(|| panic!("should be able to increment {amount_to_increase} {:?} from {current_time}", unit))
            } else {
                NaiveTime::from_hms_opt(hour_sum, current_time.minute(), current_time.second())
                    .unwrap_or_else(|| panic!("should be able to increment {amount_to_increase} {:?} from {current_time}", unit))
            }
        }
    }
}

/// compare if the time defined by hour, minutes and seconds is after current_time
pub fn is_time_after(current_time: NaiveTime, hour: u8, minutes: u8, seconds: u8) -> bool {
    let current = (current_time.hour() * 10000) + (100 * current_time.minute()) + current_time.second();
    let target = (hour as u32 * 10000) + (100 * minutes as u32) + seconds as u32;
    target >= current
}

#[cfg(test)]
mod tests {
    use crate::models::candlestick::TimeUnit::{Hours, Minutes, Seconds};
    use super::*;

    #[test]
    fn should_decrease_time_correctly() {
        let tests: Vec<(i32, u8, TimeUnit, i32)> = vec![
            // edge cases the time will affect the next unit
            (18_57_00, 60, Minutes, 17_57_00),
            (18_50_54, 60, Seconds, 18_49_54),

            (18_00_54, 60, Seconds, 17_59_54),
            (00_57_00, 60, Minutes, 23_57_00),
            (00_56_00, 13, Hours, 11_56_00),

            (18_00_00, 5, Minutes, 17_55_00),
            (18_00_00, 10, Minutes, 17_50_00),
            (18_00_00, 15, Minutes, 17_45_00),

            (09_10_00, 15, Seconds, 09_09_45),
            (09_10_15, 2, Seconds, 09_10_13),
            (09_10_15, 30, Minutes, 08_40_15),
            (09_40_15, 10, Hours, 23_40_15),

            // simple candlestick cycle of 15 seconds
            (11_00_15, 15, Seconds, 11_00_00),
            (11_00_30, 15, Seconds, 11_00_15),
            (11_00_45, 15, Seconds, 11_00_30),
            (11_01_00, 15, Seconds, 11_00_45),

            // simple candlestick cycle of 5 minutes
            (11_05_00, 5, Minutes, 11_00_00),
            (11_10_00, 5, Minutes, 11_05_00),
            (11_15_00, 5, Minutes, 11_10_00),
            (11_20_00, 5, Minutes, 11_15_00),
            (11_25_00, 5, Minutes, 11_20_00),
            (11_30_00, 5, Minutes, 11_25_00),
            (11_35_00, 5, Minutes, 11_30_00),
            (11_40_00, 5, Minutes, 11_35_00),
            (11_45_00, 5, Minutes, 11_40_00),
            (11_50_00, 5, Minutes, 11_45_00),
            (11_55_00, 5, Minutes, 11_50_00),
            (12_00_00, 5, Minutes, 11_55_00),
        ];

        for (input_time, period, time_unit, expected_time) in tests {
            let input_value = convert_to_naive_time(input_time).unwrap();
            let expected_value = convert_to_naive_time(expected_time).unwrap();
            let result = decrease_time(&input_value, period, time_unit);
            assert_eq!(expected_value, result,
                       "given the decrement of {} {:?} in the time {}, the expected value is {}, but the result was {}",
                       period, time_unit, input_value, expected_value, result);
        }
    }

    #[test]
    fn should_increase_time_correctly() {
        let tests: Vec<(i32, u8, TimeUnit, i32)> = vec![
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
            (09_59_45, 15, Seconds, 10_00_00),

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
        assert_eq!(true, is_time_after(current, 09, 10, 33));
        assert_eq!(true, is_time_after(current, 09, 10, 34));
        assert_eq!(true, is_time_after(current, 09, 11, 32));
        assert_eq!(false, is_time_after(current, 09, 10, 32));
        assert_eq!(false, is_time_after(current, 09, 00, 33));
        assert_eq!(false, is_time_after(current, 0, 0, 0));
    }

    #[test]
    fn should_convert_to_naive_date_correctly() {
        assert!(convert_to_naive_date(2024_10_28).is_ok());
        assert!(convert_to_naive_date(2020_12_01).is_ok());
        assert!(convert_to_naive_date(3001_01_31).is_ok());
    }

    #[test]
    fn should_convert_to_naive_time_correctly() {
        let valid_times = vec![
            (09_00_11, 09, 00, 11, 00),
            (11_11_11, 11, 11, 11, 00),
            (9_01_07_234, 09, 01, 07, 234)
        ];

        for (time, hour, minute, second, milli) in valid_times {
            let naive_time = convert_to_naive_time(time);
            assert!(
                naive_time.is_some(),
                "time {} should be convert correctly on conversion to NaiveTime",
                time
            );
            let naive_time = naive_time.unwrap();
            assert_eq!(naive_time.hour(), hour);
            assert_eq!(naive_time.minute(), minute);
            assert_eq!(naive_time.second(), second);
            assert_eq!(naive_time.nanosecond(), milli * 1000000);
        }
    }

    #[test]
    fn should_fail_convert_to_naive_date_correctly() {
        let invalid_dates = vec![
            999999999,
            100_10_28,
            2024_00_28,
            0000_01_00,
            2024_01_00,
            2020_13_01,
            3001_01_32,
        ];

        for date in invalid_dates {
            assert!(
                convert_to_naive_date(date).is_err(),
                "date {} should fail on conversion to NaiveDate",
                date
            );
        }
    }

    #[test]
    fn should_format_time_correctly() {
        let valid_times = vec![
            (9_60_60, 9_00_00),
            (9_60_10, 9_00_10),
            (9_05_60, 9_05_00),
            (9_22_33, 9_22_33),
            (14_38_57, 14_38_57),
            (14_38_60, 14_38_00),
            (14_60_57, 14_00_57),
        ];

        for time in valid_times {
            let expected_time = time.1;
            let actual_time = format_time(time.0);
            assert!(actual_time.is_some());
            assert_eq!(
                expected_time,
                actual_time.unwrap(),
                "after formating the time {} we expected the result to be {}, but it was {}",
                time.0,
                expected_time,
                actual_time.unwrap()
            );
        }
    }
}