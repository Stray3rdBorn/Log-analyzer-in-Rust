use clap::Parser;
use std::collections::HashMap;
use chrono::{DateTime, FixedOffset};
use std::fs::File;
use std::io::{BufRead, BufReader};

//command line arguments
#[derive(Parser)]
struct Args {
    // path of the log file
    file: String,
    // keeps only one status class, 5 means 5xx
    #[arg(long)]
    status: Option<u16>,
    // keeps only one HTTP method
     #[arg(long)]
    method: Option<String>,
    // number of rows in each top list
    #[arg(long, default_value_t = 5)]
    top: usize,
    // keeps entries at or after this time, RFC 3339 like 2026-11-17T10:00:00+06:00
    #[arg(long, value_parser = DateTime::parse_from_rfc3339)]
    from: Option<DateTime<FixedOffset>>,
    // keeps entries at or before this time
    #[arg(long, value_parser = DateTime::parse_from_rfc3339)]
    to: Option<DateTime<FixedOffset>>,

    // stores the moment of request 
    time: DateTime<FixedOffset>,
}

// one parsed log line
struct Entry {
    method: String,
    path: String,
    status: u16,
    bytes: u64,
}

// drops control characters so log text cannot change what the terminal shows
fn clean(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
    // chars one character at a time
    // filter keeps only characters when return is true
    // c.is-control so only non control characters stay
    // collect packs all charas into a string
}

// turns on raw line into an entry, none if the line is empty
fn parse_line(line: &str) -> Option<Entry> {
    // split at every "
    let parts: Vec<&str> = line.split('"').collect();
    // if line didnt split in 3, return none
    if parts.len() < 3 {
        return None;
    }
    // holds timestamp
     let stamp = parts[0].split('[').nth(1)?.split(']').next()?;
    // holds the web request like method, path/URL
    let mut request =  parts[1].split_whitespace();
    // holds the server's resposne numbers like status code, byte size
    let mut tail = parts[2].split_whitespace();
    // packs the finished box
    Some(Entry {
        // next grabs the next word, ? aborts if empty
        // to_string makes a permanent copy of the word so Entry box can own it
        // parse turns text into numbers, ok converts any error into a none
        // unwrap(0) means unknown byte size is read as 0 so it doesn't stop 
        method: clean(request.next()?), // replaces to_string to copy text into string and remove control charas
        path: clean(request.next()?),
        status: tail.next()?.parse().ok()?,
        bytes: tail.next()?.parse().unwrap_or(0),
        // reads a date from text using a pattern
        time: DateTime::parse_from_str(stamp, "%d/%b/%Y:%H:%M:%S %z").ok()?,
    })
}

// running totals filled while lines are read
#[derive(Default)] // auto writes Stats::default(), which gives zero and empty collections
struct Stats { // holds the counters 
    total: usize,
    classes: HashMap<u16, usize>,
    endpoints: HashMap<String, usize>,
    hours: HashMap<u32, usize>,
    largest: Vec<Entry>,
}

impl Stats { // attaches functions to the type
    // folds one entry into the running totals
    // the method receives the Stats it was called on and may change it
    fn add(&mut self, e: Entry, top: usize) {
        self.total += 1;
        // path clone makes a separate copy of the text
        // time hour gives the hour from 0 to 23
        *self.classes.entry(e.status / 100).or_insert(0) += 1;
        *self.endpoints.entry(e.path.clone()).or_insert(0) += 1;
        *self.hours.entry(e.time.hour()).or_insert(0) += 1;
        // appends the entry to the list
        self.largest.push(e);
        // largest bytes
        self.largest.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        // cuts the list down to first top items
        self.largest.truncate(top);
    }
}

// prints every stats for the filtered entries
fn report(entries: &[Entry], top: usize) { // read-only window into the list of log entries
    // count number of valid log entries
    let total = entries.len();
    // tracks status code & number of times appeared
    let mut classes: HashMap<u16, usize> = HashMap::new();
    // tracks website URLs & number of times visited 
    let mut endpoints: HashMap<&str, usize> = HashMap::new();
    for e in entries {
        // whole-number division chops off the last two digits, 404 / 100 equals 4
        *classes.entry(e.status / 100).or_insert(0) += 1;
        // hands over a temporary, read-only view of the URL string
        *endpoints.entry(e.path.as_str()).or_insert(0) += 1;
    }

    // client and server errors count as errors
    // looks up how many 4 & 5 were tailed, if 0 uses 0 instead of crashing
    let errors = classes.get(&4).unwrap_or(&0) + classes.get(&5).unwrap_or(&0);
    println!("Total: {total}");
    println!("Error rate: {:.1}%", errors as f64 / total as f64 * 100.0);

    // sorting status classes smallest to largest
    let mut class_list: Vec<_> = classes.into_iter().collect();
    class_list.sort();
    for (class, count) in class_list  {
        println!("{class}xx: {count}");
    }

    // sorting top URLs biggest to smallest
    let mut top_paths: Vec<_> = endpoints.into_iter().collect();
    top_paths.sort_by(|a, b| b.1.cmp(&a.1));
    println!("Top endpoints: ");
    for (path, count) in top_paths.iter().take(top) { // grabs only the top results
        println!("{count:>6} {path}"); // padding number 6
    }

    // sorting by byte size, largest to smallest
    let mut largest: Vec<&Entry> = entries.iter().collect();
    largest.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    println!("Largest response: ");
    for e in largest.iter().take(top) {
        println!("{:>8} bytes {} {}", e.bytes, e.method, e.path);
    }
}

fn main() -> Result<(), std::io::Error> {
    let args = Args::parse(); // reads commands in terminal 
    let text = fs::read_to_string(&args.file)?; // puts all texts into one string

    let entries: Vec<Entry> = text
        .lines() // splits string into individual lines
        .filter_map(parse_line) // feeds raw lines to parse_line
        .filter(|e| args.status.is_none_or(|s| e.status / 100 == s)) // checks valind entry
        .filter(|e| {
            args.method
                .as_ref() // looks at method text
                .is_none_or(|m| e.method.eq_ignore_ascii_case(m)) // ignores letter case
        })
        .collect(); // pulls every entry into a final list
    
    // to prevent a divide by 0 crash later if it returns ok
    if entries.is_empty() {
        println!("no matching entries");
        return Ok(());
    }
    // puts filtered list to report for printing
    report(&entries, args.top);
    Ok(())
}