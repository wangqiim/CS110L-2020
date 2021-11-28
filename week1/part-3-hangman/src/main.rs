// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn vec2str(v: &Vec<char>) -> String {
    let mut s = String::new();
    for c in v {
        s.push(*c);
    }
    s
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    println!("[debug] random word: {}", secret_word);
    
    let mut so_far_word = vec!['-'; secret_word_chars.len()];
    let mut guessed_letters = String::new();
    let mut used_guessed = 0;
    let mut guessd_count = 0;
    println!("Welcome to CS110L Hangman!");
    while NUM_INCORRECT_GUESSES - used_guessed != 0 && guessd_count != secret_word_chars.len() {
        println!("The word so far is {}", vec2str(&so_far_word));
        println!("You have guessed the following letters: {}", guessed_letters);
        println!("You have {} guesses left", NUM_INCORRECT_GUESSES - used_guessed);
        print!("Please guess a letter: ");
        io::stdout().flush().expect("Error flushing stdout.");
        let mut guess = String::new();
        io::stdin().read_line(&mut guess).expect("Error reading line.");

        guessed_letters.push(guess.chars().nth(0).unwrap());
        let mut right_letter = false;
        let mut i = 0;
        while i < secret_word_chars.len() {
            if secret_word_chars[i] == guess.chars().nth(0).unwrap() && so_far_word[i] =='-' {
                so_far_word[i] = secret_word_chars[i];
                right_letter = true;
                guessd_count += 1;
                break;
            }
            i += 1;
        }
        if !right_letter {
            used_guessed += 1;
        }
        println!();
    }

    if NUM_INCORRECT_GUESSES - used_guessed == 0 {
        println!("Sorry, you ran out of guesses!")
    } else {
        println!("Congratulations you guessed the secret word: {}!", secret_word);
    }
}
