extern crate peg;
use chrono::{Duration, FixedOffset, LocalResult, TimeZone};
use peg::parser;
use std::ops::{Add, Sub};

#[derive(Clone, PartialEq, Debug)]
pub enum Expression {
    Duration(Duration),
    Timestamp(i64),
    None,
}

impl Add<Expression> for Expression {
    type Output = Expression;

    fn add(self, rhs: Expression) -> Expression {
        match (self, rhs) {
            (Expression::Duration(l), Expression::Duration(r)) => Expression::Duration(l + r),
            (Expression::Duration(l), Expression::Timestamp(r)) => {
                Expression::Timestamp(l.num_seconds() + r)
            }
            (Expression::Timestamp(l), Expression::Duration(r)) => {
                Expression::Timestamp(l + r.num_seconds())
            }
            (Expression::Timestamp(l), Expression::Timestamp(r)) => {
                Expression::Duration(Duration::seconds(l + r))
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
                Expression::Timestamp(l.num_seconds() - r)
            }
            (Expression::Timestamp(l), Expression::Duration(r)) => {
                Expression::Timestamp(l - r.num_seconds())
            }
            (Expression::Timestamp(l), Expression::Timestamp(r)) => {
                Expression::Duration(Duration::seconds(l - r))
            }
            _ => Expression::None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct State<'a> {
    offset: i32,
    now: i64,
    records: &'a [Expression],
}

impl<'a> State<'a> {
    pub fn new(offset: i32, now: i64, records: &'a [Expression]) -> Self {
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

    rule record() -> Expression = "#" + r:$(['0'..='9']+) {
        let record: usize = r.parse().unwrap();
        match state.records.get(record - 1) {
            Some(v) => v.clone(),
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
            let tz = FixedOffset::east(state.offset * 3600);
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

#[test]
fn durations() {
    let records = vec![];
    let state = State::new(0, 0, &records);
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
    let state = State::new(5, 0, &records);
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
    let state = State::new(0, 0, &records);
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
    let state = State::new(1, 0, &records);
    let tz = FixedOffset::east(3600);
    let d = chrono::TimeZone::ymd(&tz, 2014, 5, 6).and_hms(20, 8, 7);
    assert_eq!(
        arithmetic::expression("'2014-05-06 20:08:07'", &state),
        Ok(Expression::Timestamp(d.timestamp())),
    );
    assert_eq!(
        arithmetic::expression("'2014/05/06 18:08:07'", &State::new(-1, 0, &records)),
        Ok(Expression::Timestamp(d.timestamp())),
    );
    assert_eq!(
        arithmetic::expression("'2014-05-06T21:08:07'", &State::new(2, 0, &records)),
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
    let state = &State::new(1, 1, &records);
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

    let state = &State::new(1, 10, &records);
    assert_eq!(
        arithmetic::expression("now - 1", &state),
        Ok(Expression::Duration(Duration::seconds(9)))
    );
}

#[test]
fn parsing_errors() {
    let records = vec![];
    let state = &State::new(1, 10, &records);
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
