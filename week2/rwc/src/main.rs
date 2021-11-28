use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::process;

/// Reads the file at the supplied path, and returns a vector of strings.
fn read_file_words(filename: &String) -> Result<Vec<String>, io::Error> {
    let file = File::open(filename)?;
    let mut words: Vec<String> = Vec::new();
    for line in io::BufReader::new(file).lines() {
        let line_str = line?;
        let v: Vec<&str> = line_str.split(' ').collect();
        for s in v.iter() {
            words.push(String::from(*s));
        }
    }
    Ok(words)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    let seq = read_file_words(filename).expect("read file error");
    
    let word_count = seq.len();
    println!("{}", word_count);
}
