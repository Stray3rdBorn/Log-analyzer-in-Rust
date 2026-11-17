use clap::Parser;
use std::collections::HashMap;
use std::fs;

//command line arguments
#[dervice(Parser)]
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
}

// one parsed log line
struct Entry {
    method: String,
    path: String,
    status: u16,
    bytes: u64,
}

// turns on raw line into an entry, none if the line is empty
fn parse_line(line: &str) -> Option<Entry> {
    // split at every "
    let parts: Vec<&str> = line.split('"').collect();
    // if line didnt split in 3, return none
    if parts.len() < 3 {
        return None;
    }
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
        method: request.next()?.to_string(),
        path: request.next()?.to_string(),
        status: tail.next()?.parse().ok()?,
        bytes: tail.next()?.parse().unwrap_or(0),
    })
}