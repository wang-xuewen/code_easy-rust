use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    pub name: String,
    pub value: i32,
}

impl Data {
    pub fn new(name: &str, value: i32) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
    
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

// 只有在启用 regex 特性时才编译
#[cfg(feature = "regex")]
pub fn validate_pattern(pattern: &str, text: &str) -> bool {
    use regex::Regex;
    Regex::new(pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

// 只有在启用 rand 特性时才编译
#[cfg(feature = "rand")]
pub fn random_number(max: u32) -> u32 {
    use rand::Rng;
    rand::thread_rng().gen_range(0..max)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_data_creation() {
        let data = Data::new("test", 42);
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }
    
    #[test]
    fn test_json_output() {
        let data = Data::new("test", 42);
        let json = data.to_json();
        assert!(json.contains("test"));
        assert!(json.contains("42"));
    }
}