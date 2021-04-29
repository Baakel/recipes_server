

pub fn process_steps(steps_string: String) -> Option<Vec<String>> {
    let split_string: Vec<_> = steps_string.lines().map(|s| s.to_string()).collect();
    Option::from(split_string)
}