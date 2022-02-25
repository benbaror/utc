use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime};
use parser::Expression;
use web_sys::HtmlInputElement;
use yew::{html, Component, Context, Html, InputEvent, TargetCast};
mod parser;

fn now() -> i64 {
    (stdweb::web::Date::now() / 1000.0) as i64
}

fn parse(input: String, now: i64) -> Vec<Record> {
    let records = parser::parse(input, now);
    records.iter().map(|record| record.into()).collect()
}

enum Msg {
    InputValue(String),
}

enum Record {
    DateTime(DateTime<FixedOffset>),
    Duration(Duration),
    Offset(FixedOffset),
    None,
}

impl From<&parser::Record> for Record {
    fn from(record: &parser::Record) -> Self {
        match record.expression {
            Expression::Timestamp(t) => Self::timestamp(t, record.offset),
            Expression::Duration(d) => Self::duration(d),
            Expression::Offset(offset) => Self::Offset(offset),
            _ => Self::None,
        }
    }
}

pub trait ToFormattedString {
    fn to_fmt_string(&self) -> String;
}

impl ToFormattedString for Duration {
    fn to_fmt_string(&self) -> String {
        if *self == Duration::seconds(0) {
            return "0s".to_string();
        }
        let (abs, sign) = if self.num_milliseconds() < 0 {
            (-*self, "-")
        } else {
            (*self, "")
        };
        let days = abs.num_days();
        let hours = abs.num_hours() - days * 24;
        let minutes = abs.num_minutes() - days * 24 * 60 - hours * 60;
        let seconds = abs.num_seconds() - days * 24 * 60 * 60 - hours * 60 * 60 - minutes * 60;
        let milliseconds = abs.num_milliseconds()
            - days * 24 * 60 * 60 * 1000
            - hours * 60 * 60 * 1000
            - minutes * 60 * 1000
            - seconds * 1000;
        let mut string = sign.to_string();
        if days > 0 {
            string = format!("{string}{}d", days);
        }
        if hours > 0 {
            string = format!("{string}{}h", hours);
        }
        if minutes > 0 {
            string = format!("{string}{}m", minutes);
        }
        if seconds > 0 {
            string = format!("{string}{}s", seconds);
        }
        if milliseconds > 0 {
            string = format!("{string}{}ms", milliseconds);
        }
        string
    }
}

impl Record {
    fn timestamp(timestamp: i64, offset: FixedOffset) -> Self {
        let naive_date_time = NaiveDateTime::from_timestamp_opt(timestamp, 0);
        match naive_date_time {
            Some(d) => Self::DateTime(DateTime::from_utc(d, offset)),
            _ => Record::None,
        }
    }

    fn duration(duration: Duration) -> Self {
        Self::Duration(duration)
    }

    fn empty() -> Self {
        Self::None
    }

    fn to_datetime_string(&self) -> String {
        match self {
            Self::DateTime(datetime) => datetime.to_string(),
            Self::Duration(duration) => duration.to_fmt_string(),
            Self::Offset(offset) => format!("UTC{}", offset),
            _ => "...".to_string(),
        }
    }

    fn to_timestamp_string(&self) -> String {
        match self {
            Self::DateTime(datetime) => datetime.timestamp().to_string(),
            Self::Duration(duration) => (duration.num_milliseconds() as f64 / 1000.).to_string(),
            Self::Offset(offset) => format!("UTC{}", offset),
            _ => "...".to_string(),
        }
    }
}

struct Container {
    records: Vec<Record>,
}

impl Component for Container {
    type Message = Msg;
    type Properties = ();

    fn create(_context: &Context<Self>) -> Self {
        let record = Record::empty();
        Self {
            records: vec![record],
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::InputValue(input) => {
                self.records = parse(input, now());
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let title = "Unix Time Calculator".to_string();
        let link = ctx.link();

        let on_input = link.callback(|e: InputEvent| {
            Msg::InputValue(e.target_unchecked_into::<HtmlInputElement>().value())
        });

        return html! {
            <div>
                <div class="page">
                    <div class="banner">
                        <h1 class="title">{ title }</h1>
                        <div class="github">
                            <a href="https://github.com/benbaror/utc">
                            <svg width="24" height="24" viewBox="0 0 16 16" fill="currentColor"><path fill-rule="evenodd" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path></svg>
                            </a>
                        </div>
                    </div>

                    <div class="app">
                        <div class="container">
                            <div class="line-number">
                                <div> {
                                    for (1..self.records.len() + 1).map(|i| {
                                        html!{
                                            <div>{i}</div>
                                        } })
                                    }
                                </div>
                            </div>
                            <div class="input-text">
                                <textarea
                                oninput={on_input}
                                class="input-textarea"
                                style="resize: none"
                                data-gramm="false"
                                placeholder=""
                                >
                                </textarea>
                            </div>
                            <div class="date-format">
                                <div> {
                                    for self.records.iter().map(|v| {
                                        html!{
                                            <div>{ v.to_datetime_string() }</div>
                                        } })
                                    }
                                </div>
                            </div>
                            <div class="timestamp">
                                <div> {
                                    for self.records.iter().map(|v| {
                                        html!{
                                            <div>{ v.to_timestamp_string() }</div>
                                        } })
                                    }
                                </div>
                            </div>
                        </div>

                    </div>
                </div>
            </div>
        };
    }
}

fn main() {
    yew::start_app::<Container>();
}

#[test]
fn test() {
    let input: String = "#UTC+1\n12323123\n'1970-05-23 16:05:23'".to_string();
    let records = parse(input, 1);
    assert_eq!(records.len(), 3)
}
