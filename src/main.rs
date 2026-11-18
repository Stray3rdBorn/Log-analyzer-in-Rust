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
        // appends the entry to the list
        self.largest.push(e);
        // largest bytes
        self.largest.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        // cuts the list down to first top items
        self.largest.truncate(top);
    }
}

// defines a function that prints summary stats from collected log data
fn report(stats: &Stats, skipped: usize, top: usize) {
    // adds up the counts of 4xx and 5xx error responses, defaulting to 0 if none exist
    let errors = stats.classes.get(&4).unwrap_or(&0) + stats.classes.get(&5).unwrap_or(&0);
    // prints the total number of processed requests
    println!("Total: {}", stats.total);
    // prints how many malformed or unparseable lines were skipped
    println!("Skipped Lines: {skipped}");
    // calculates and prints the error percentage formatted to one decimal place
    println!("Error Rate: {:.1}%", errors as f64 / stats.total as f64 * 100.0);

    // collects status code classes (e.g., 2, 4, 5) and their counts into a vector
    let mut classes: Vec<_> = stats.classes.iter().collect();
    // sorts the status code classes in ascending numerical order
    classes.sort();
    // loops through each sorted status class and its count
    for (class, count) in classes {
        // prints the status class (like "2xx") alongside its total count
        println!("{class}xx: {count}");
    }

    // collects endpoint paths and their request counts into a vector
    let mut paths: Vec<_> = stats.endpoints.iter().collect();
    // sorts the endpoints by count in descending order (highest traffic first)
    paths.sort_by(|a, b| b.1.cmp(a.1));
    // prints a section header for the most requested endpoints
    println!("Top Endpoints:");
    // loops through only the top N endpoints specified by the `top` parameter
    for (path, count) in paths.iter().take(top) {
        // prints the request count right-aligned to 6 characters followed by the path
        println!("{count:>6}  {path}");
    }

    // prints a section header for the largest recorded responses
    println!("Largest Responses:");
    // loops through each entry in the list of largest responses
    for e in &stats.largest {
        // prints the response size right-aligned to 8 characters, followed by the HTTP method and path
        println!("{:>8} bytes  {} {}", e.bytes, e.method, e.path);
    }
}


// true when the entry passes every filter flag
// evaluation stops at the first false
// the last expression has no ; so its the return value
fn passes(e: &Entry, args: &Args) -> bool {
    args.status.is_none_or(|s| e.status / 100 == s)
        && args.method.as_ref().is_none_or(|m| e.method.eq_ignore_ascii_case(m))
        && args.from.is_none_or(|f| e.time >= f)
        && args.to.is_none_or(|t| e.time <= t)
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