use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

pub fn yes_no(question: &str, default_yes: bool) -> Result<bool> {
    if let Ok(tty) = OpenOptions::new().read(true).write(true).open("/dev/tty") {
        return yes_no_on_tty(tty, question, default_yes);
    }

    eprint!("{question}");
    std::io::stderr().flush().ok();
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("failed to read prompt response")?;
    Ok(parse_yes_no(&input, default_yes))
}

fn yes_no_on_tty(mut tty: std::fs::File, question: &str, default_yes: bool) -> Result<bool> {
    write!(tty, "{question}").context("failed to write prompt")?;
    tty.flush().context("failed to flush prompt")?;
    let mut reader = BufReader::new(tty.try_clone().context("failed to clone tty")?);
    let mut input = String::new();
    reader
        .read_line(&mut input)
        .context("failed to read prompt response")?;
    Ok(parse_yes_no(&input, default_yes))
}

fn parse_yes_no(input: &str, default_yes: bool) -> bool {
    let value = input.trim().to_ascii_lowercase();
    if value.is_empty() {
        return default_yes;
    }
    matches!(value.as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_yes_no_answers_with_default() {
        assert!(parse_yes_no("", true));
        assert!(!parse_yes_no("", false));
        assert!(parse_yes_no("yes", false));
        assert!(parse_yes_no("Y", false));
        assert!(!parse_yes_no("no", true));
    }
}
