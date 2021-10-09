use lazy_static::lazy_static;
use regex::Regex;

// Capitalizes the first letter of a str.
pub fn capitalize_first(input: &str) -> String {
	let mut c = input.chars();
	match c.next() {
		None => String::new(),
		Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
	}
}

pub fn format_seconds(seconds: u64) -> String {
	let d = seconds / 86_400;
	let h = seconds / 3600 % 24;
	let m = seconds % 3600 / 60;
	let s = seconds % 3600 % 60;
	let mut output = format!("{}s", s);
	if m != 0 {
		output = format!("{}m {}", m, output);
	}
	if h != 0 {
		output = format!("{}h {}", h, output);
	}
	if d != 0 {
		output = format!("{}D {}", d, output);
	}
	output
}

pub fn shorten(s: &str, max_chars: usize) -> String {
	match s.char_indices().nth(max_chars) {
		None => s.to_string(),
		Some((idx, _)) => s[..idx].to_string(),
	}
}

/// Escapes a string for use in Discord, escaping all Markdown characters.
///
/// Square brackets can't be escaped with slashes for some reason, so they're
/// replaced with similar-looking characters.
pub fn escape_str(s: &str) -> String {
	lazy_static! {
		static ref ESCAPE_REGEX: Regex = Regex::new(r"([\\_*~`|])").unwrap();
	}
	ESCAPE_REGEX
		.replace_all(s, r"\$0")
		.replace('[', "⁅")
		.replace(']', "⁆")
}

pub fn push_chopped_str(base: &mut String, new_str: &str, max_len: usize) {
	const ELLIPSIS: char = '…';

	if new_str.len() > max_len {
		base.push_str(escape_str(&new_str[0..(max_len - 1)]).trim_end());
		base.push(ELLIPSIS);
	} else {
		base.push_str(escape_str(new_str).as_str());
	}
}

pub fn chop_str(s: &str, max_len: usize) -> String {
	let mut base = String::new();
	push_chopped_str(&mut base, s, max_len);
	base
}

pub fn display_time_span(millis: u64) -> String {
	const MILLIS_PER_SECOND: u64 = 1000;
	const SECONDS_PER_MINUTE: u64 = 60;
	const MINUTES_PER_HOUR: u64 = 60;
	const MILLIS_PER_MINUTE: u64 = MILLIS_PER_SECOND * SECONDS_PER_MINUTE;
	const MILLIS_PER_HOUR: u64 = MILLIS_PER_MINUTE * MINUTES_PER_HOUR;

	if millis >= MILLIS_PER_HOUR {
		format!(
			"{:02}:{:02}:{:02}",
			millis / MILLIS_PER_HOUR,
			(millis / MILLIS_PER_MINUTE) % MINUTES_PER_HOUR,
			(millis / MILLIS_PER_SECOND) % SECONDS_PER_MINUTE
		)
	} else {
		format!(
			"{:02}:{:02}",
			millis / MILLIS_PER_MINUTE,
			(millis / MILLIS_PER_SECOND) % SECONDS_PER_MINUTE
		)
	}
}
