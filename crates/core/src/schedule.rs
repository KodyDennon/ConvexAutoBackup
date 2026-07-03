use chrono::{DateTime, Datelike, Days, Duration, NaiveTime, TimeZone, Utc, Weekday};
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schedule {
    IntervalMinutes { every: u32 },
    Daily { time: NaiveTime },
    Weekly { weekday: Weekday, time: NaiveTime },
    Cron { expression: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    RunOnceOnResume,
    Skip,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScheduleError {
    #[error("interval must be at least one minute")]
    ZeroInterval,
    #[error("invalid cron expression")]
    InvalidCron,
}

impl Schedule {
    pub fn next_after(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, ScheduleError> {
        match self {
            Self::IntervalMinutes { every } => {
                if *every == 0 {
                    return Err(ScheduleError::ZeroInterval);
                }
                Ok(after + Duration::minutes(i64::from(*every)))
            }
            Self::Daily { time } => {
                let today = after.date_naive();
                let candidate = Utc
                    .from_local_datetime(&today.and_time(*time))
                    .single()
                    .expect("UTC date time is unambiguous");
                if candidate > after {
                    Ok(candidate)
                } else {
                    Ok(Utc
                        .from_local_datetime(
                            &today
                                .checked_add_days(Days::new(1))
                                .expect("date addition should not overflow")
                                .and_time(*time),
                        )
                        .single()
                        .expect("UTC date time is unambiguous"))
                }
            }
            Self::Weekly { weekday, time } => {
                let mut date = after.date_naive();
                for _ in 0..8 {
                    if date.weekday() == *weekday {
                        let candidate = Utc
                            .from_local_datetime(&date.and_time(*time))
                            .single()
                            .expect("UTC date time is unambiguous");
                        if candidate > after {
                            return Ok(candidate);
                        }
                    }
                    date = date
                        .checked_add_days(Days::new(1))
                        .expect("date addition should not overflow");
                }
                unreachable!("weekly schedule should find a matching day within eight days")
            }
            Self::Cron { expression } => CronSchedule::from_str(expression)
                .map_err(|_| ScheduleError::InvalidCron)?
                .after(&after)
                .next()
                .ok_or(ScheduleError::InvalidCron),
        }
    }

    pub fn should_catch_up(
        &self,
        last_due_at: DateTime<Utc>,
        resumed_at: DateTime<Utc>,
        policy: MissedRunPolicy,
    ) -> bool {
        match policy {
            MissedRunPolicy::Skip => false,
            MissedRunPolicy::RunOnceOnResume => self
                .next_after(last_due_at)
                .map(|next| next <= resumed_at)
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Timelike};

    fn dt(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, day)
                .unwrap()
                .and_hms_opt(hour, minute, 0)
                .unwrap(),
        )
    }

    #[test]
    fn interval_schedule_rejects_zero_minutes() {
        let result = Schedule::IntervalMinutes { every: 0 }.next_after(dt(1, 10, 0));
        assert_eq!(result, Err(ScheduleError::ZeroInterval));
    }

    #[test]
    fn interval_schedule_returns_next_due_time() {
        let next = Schedule::IntervalMinutes { every: 90 }
            .next_after(dt(1, 10, 0))
            .unwrap();
        assert_eq!(next, dt(1, 11, 30));
    }

    #[test]
    fn daily_schedule_uses_today_when_time_is_future() {
        let next = Schedule::Daily {
            time: NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
        }
        .next_after(dt(1, 10, 0))
        .unwrap();
        assert_eq!(next, dt(1, 23, 0));
    }

    #[test]
    fn daily_schedule_rolls_to_tomorrow_after_time_passes() {
        let next = Schedule::Daily {
            time: NaiveTime::from_hms_opt(2, 0, 0).unwrap(),
        }
        .next_after(dt(1, 10, 0))
        .unwrap();
        assert_eq!(next, dt(2, 2, 0));
    }

    #[test]
    fn weekly_schedule_finds_next_named_day() {
        let next = Schedule::Weekly {
            weekday: Weekday::Mon,
            time: NaiveTime::from_hms_opt(9, 15, 0).unwrap(),
        }
        .next_after(dt(2, 12, 0))
        .unwrap();
        assert_eq!(next.weekday(), Weekday::Mon);
        assert_eq!(next.hour(), 9);
        assert_eq!(next.minute(), 15);
    }

    #[test]
    fn cron_schedule_parses_standard_six_field_expression() {
        let next = Schedule::Cron {
            expression: "0 0 2 * * *".to_string(),
        }
        .next_after(dt(1, 10, 0))
        .unwrap();
        assert_eq!(next, dt(2, 2, 0));
    }

    #[test]
    fn catch_up_policy_detects_missed_due_time() {
        let schedule = Schedule::IntervalMinutes { every: 60 };
        assert!(schedule.should_catch_up(
            dt(1, 10, 0),
            dt(1, 12, 30),
            MissedRunPolicy::RunOnceOnResume
        ));
        assert!(!schedule.should_catch_up(dt(1, 10, 0), dt(1, 12, 30), MissedRunPolicy::Skip));
    }
}
