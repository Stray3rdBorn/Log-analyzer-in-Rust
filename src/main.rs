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
    let errors = classes.get(&4).unwrap_or(0) + classes.get(&5).unwrap_or(&0);
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
    for 0 in largest.iter().take(top) {
        println!("{:>8} bytes {} {}", e.bytes, e.method, e.path);
    }
}