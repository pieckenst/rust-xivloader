use std::io::{self, Write, Read};
use std::process::Command;
use regex::Regex;

fn read_password() -> io::Result<String> {
    let mut password = String::new();
    loop {
        let mut input = [0];
        io::stdin().read_exact(&mut input)?;
        match input[0] as char {
            '\n' => break,
            '\x08' => {
                password.pop();
                print!("\x08 \x08");
            }
            c => {
                password.push(c);
                print!("*");
            }
        }
        io::stdout().flush()?;
    }
    println!();
    Ok(password)
}

fn main() {
    Command::new("cmd")
            .args(&["/C", "title XIVLOADER"])
            .status()
            .expect("failed to execute process");

    let arr = [
        "                                             ",
        " __  _______   ___                 _         ",
        " \\ \\/ /_ _\\ \\ / / |   ___  __ _ __| |___ _ _ ",
        "  >  < | | \\ V /| |__/ _ \\/ _` / _` / -_) '_| ",
        " /_/\\_\\___| \\_/ |____\\___/\\__,_\\__,_\\___|_|  ",
        "                                             ",
    ];

    println!("\n\n");
    for line in arr.iter() {
        println!("{}", line);
    }

    println!("0 - Japanese , 1 - English , 2 - German , 3 - French , 4 - Russian ( The client will still be in english)");
    print!("Enter your language - ");

    let mut language = String::new();
    io::stdin().read_line(&mut language).expect("Failed to read line");
    let language: i32 = language.trim().parse().expect("Invalid input");

    match language {
        0 => japan_launch(),
        1 => english_launch(),
        2 => german_launch(),
        3 => french_launch(),
        4 => russian_launch(),
        _ => println!("Invalid language selection."),
    }
}

fn japan_launch() {
    // Implement Japan launch method
}

fn english_launch() {
    // Implement English launch method
}

fn german_launch() {
    // Implement German launch method
}

fn french_launch() {
    // Implement French launch method
}

fn russian_launch() {
    // Implement Russian launch method
}
