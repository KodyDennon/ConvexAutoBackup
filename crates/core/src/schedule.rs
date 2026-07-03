use chrono::{DateTime, Datelike, Days, Duration, NaiveTime, TimeZone, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::fmt;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    ZeroInterval,
    InvalidCron,
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInterval => formatter.write_str("interval must be at least one minute"),
            Self::InvalidCron => formatter.write_str("invalid cron expression"),
        }
    }
}

impl std::error::Error for ScheduleError {}

impl From<ScheduleError> for crate::Error {
    fn from(error: ScheduleError) -> Self {
        Self::message(error.to_string())
    }
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
            Self::Cron { expression } => CronExpression::parse(expression)?.next_after(after),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronExpression {
    seconds: CronField,
    minutes: CronField,
    hours: CronField,
    days_of_month: CronField,
    months: CronField,
    days_of_week: CronField,
}

impl CronExpression {
    fn parse(expression: &str) -> Result<Self, ScheduleError> {
        let fields = expression.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err(ScheduleError::InvalidCron);
        }
        Ok(Self {
            seconds: CronField::parse(fields[0], 0, 59, NameSet::None)?,
            minutes: CronField::parse(fields[1], 0, 59, NameSet::None)?,
            hours: CronField::parse(fields[2], 0, 23, NameSet::None)?,
            days_of_month: CronField::parse(fields[3], 1, 31, NameSet::None)?,
            months: CronField::parse(fields[4], 1, 12, NameSet::Months)?,
            days_of_week: CronField::parse(fields[5], 0, 7, NameSet::Weekdays)?,
        })
    }

    fn next_after(&self, after: DateTime<Utc>) -> Result<DateTime<Utc>, ScheduleError> {
        let mut candidate = after + Duration::seconds(1);
        let deadline = after + Duration::days(366 * 5);

        while candidate <= deadline {
            if !self.months.matches(candidate.month()) {
                candidate = next_month_start(candidate);
                continue;
            }

            if !self.matches_day(candidate) {
                candidate = next_day_start(candidate);
                continue;
            }

            if !self.hours.matches(candidate.hour()) {
                candidate = match self.hours.next_at_or_after(candidate.hour()) {
                    Some(hour) => set_time(candidate, hour, 0, 0),
                    None => set_time(next_day_start(candidate), self.hours.first(), 0, 0),
                };
                continue;
            }

            if !self.minutes.matches(candidate.minute()) {
                candidate = match self.minutes.next_at_or_after(candidate.minute()) {
                    Some(minute) => set_time(candidate, candidate.hour(), minute, 0),
                    None => {
                        let next_hour = candidate + Duration::hours(1);
                        set_time(next_hour, next_hour.hour(), self.minutes.first(), 0)
                    }
                };
                continue;
            }

            if !self.seconds.matches(candidate.second()) {
                candidate = match self.seconds.next_at_or_after(candidate.second()) {
                    Some(second) => {
                        set_time(candidate, candidate.hour(), candidate.minute(), second)
                    }
                    None => {
                        let next_minute = candidate + Duration::minutes(1);
                        set_time(
                            next_minute,
                            next_minute.hour(),
                            next_minute.minute(),
                            self.seconds.first(),
                        )
                    }
                };
                continue;
            }

            return Ok(candidate);
        }

        Err(ScheduleError::InvalidCron)
    }

    fn matches_day(&self, candidate: DateTime<Utc>) -> bool {
        let dom_restricted = self.days_of_month.is_restricted();
        let dow_restricted = self.days_of_week.is_restricted();
        let dom_matches = self.days_of_month.matches(candidate.day());
        let dow_matches = self
            .days_of_week
            .matches(candidate.weekday().num_days_from_sunday())
            || self.days_of_week.matches(7) && candidate.weekday() == Weekday::Sun;

        match (dom_restricted, dow_restricted) {
            (true, true) => dom_matches || dow_matches,
            (true, false) => dom_matches,
            (false, true) => dow_matches,
            (false, false) => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronField {
    min: u32,
    max: u32,
    wildcard: bool,
    values: Vec<u32>,
}

impl CronField {
    fn parse(value: &str, min: u32, max: u32, names: NameSet) -> Result<Self, ScheduleError> {
        let mut values = Vec::new();
        let mut wildcard = false;

        for part in value.split(',') {
            if part.is_empty() {
                return Err(ScheduleError::InvalidCron);
            }
            let (base, step) = match part.split_once('/') {
                Some((base, step)) => {
                    let step = step
                        .parse::<u32>()
                        .map_err(|_| ScheduleError::InvalidCron)?;
                    if step == 0 {
                        return Err(ScheduleError::InvalidCron);
                    }
                    (base, step)
                }
                None => (part, 1),
            };

            let (start, end, is_wildcard) = if matches!(base, "*" | "?") {
                (min, max, true)
            } else if let Some((start, end)) = base.split_once('-') {
                (
                    parse_cron_value(start, min, max, names)?,
                    parse_cron_value(end, min, max, names)?,
                    false,
                )
            } else {
                let parsed = parse_cron_value(base, min, max, names)?;
                (parsed, parsed, false)
            };

            if start > end {
                return Err(ScheduleError::InvalidCron);
            }
            wildcard |= is_wildcard;
            let mut current = start;
            while current <= end {
                values.push(current);
                current = current.saturating_add(step);
                if step == 0 {
                    return Err(ScheduleError::InvalidCron);
                }
            }
        }

        values.sort_unstable();
        values.dedup();
        if values.is_empty() {
            return Err(ScheduleError::InvalidCron);
        }
        Ok(Self {
            min,
            max,
            wildcard,
            values,
        })
    }

    fn matches(&self, value: u32) -> bool {
        value >= self.min && value <= self.max && self.values.binary_search(&value).is_ok()
    }

    fn next_at_or_after(&self, value: u32) -> Option<u32> {
        self.values
            .iter()
            .copied()
            .find(|candidate| *candidate >= value)
    }

    fn first(&self) -> u32 {
        self.values[0]
    }

    fn is_restricted(&self) -> bool {
        !self.wildcard
    }
}

#[derive(Debug, Clone, Copy)]
enum NameSet {
    None,
    Months,
    Weekdays,
}

fn parse_cron_value(value: &str, min: u32, max: u32, names: NameSet) -> Result<u32, ScheduleError> {
    let parsed = match names {
        NameSet::None => value.parse::<u32>().ok(),
        NameSet::Months => month_name(value).or_else(|| value.parse::<u32>().ok()),
        NameSet::Weekdays => weekday_name(value).or_else(|| value.parse::<u32>().ok()),
    }
    .ok_or(ScheduleError::InvalidCron)?;

    if parsed < min || parsed > max {
        return Err(ScheduleError::InvalidCron);
    }
    Ok(parsed)
}

fn month_name(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

fn weekday_name(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
        "sun" => Some(0),
        "mon" => Some(1),
        "tue" => Some(2),
        "wed" => Some(3),
        "thu" => Some(4),
        "fri" => Some(5),
        "sat" => Some(6),
        _ => None,
    }
}

fn next_month_start(candidate: DateTime<Utc>) -> DateTime<Utc> {
    let (year, month) = if candidate.month() == 12 {
        (candidate.year() + 1, 1)
    } else {
        (candidate.year(), candidate.month() + 1)
    };
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .expect("UTC date time is unambiguous")
}

fn next_day_start(candidate: DateTime<Utc>) -> DateTime<Utc> {
    let date = candidate
        .date_naive()
        .checked_add_days(Days::new(1))
        .expect("date addition should not overflow");
    Utc.from_local_datetime(&date.and_time(NaiveTime::MIN))
        .single()
        .expect("UTC date time is unambiguous")
}

fn set_time(candidate: DateTime<Utc>, hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(
        candidate.year(),
        candidate.month(),
        candidate.day(),
        hour,
        minute,
        second,
    )
    .single()
    .expect("UTC date time is unambiguous")
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
    fn cron_schedule_supports_steps_lists_ranges_and_names() {
        let next = Schedule::Cron {
            expression: "0 */15 9-17 * jan,Jul mon-fri".to_string(),
        }
        .next_after(dt(1, 10, 16))
        .unwrap();

        assert_eq!(next.month(), 7);
        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 30);
        assert_eq!(next.second(), 0);
    }

    #[test]
    fn cron_schedule_rejects_invalid_expressions() {
        assert_eq!(
            Schedule::Cron {
                expression: "0 0 2 * *".to_string(),
            }
            .next_after(dt(1, 10, 0)),
            Err(ScheduleError::InvalidCron)
        );
        assert_eq!(
            Schedule::Cron {
                expression: "0 0 25 * * *".to_string(),
            }
            .next_after(dt(1, 10, 0)),
            Err(ScheduleError::InvalidCron)
        );
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
