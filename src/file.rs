pub fn read(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let file = String::from_utf8(std::fs::read(path)?)?;
    Ok(file
        .lines()
        .filter_map(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        })
        .collect())
}
