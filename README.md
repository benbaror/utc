# [UTC - Unix Time Calculator](https://benbaror.github.io/utc/)

Each line of input is parsed and displayed as both a human-readable datetime and a Unix timestamp.

### Syntax

| Input | Meaning |
|---|---|
| `1748000000` | Unix timestamp (seconds) → datetime |
| `-1748000000` | Negative timestamp |
| `now` | Current time |
| `2024-01-15 12:00:00` | Datetime → timestamp (uses active timezone) |
| `2024-01-15 12:00:00 +05:00` | Datetime with UTC offset (paste directly from middle panel) |
| `'2024-01-15T12:00:00'` | ISO 8601 format (quotes required) |
| `'2024/01/15 12:00:00'` | Slash-separated date (quotes required) |
| `2h30m`, `1.5d`, `90s`, `500ms` | Duration (d h m s ms) |
| `now - 7d` | Arithmetic with `+` and `-` |
| `'2024-06-01 00:00:00' + 30d` | Add duration to datetime |
| `#2 - #1` | Reference line by number |
| `#UTC+5`, `#UTC-8` | Set timezone offset for all lines below |

**JSON pasting:** JSON keys are stripped automatically, so you can paste `{"ts": 1748000000}` directly.

**Durations** can be combined without spaces: `4h30m20s`, `1d12h`.

**Line references** (`#N`) use the timestamp value of line N. Combined with a timezone header this lets you convert between zones:
```
#UTC+9
1748000000
#UTC-5
#1
```

### How to run

Install [the webassembly target](https://yew.rs/docs/getting-started/introduction#install-webassembly-target) and [trunk](https://trunkrs.dev/#install).

Run `trunk serve` and open http://localhost:8080 in your browser.

### Contributing

Before creating a pull request run the sanity checks:

```
cargo fmt
cargo clippy -- -D warnings
cargo check
cargo test
trunk build
```
