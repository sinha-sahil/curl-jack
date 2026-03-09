use std::io::{self, BufRead, IsTerminal, Write};

/// Check whether the input is complete: all quotes are balanced and
/// the line doesn't end with a backslash continuation (outside quotes).
fn input_complete(s: &str) -> bool {
    let trimmed = s.trim_end();
    let mut in_single = false;
    let mut in_double = false;
    let mut ends_with_continuation = false;

    let mut chars = trimmed.chars().peekable();
    while let Some(ch) = chars.next() {
        ends_with_continuation = false;
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '\\' if !in_single && !in_double => {
                if chars.peek().is_none() {
                    ends_with_continuation = true;
                } else {
                    chars.next(); // skip escaped char
                }
            }
            _ => {}
        }
    }

    !in_single && !in_double && !ends_with_continuation
}

fn prompt(s: &str, is_tty: bool) {
    if is_tty {
        print!("{s}");
        let _ = io::stdout().flush();
    }
}

fn main() {
    let json_mode = std::env::args().any(|a| a == "--json");
    let is_tty = io::stdin().is_terminal();

    if is_tty {
        println!("curl-jack parser (type \"quit\" or \"exit\" to stop)");
        if json_mode {
            println!("  (JSON output mode)");
        }
        println!();
    }

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        prompt("curl> ", is_tty);

        let mut input = String::new();
        loop {
            let line = match lines.next() {
                Some(Ok(line)) => line,
                _ => {
                    if is_tty {
                        println!();
                    }
                    return;
                }
            };

            if input.is_empty() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if trimmed == "quit" || trimmed == "exit" {
                    if is_tty {
                        println!("Bye!");
                    }
                    return;
                }
            }

            input.push_str(&line);
            input.push('\n');

            if input_complete(&input) {
                break;
            }

            prompt("   > ", is_tty);
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match curl_jack::parse(input) {
            Ok(request) => {
                if json_mode {
                    match serde_json::to_string_pretty(&request) {
                        Ok(json) => println!("{json}"),
                        Err(e) => eprintln!("Serialization error: {e}"),
                    }
                } else {
                    print!("{request}");
                }
            }
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
        println!();
    }
}
