# Log Analyzer

A small command line tool I built in Rust to read an Nginx access log and print a summary of it. I made it to learn Rust fundamentals on a real problem: parsing text, counting things, and handling bad input safely.

## Features

- Parses the Nginx common log format
- Reports total requests, error rate (4xx and 5xx), and counts per status class
- Lists the top endpoints and the largest responses
- Filters by status class, HTTP method, and a time range
- Counts malformed lines instead of silently dropping them
- Reads the file line by line, so large logs do not fill memory

## Requirements

Rust 1.82 or newer.

## Installation

```
git clone https://github.com/Stray3rdBorn/log-analyzer.git
cd log-analyzer
cargo build --release
```

## Usage

```
cargo run -- <FILE> [OPTIONS]
```

| Option          | Meaning                                    | Example                            |
| --------------- | ------------------------------------------ | ---------------------------------- |
| `--status <N>`  | Only one status class, `5` means 5xx       | `--status 5`                       |
| `--method <M>`  | Only one HTTP method, case does not matter | `--method post`                    |
| `--from <TIME>` | Only requests at or after this time        | `--from 2026-11-18T10:00:00+06:00` |
| `--to <TIME>`   | Only requests at or before this time       | `--to 2026-11-18T10:30:00+06:00`   |
| `--top <N>`     | Rows in each top list, default 5           | `--top 3`                          |

Times use RFC 3339 with a timezone offset. The offset is compared properly, so `10:00+06:00` and `04:00+00:00` count as the same moment.

Try it on the included sample:

```
cargo run -- sample.log
cargo run -- sample.log --status 5
cargo run -- sample.log --method post --top 3
cargo run -- sample.log --from 2026-11-18T10:00:00+06:00 --to 2026-11-18T10:30:00+06:00
```

## Example output

```
total: 71
skipped lines: 3
error rate: 18.3%
2xx: 54
3xx: 4
4xx: 4
5xx: 9
top endpoints:
    22  /api/posts
    17  /api/users
     8  /index.html
     7  /api/login
     7  /api/orders
largest responses:
   91536 bytes  GET /static/app.js
   88112 bytes  GET /static/app.js
   24278 bytes  GET /api/posts
   23974 bytes  GET /api/posts
   21804 bytes  GET /api/posts
```

## Security and robustness

- Bad input never crashes the tool. Lines that do not match the format are skipped and counted.
- Control characters are stripped from printed paths and methods, so a hostile log line cannot send terminal escape codes.
- Invalid `--from` and `--to` values are rejected before the program runs.
- No division by zero when nothing matches the filters.
- Memory use stays small because only running totals are kept, not every line.

Known limits:

- A log with millions of unique paths can still grow the endpoint counter.
- A line containing invalid UTF-8 stops the run.
