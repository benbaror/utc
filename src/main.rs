use std::fmt::{self, Display};

use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime};
use parser::Expression;
use thiserror::Error;
use web_sys::HtmlInputElement;
use yew::{html, Component, Context, Html, InputEvent, TargetCast};
mod parser;

fn now() -> i64 {
    (stdweb::web::Date::now() / 1000.0) as i64
}

fn parse(input: &str, now: i64) -> Vec<Record> {
    let records = parser::parse(input, now);
    records.iter().map(std::convert::Into::into).collect()
}

enum Msg {
    InputValue(String),
    CopyToClipboard,
}

#[non_exhaustive]
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
        if *self == Self::seconds(0) {
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
            _ => Self::None,
        }
    }

    const fn duration(duration: Duration) -> Self {
        Self::Duration(duration)
    }

    const fn empty() -> Self {
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

#[derive(Error, Debug)]
pub enum ClipboardError {
    #[error("Clipboard API not available")]
    NotAvailable,
    #[error("Could not write text to clipboard")]
    Write,
}

struct Container {
    records: Vec<Record>,
    input: String,
}

impl Display for Container {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let input_lines = self.input.split('\n').map(|s| s.trim());
        let max_length = input_lines.clone().map(|s| s.len()).max().unwrap_or(0);
        let text = input_lines
            .zip(
                self.records
                    .iter()
                    .map(|record| record.to_datetime_string()),
            )
            .map(|(input, record)| format!("{input:max_length$} {record}"))
            .collect::<Vec<_>>()
            .join("\n");
        write!(f, "{text}")
    }
}

impl Container {
    #[cfg(web_sys_unstable_apis)]
    fn copy_to_clipboard(&self) -> Result<(), ClipboardError> {
        use wasm_bindgen_futures::JsFuture;

        let window = web_sys::window().ok_or(ClipboardError::NotAvailable)?;
        let clipboard = window
            .navigator()
            .clipboard()
            .ok_or(ClipboardError::NotAvailable)?;
        let promise = clipboard.write_text(&self.to_string());
        wasm_bindgen_futures::spawn_local(async {
            JsFuture::from(promise).await;
        });
        Ok(())
    }

    #[cfg(not(web_sys_unstable_apis))]
    fn copy_to_clipboard(&self) -> Result<(), ClipboardError> {
        Err(ClipboardError::NotAvailable)
    }
}

impl Component for Container {
    type Message = Msg;
    type Properties = ();

    fn create(_context: &Context<Self>) -> Self {
        let record = Record::empty();
        Self {
            records: vec![record],
            input: "".to_string(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::InputValue(input) => {
                self.records = parse(&input, now());
                let input_lines: Vec<_> = input.split('\n').map(|s| s.trim_start()).collect();
                self.input = input_lines.join("\n");
                true
            }
            Msg::CopyToClipboard => match self.copy_to_clipboard() {
                Ok(()) => true,
                Err(_) => false,
            },
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let title = "Unix Time Calculator".to_string();
        let link = ctx.link();

        let on_input = link.callback(|e: InputEvent| {
            Msg::InputValue(e.target_unchecked_into::<HtmlInputElement>().value())
        });

        let copy_to_clipboard = link.callback(|_| Msg::CopyToClipboard);

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
                                    for (1..=self.records.len()).map(|i| {
                                        html!{
                                            <div>{i}</div>
                                        } })
                                    }
                                </div>
                            </div>
                            <div class="input-text">
                                <textarea
                                oninput={on_input}
                                value={self.input.clone()}
                                class="input-textarea"
                                style="resize: none"
                                data-gramm="false"
                                placeholder=""
                                wrap = "off"
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
                            <button class="btn" onclick={copy_to_clipboard}><i class="fa-solid clipboard"></i></button>
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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test() {
        let input: String = "#UTC+1\n12323123\n'1970-05-23 16:05:23'".to_string();
        let records = parse(&input, 1);
        assert_eq!(records.len(), 3);
        let container = Container { records, input };
        assert_eq!(
            container.to_string(),
            concat!(
                "#UTC+1                UTC+01:00\n",
                "12323123              1970-05-23 16:05:23 +01:00\n",
                "'1970-05-23 16:05:23' 1970-05-23 16:05:23 +01:00"
            ),
        );
    }
}
