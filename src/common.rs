use std::fmt::Display;

#[derive(Debug)]
pub struct CustomError (pub String);

impl std::error::Error for CustomError {}

impl Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}


pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

