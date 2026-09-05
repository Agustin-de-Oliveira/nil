pub fn detect_url(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| {
            c == '"' || c == '\'' || c == '`' || c == '(' || c == ')' || c == '[' || c == ']' || c == '<' || c == '>' || c == '{' || c == '}' || c == ',' || c == ';' || c == '.'
        });
        if clean.starts_with("http://") || clean.starts_with("https://") {
            return Some(clean.to_string());
        }
        if clean.starts_with("www.") {
            return Some(format!("https://{}", clean));
        }
        if (clean.contains(".com") || clean.contains(".org") || clean.contains(".io") || clean.contains(".dev") || clean.contains(".net") || clean.contains(".app") || clean.contains(".ar"))
            && clean.contains('.')
            && !clean.contains('@')
            && clean.len() >= 4
        {
            return Some(format!("https://{}", clean));
        }
    }
    None
}

pub fn detect_color(text: &str) -> Option<String> {
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| {
            c == '"' || c == '\'' || c == '`' || c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' || c == ',' || c == ';' || c == ':'
        });
        if clean.starts_with('#') {
            let hex = &clean[1..];
            if (hex.len() == 3 || hex.len() == 4 || hex.len() == 6 || hex.len() == 8)
                && hex.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Some(clean.to_string());
            }
        } else if (clean.starts_with("rgb(") || clean.starts_with("rgba(") || clean.starts_with("hsl(") || clean.starts_with("hsla(")) && clean.ends_with(')') {
            return Some(clean.to_string());
        }
    }
    None
}
