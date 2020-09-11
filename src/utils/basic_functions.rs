// Capitalizes the first letter of a str.
pub fn capitalize_first(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn string_to_seconds(text: impl ToString) -> u64 {
    let s = text.to_string();
    let words = s.split(' ');
    let mut seconds = 0;

    for i in words {
        if i.ends_with("s") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0);
        }
        if i.ends_with("m") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 60;
        }
        if i.ends_with("h") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 3600;
        }
        if i.ends_with("D") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 86_400;
        }
        if i.ends_with("W") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 604_800;
        }
        if i.ends_with("M") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 2_628_288;
        }
        if i.ends_with("Y") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 31_536_000;
        }
    }

    seconds
}
