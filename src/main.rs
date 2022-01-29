use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use std::time::Duration;
use web_sys::HtmlInputElement;
use yew::{html, Component, Context, Html, InputEvent, TargetCast};

fn now() -> Option<DateTime<Utc>> {
    let duration = Duration::from_secs_f64(stdweb::web::Date::now() / 1000.0);
    NaiveDateTime::from_timestamp_opt(duration.as_secs() as i64, 0)
        .map(|naive_date_time| DateTime::from_utc(naive_date_time, Utc))
}

fn get_time_zone(text: &str) -> FixedOffset {
    let re = Regex::new(r"^#UTC([+-])(\d{1,2})$").unwrap();
    match re.captures(text.trim()) {
        Some(x) => {
            if x.len() != 3 {
                return FixedOffset::east(0);
            }
            let sign = &x[1];
            let value = match (&x[2]).parse::<i32>().unwrap_or(0) {
                x @ -23..=23 => x * 3600,
                _ => 0,
            };
            match sign {
                "+" => FixedOffset::east(value),
                "-" => FixedOffset::east(-value),
                _ => FixedOffset::east(0),
            }
        }
        _ => FixedOffset::east(0),
    }
}

fn parse(input: &str) -> Record {
    if input.eq_ignore_ascii_case("now") {
        return Record::new(now());
    }
    let timestamp = match input.parse::<i64>() {
        Ok(r) => Some(r),
        Err(_) => None,
    };
    let date_time_2 = match Utc.datetime_from_str(input, "%Y-%m-%d %H:%M:%S") {
        Ok(r) => Some(r),
        Err(_) => None,
    };
    let naive_date_time = match timestamp {
        Some(timestamp) => NaiveDateTime::from_timestamp_opt(timestamp, 0),
        _ => None,
    };
    let date_time: Option<DateTime<Utc>> =
        naive_date_time.map(|naive_date_time| DateTime::from_utc(naive_date_time, Utc));

    let date_time = match date_time {
        Some(date_time) => Some(date_time),
        _ => date_time_2,
    };
    Record::new(date_time)
}

// .format("%Y-%m-%d %H:%M:%S").to_string()

enum Msg {
    InputValue(String),
}

struct Record {
    datetime: Option<DateTime<Utc>>,
    offset: FixedOffset,
}

impl Record {
    fn new(datetime: Option<DateTime<Utc>>) -> Self {
        Self {
            datetime,
            offset: FixedOffset::east(0),
        }
    }
    fn empty() -> Self {
        Self {
            datetime: None,
            offset: FixedOffset::east(0),
        }
    }
    fn to_datetime_string(&self) -> String {
        match self.datetime {
            Some(datetime) => datetime.with_timezone(&self.offset).to_string(),
            _ => "...".to_string(),
        }
    }

    fn to_timestamp_string(&self) -> String {
        match self.datetime {
            Some(datetime) => datetime.timestamp().to_string(),
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
                let mut records = vec![];
                let mut offset = FixedOffset::east(0);
                let split = input.split('\n');
                for s in split {
                    let mut record = parse(s);
                    record.offset = offset;
                    if record.datetime == None {
                        offset = get_time_zone(s);
                    }
                    records.push(record);
                }
                self.records = records;
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
                            <a href="https://github.com">
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
