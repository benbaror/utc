extern crate peg;
use chrono::{Duration, FixedOffset, LocalResult, TimeZone};
use peg::parser;
use regex::Regex;
use std::{
    ops::{Add, Sub},
    panic,
};

fn get_time_zone(input: &str) -> Option<FixedOffset> {
    let re = Regex::new(r"^#UTC([+-])(\d{1,2})$").unwrap();
    match re.captures(input.trim()) {
        Some(x) => {
            if x.len() != 3 {
                return None;
            }
            let sign = &x[1];
            let value = match (&x[2]).parse::<i32>().unwrap_or(-25) {
                x @ -23..=23 => Some(x * 3600),
                _ => None,
            };
            match (sign, value) {
                ("+", Some(value)) => FixedOffset::east_opt(value),
                ("-", Some(value)) => FixedOffset::east_opt(-value),
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn parse(input: String, now: i64) -> Vec<Record> {
    let mut records = vec![];
    let mut offset = FixedOffset::east(0);
    let split = input.split('\n');
    for line in split {
        let expression = safe_parse_line(line, offset, now, &records);
        records.push(Record { offset, expression });
        offset = match expression {
            Expression::Offset(offset) => offset,
            _ => offset,
        };
    }
    records
}

fn parse_line(input: &str, offset: FixedOffset, now: i64, records: &[Record]) -> Expression {
    let expressions: Vec<Expression> = records.iter().map(|record| record.into()).collect();
    let state = State::new(offset, now, &expressions);
    match arithmetic::expression(input, &state) {
        Ok(result) => result,
        _ => match get_time_zone(input) {
            Some(offset) => Expression::Offset(offset),
            _ => Expression::None,
        },
    }
}

fn safe_parse_line(input: &str, offset: FixedOffset, now: i64, records: &[Record]) -> Expression {
    let result = panic::catch_unwind(|| parse_line(input, offset, now, records));
    match result {
        Ok(result) => result,
        _ => Expression::None,
    }
}

impl From<&Record> for Expression {
    fn from(record: &Record) -> Self {
        record.expression
    }
}

pub struct Record {
    pub offset: FixedOffset,
    pub expression: Expression,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Expression {
    Offset(FixedOffset),
    Duration(Duration),
    Timestamp(i64),
    None,
}

impl Expression {
    fn timestamp(timestamp: Option<i64>) -> Self {
        match timestamp {
            Some(timestamp) => Self::Timestamp(timestamp),
            _ => Self::None,
        }
    }
    fn seconds(seconds: Option<i64>) -> Self {
        match seconds {
            Some(seconds) => Self::milliseconds(seconds.checked_mul(1000)),
            _ => Self::None,
        }
    }
    fn milliseconds(milliseconds: Option<i64>) -> Self {
        match milliseconds {
            Some(milliseconds) => Self::Duration(Duration::milliseconds(milliseconds)),
            _ => Self::None,
        }
    }
}

impl Add<Expression> for Expression {
    type Output = Expression;

    fn add(self, rhs: Expression) -> Expression {
        match (self, rhs) {
            (Expression::Duration(l), Expression::Duration(r)) => Expression::Duration(l + r),
            (Expression::Duration(l), Expression::Timestamp(r)) => {
                Expression::timestamp(r.checked_add(l.num_seconds()))
            }
            (Expression::Timestamp(l), Expression::Duration(r)) => {
                Expression::timestamp(l.checked_add(r.num_seconds()))
            }
            (Expression::Timestamp(l), Expression::Timestamp(r)) => {
                Expression::seconds(l.checked_add(r))
            }
            _ => Expression::None,
        }
    }
}

impl Sub<Expression> for Expression {
    type Output = Expression;

    fn sub(self, rhs: Expression) -> Expression {
        match (self, rhs) {
            (Expression::Duration(l), Expression::Duration(r)) => Expression::Duration(l - r),
            (Expression::Duration(l), Expression::Timestamp(r)) => {
                Expression::timestamp(l.num_seconds().checked_sub(r))
            }
            (Expression::Timestamp(l), Expression::Duration(r)) => {
                Expression::timestamp(l.checked_sub(r.num_seconds()))
            }
            (Expression::Timestamp(l), Expression::Timestamp(r)) => {
                Expression::seconds(l.checked_sub(r))
            }
            _ => Expression::None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct State<'a> {
    offset: FixedOffset,
    now: i64,
    records: &'a [Expression],
}

impl<'a> State<'a> {
    pub fn new(offset: FixedOffset, now: i64, records: &'a [Expression]) -> Self {
        Self {
            offset,
            now,
            records,
        }
    }
}

parser!(
    pub grammar arithmetic(state: &State) for str {
    use peg::ParseLiteral;

    pub rule expression() -> Expression = precedence!{
        x:(@) _ "+" _ y:@ { x + y }
        x:(@) _ "-" _ y:@ { x - y }
        --
        "(" _ v:expression() _ ")" { v }
        d:duration_expression() { Expression::Duration(d) }
        t:timestamp() {t}
        r:record() {r}
    }

    rule _ = quiet!{[' ']*}
    rule end() = !['a'..='z' | 'A'..='Z']

    rule record() -> Expression = "#" + idx:$(['0'..='9']+) {
        let record_index: usize = idx.parse().unwrap();
        match state.records.get(record_index - 1) {
            Some(v) => *v,
            _ => Expression::None
        }
    }

    rule days() -> Duration
        = n:number() "d" { Duration::milliseconds((n * 1e3 * 60.0 * 60.0 * 24.0) as i64) }

    rule hours() -> Duration
        = n:number() "h" end() { Duration::milliseconds((n * 1e3 * 60.0 * 60.0) as i64) }

    rule minutes() -> Duration
        = n:number() "m" end() { Duration::milliseconds((n * 1e3 * 60.0) as i64) }

    rule seconds() -> Duration
        = n:number() "s" end() { Duration::milliseconds((n * 1e3) as i64) }

    rule milliseconds() -> Duration
         = n:number() "ms" end() { Duration::milliseconds(n as i64) }

    rule duration_expression() -> Duration = precedence!{
        x:(@) "" y:@ { x + y }
        --
        d:duration() {d}
    }

    rule duration() -> Duration
        = s:seconds() {s}
        / m:minutes() {m}
        / h:hours() {h}
        / d:days() {d}
        / ms:milliseconds() {ms}

    rule timestamp() -> Expression
        = ("-")n:number()end() {Expression::Timestamp(-n as i64)}
        / n:number()end() {Expression::Timestamp(n as i64)} / datetime() / $("now") {Expression::Timestamp(state.now)}

    rule number() -> f64
        = n:$(['0'..='9']+(r"."(['0'..='9']+)?)?) { n.parse().unwrap() }

    pub rule bad_number() -> f64
        = n:$("a"['0'..='9']+(r"."(['0'..='9']+)?)?) { n.parse().unwrap() }

    rule n_digit_number(n: usize) -> u32
        = s:$(['0'..='9']*<{n}>) { s.parse().unwrap() }

    rule ydm_fmt(sep: &str) -> (i32, u32, u32)
        = year:n_digit_number(4)##parse_string_literal(sep)
          + month:n_digit_number(2)##parse_string_literal(sep)
          + day:n_digit_number(2)
        {
            (year as i32, month, day)
        }

    rule hms_fmt() -> (u32, u32, u32)
        = hour:n_digit_number(2)":"
          + minute:n_digit_number(2)":"
          + second:n_digit_number(2)
        {
            (hour, minute, second)
        }

    rule datetime_fmt(sep_ymd: &str, sep: &str) -> Expression
        = "'" ymd:ydm_fmt(sep_ymd)##parse_string_literal(sep) + hms:hms_fmt() "'"
        {
            let tz = state.offset;
            let (year, month, day) = ymd;
            let (hour, minute, second) = hms;
            let datetime = TimeZone::ymd_opt(&tz, year, month, day).map(|s| s.and_hms_opt(hour, minute, second));
            match datetime {
                LocalResult::Single(Some(datetime)) => Expression::Timestamp(datetime.timestamp()),
                _ => Expression::None
            }
        }

    rule datetime() -> Expression
        = d:datetime_fmt("-", " ") {d}
        / d:datetime_fmt("-", "T") {d}
        / d:datetime_fmt("/", " ") {d}
});

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn durations() {
        let records = vec![];
        let state = State::new(FixedOffset::east(0), 0, &records);
        assert_eq!(
            arithmetic::expression("30s + 5m + 4h", &state),
            Ok(Expression::Duration(Duration::seconds(
                30 + 5 * 60 + 4 * 60 * 60
            )))
        );
        assert_eq!(
            arithmetic::expression("30s - 5m + 4h", &state),
            Ok(Expression::Duration(Duration::seconds(
                30 - 5 * 60 + 4 * 60 * 60
            )))
        );
        assert_eq!(
            arithmetic::expression("6s + 10m + 4h", &state),
            Ok(Expression::Duration(Duration::seconds(
                2 * (3 + 5 * 60) + 4 * 60 * 60
            )))
        );
        assert_eq!(
            arithmetic::expression("6.0s + 10.0m + 1.5h", &state),
            Ok(Expression::Duration(Duration::seconds(
                2 * (3 + 5 * 60) + 90 * 60
            )))
        );
        assert_eq!(
            arithmetic::expression("0.1s", &state),
            Ok(Expression::Duration(Duration::milliseconds(100)))
        );
        assert_eq!(
            arithmetic::expression("5s - 2h", &state),
            Ok(Expression::Duration(Duration::seconds(5 - 2 * 60 * 60)))
        );
        assert_eq!(
            arithmetic::expression("4h5m30s", &state),
            Ok(Expression::Duration(Duration::seconds(
                30 + 5 * 60 + 4 * 60 * 60
            )))
        );
        assert_eq!(
            arithmetic::expression("4h5m + 2s", &state),
            Ok(Expression::Duration(Duration::seconds(
                5 * 60 + 4 * 60 * 60 + 2
            )))
        );
        assert_eq!(
            arithmetic::expression("4h30s", &state),
            Ok(Expression::Duration(Duration::seconds(30 + 4 * 60 * 60)))
        );
        assert_eq!(
            arithmetic::expression("5m30s", &state),
            Ok(Expression::Duration(Duration::seconds(30 + 5 * 60)))
        );
        assert_eq!(
            arithmetic::expression("4h5m30s + 2s", &state),
            Ok(Expression::Duration(Duration::seconds(
                30 + 5 * 60 + 4 * 60 * 60 + 2
            )))
        );
        assert_eq!(
            arithmetic::expression("4h2s5ms", &state),
            Ok(Expression::Duration(
                Duration::seconds(4 * 60 * 60 + 2) + Duration::milliseconds(5)
            ))
        );
    }

    #[test]
    fn timestamps() {
        let records = vec![];
        let state = State::new(FixedOffset::east(5 * 3600), 0, &records);
        let tz = FixedOffset::east(5 * 3600);
        let d = chrono::TimeZone::ymd(&tz, 2014, 5, 6).and_hms(10, 8, 7);
        assert_eq!(
            arithmetic::expression("0", &state),
            Ok(Expression::Timestamp(0))
        );
        assert_eq!(
            arithmetic::expression("1006", &state),
            Ok(Expression::Timestamp(1006))
        );
        assert_eq!(
            arithmetic::expression("1006.0", &state),
            Ok(Expression::Timestamp(1006))
        );
        assert_eq!(
            arithmetic::expression("1006.1", &state),
            Ok(Expression::Timestamp(1006))
        );
        assert_eq!(
            arithmetic::expression("-1006", &state),
            Ok(Expression::Timestamp(-1006))
        );
        assert_eq!(
            arithmetic::expression("-1006.0", &state),
            Ok(Expression::Timestamp(-1006))
        );
        assert_eq!(
            arithmetic::expression("3 + 2h", &state),
            Ok(Expression::Timestamp(3 + 2 * 60 * 60))
        );
        assert_eq!(
            arithmetic::expression("( 3 + 2h )", &state),
            Ok(Expression::Timestamp(3 + 2 * 60 * 60))
        );
        assert_eq!(
            arithmetic::expression("(3 + 2h)", &state),
            Ok(Expression::Timestamp(3 + 2 * 60 * 60))
        );
        assert_eq!(
            arithmetic::expression("3 -2h", &state),
            Ok(Expression::Timestamp(3 - 2 * 60 * 60))
        );
        assert_eq!(
            arithmetic::expression("3-2h", &state),
            Ok(Expression::Timestamp(3 - 2 * 60 * 60))
        );
        assert_eq!(
            arithmetic::expression("3- 2h", &state),
            Ok(Expression::Timestamp(3 - 2 * 60 * 60))
        );
        assert_eq!(
            arithmetic::expression("3- 2h + 5m", &state),
            Ok(Expression::Timestamp(3 - 2 * 60 * 60 + 5 * 60))
        );
        assert_eq!(
            arithmetic::expression("1 + 2", &state),
            Ok(Expression::Duration(Duration::seconds(3)))
        );
        assert_eq!(
            arithmetic::expression("1s + 2", &state),
            Ok(Expression::Timestamp(3))
        );
        assert_eq!(
            arithmetic::expression("1s - 2", &state),
            Ok(Expression::Timestamp(-1))
        );
        assert_eq!(
            arithmetic::expression("'2014-05-06 10:08:07' + '2014-05-06 10:08:07'", &state),
            Ok(Expression::Duration(Duration::seconds(d.timestamp() * 2)))
        );
        assert_eq!(
            arithmetic::expression("'2014/05/06 10:08:07' + 2", &state),
            Ok(Expression::Duration(Duration::seconds(d.timestamp() + 2)))
        );
        assert_eq!(
            arithmetic::expression("2 + (100 - 500)", &state),
            Ok(Expression::Timestamp(2 - 400))
        );
    }
    #[test]
    fn timestamps_to_durations() {
        let records = vec![];
        let state = State::new(FixedOffset::east(0), 0, &records);
        assert_eq!(
            arithmetic::expression("100 - 70", &state),
            Ok(Expression::Duration(Duration::seconds(30)))
        );
        assert_eq!(
            arithmetic::expression("100- 70", &state),
            Ok(Expression::Duration(Duration::seconds(30)))
        );
        assert_eq!(
            arithmetic::expression("100-70", &state),
            Ok(Expression::Duration(Duration::seconds(30)))
        );
        assert_eq!(
            arithmetic::expression("5 - 3 + 2h", &state),
            Ok(Expression::Duration(
                Duration::hours(2) + Duration::seconds(2)
            ))
        );
        assert_eq!(
            arithmetic::expression("(100 - 100) + 2h", &state),
            Ok(Expression::Duration(Duration::hours(2)))
        );
        assert_eq!(
            arithmetic::expression("2h + (100 - 100)", &state),
            Ok(Expression::Duration(Duration::hours(2)))
        );
        assert_eq!(
            arithmetic::expression("2h - (100 - 100)", &state),
            Ok(Expression::Duration(Duration::hours(2)))
        );
        assert_eq!(
            arithmetic::expression("2h - 100 + 100", &state),
            Ok(Expression::Duration(Duration::hours(2)))
        );
        assert_eq!(
            arithmetic::expression("(100 - 1s) - (100 + 1s)", &state),
            Ok(Expression::Duration(Duration::seconds(-2)))
        );
    }
    #[test]
    fn datetime_to_durations() {
        let records = vec![];
        let tz = FixedOffset::east(3600);
        let state = State::new(tz, 0, &records);
        let d = chrono::TimeZone::ymd(&tz, 2014, 5, 6).and_hms(20, 8, 7);
        assert_eq!(
            arithmetic::expression("'2014-05-06 20:08:07'", &state),
            Ok(Expression::Timestamp(d.timestamp())),
        );
        assert_eq!(
            arithmetic::expression(
                "'2014/05/06 18:08:07'",
                &State::new(FixedOffset::east(-3600), 0, &records)
            ),
            Ok(Expression::Timestamp(d.timestamp())),
        );
        assert_eq!(
            arithmetic::expression(
                "'2014-05-06T21:08:07'",
                &State::new(FixedOffset::east(2 * 3600), 0, &records)
            ),
            Ok(Expression::Timestamp(d.timestamp())),
        );
        assert_eq!(
            arithmetic::expression("'2014-05-06 20:08:05' + 2.0s", &state),
            Ok(Expression::Timestamp(d.timestamp())),
        );
        assert_eq!(
            arithmetic::expression("'2014-05-06 22:08:07' - 2h", &state),
            Ok(Expression::Timestamp(d.timestamp())),
        );
        assert_eq!(
            arithmetic::expression("'2014-05-06 20:10:07' - 2.0m", &state),
            Ok(Expression::Timestamp(d.timestamp())),
        );
        assert_eq!(
            arithmetic::expression("'2014-05-06 20:08:09' - '2014-05-06 10:08:09' + 2h", &state),
            Ok(Expression::Duration(Duration::hours(12)))
        );
        assert_eq!(
            arithmetic::expression("'2014-05-06 20:08:09' - '2014-05-06 10:08:09' + 2h", &state),
            Ok(Expression::Duration(Duration::hours(12)))
        );
        assert_eq!(
            arithmetic::expression(
                "'2014-05-06 10:08:07' + ('2013-05-06T20:08:09' - '2013-05-06 10:08:09')",
                &state
            ),
            Ok(Expression::Timestamp(d.timestamp())),
        );
    }
    #[test]
    fn now() {
        let records = vec![];
        let state = &State::new(FixedOffset::east(3600), 1, &records);
        assert_eq!(
            arithmetic::expression("now", &state),
            Ok(Expression::Timestamp(1))
        );
        assert_eq!(
            arithmetic::expression("now + 1m2s", &state),
            Ok(Expression::Timestamp(63))
        );
        assert_eq!(
            arithmetic::expression("now + 1", &state),
            Ok(Expression::Duration(Duration::seconds(2)))
        );

        let state = &State::new(FixedOffset::east(3600), 10, &records);
        assert_eq!(
            arithmetic::expression("now - 1", &state),
            Ok(Expression::Duration(Duration::seconds(9)))
        );
    }

    #[test]
    fn parsing_errors() {
        let records = vec![];
        let state = State::new(FixedOffset::east(3600), 10, &records);
        assert!(arithmetic::expression("3-", &state).is_err());
        assert_eq!(
            arithmetic::expression("'2014-25-06 10:08:07'", &state),
            Ok(Expression::None)
        );
        assert_eq!(
            arithmetic::expression("'2014-12-06 50:08:07'", &state),
            Ok(Expression::None)
        );
        assert_eq!(
            arithmetic::expression("'2014-12-06 00:08:70'", &state),
            Ok(Expression::None)
        );
        assert_eq!(
            arithmetic::expression("'2014-12-06 00:80:00'", &state),
            Ok(Expression::None)
        );
    }

    #[test]
    fn test_offset() {
        let input: String = "#UTC+1\n12323123\n'1970-05-23 16:05:23'".to_string();
        let records = parse(input, 1);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].offset, FixedOffset::east(0));
        assert_eq!(records[1].offset, FixedOffset::east(3600));
        assert_eq!(records[2].offset, FixedOffset::east(3600));
    }

    #[test]
    fn test_overflow() {
        let records = vec![];
        let state = State::new(FixedOffset::east(3600), 10, &records);
        assert!(arithmetic::expression("3-", &state).is_err());
        assert_eq!(
            arithmetic::expression("4324234034234234234039442343", &state),
            Ok(Expression::Timestamp(i64::MAX))
        );
        assert_eq!(
            arithmetic::expression("1 + 4324234034234234234039442343", &state),
            Ok(Expression::None)
        );
        assert_eq!(
            arithmetic::expression("1 - 4324234034234234234039442343", &state),
            Ok(Expression::None)
        );
    }
}
