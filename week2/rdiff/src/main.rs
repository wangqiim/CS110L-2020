use grid::Grid; // For lcs()
use std::env;
use std::fs::File; // For read_file_lines()
use std::io::{self, BufRead}; // For read_file_lines()
use std::process;
use std::cmp;

pub mod grid;

/// Reads the file at the supplied path, and returns a vector of strings.
fn read_file_lines(filename: &String) -> Result<Vec<String>, io::Error> {
    let file = File::open(filename)?;
    let mut lines: Vec<String> = Vec::new();
    for line in io::BufReader::new(file).lines() {
        let line_str = line?;
        lines.push(line_str);
    }
    Ok(lines)
}

fn lcs(seq1: &Vec<String>, seq2: &Vec<String>) -> Grid {
    let m = seq1.len() + 1;
    let n = seq2.len() + 1;
    let mut c = Grid::new(m, n);
    
    for i in 0..(m - 1) {
        for j in 0..(n - 1) {
            if seq1[i] == seq2[j] {
                c.set(i + 1, j + 1, c.get(i, j).unwrap() + 1).expect("must be Ok");
            } else {
                c.set(i + 1, j + 1, cmp::max(c.get(i + 1, j).unwrap(), c.get(i, j + 1).unwrap())).expect("must be Ok");
            }
        }
    }
    c
}

fn print_diff(lcs_table: &Grid, lines1: &Vec<String>, lines2: &Vec<String>, i: usize, j: usize) {
    if i > 0 && j > 0 && lines1[i - 1] == lines2[j - 1] {
        print_diff(lcs_table, lines1, lines2, i - 1, j - 1);
        println!(" {}", lines1[i - 1]);
    } else if j > 0 && (i == 0 || (lcs_table.get(i, j - 1).unwrap() >= lcs_table.get(i - 1, j).unwrap())) {
        print_diff(lcs_table, lines1, lines2, i, j - 1);
        println!("> {}", lines2[j - 1])
    } else if i > 0 && (j == 0 || (lcs_table.get(i, j - 1).unwrap() <= lcs_table.get(i - 1, j).unwrap())) {
        print_diff(lcs_table, lines1, lines2, i, j - 1);
        println!("> {}", lines1[i - 1])
    } else {
        println!("")
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename1 = &args[1];
    let filename2 = &args[2];

    let seq1 = read_file_lines(filename1).expect("read file1 error");
    let seq2 = read_file_lines(filename2).expect("read file2 error");

    let c = lcs(&seq1, &seq2);
    let i = seq1.len();
    let j = seq2.len();
    print_diff(&c, &seq1, &seq2, i, j);

}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_file_lines() {
        let lines_result = read_file_lines(&String::from("handout-a.txt"));
        assert!(lines_result.is_ok());
        let lines = lines_result.unwrap();
        assert_eq!(lines.len(), 8);
        assert_eq!(
            lines[0],
            "This week's exercises will continue easing you into Rust and will feature some"
        );
    }

    #[test]
    fn test_lcs() {
        let mut expected = Grid::new(5, 4);
        expected.set(1, 1, 1).unwrap();
        expected.set(1, 2, 1).unwrap();
        expected.set(1, 3, 1).unwrap();
        expected.set(2, 1, 1).unwrap();
        expected.set(2, 2, 1).unwrap();
        expected.set(2, 3, 2).unwrap();
        expected.set(3, 1, 1).unwrap();
        expected.set(3, 2, 1).unwrap();
        expected.set(3, 3, 2).unwrap();
        expected.set(4, 1, 1).unwrap();
        expected.set(4, 2, 2).unwrap();
        expected.set(4, 3, 2).unwrap();

        println!("Expected:");
        expected.display();
        let result = lcs(
            &"abcd".chars().map(|c| c.to_string()).collect(),
            &"adb".chars().map(|c| c.to_string()).collect(),
        );
        println!("Got:");
        result.display();
        assert_eq!(result.size(), expected.size());
        for row in 0..expected.size().0 {
            for col in 0..expected.size().1 {
                assert_eq!(result.get(row, col), expected.get(row, col));
            }
        }
    }
}