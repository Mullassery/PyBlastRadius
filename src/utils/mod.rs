use crate::error::Result;
use std::fs;
use std::path::Path;

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
    fs::read_to_string(path).map_err(crate::error::Error::IoError)
}

pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    fs::write(path, content).map_err(crate::error::Error::IoError)
}

pub fn read_json_file<P: AsRef<Path>, T: for<'de> serde::de::Deserialize<'de>>(
    path: P,
) -> Result<T> {
    let content = read_file(path)?;
    serde_json::from_str(&content).map_err(crate::error::Error::JsonError)
}

pub fn write_json_file<P: AsRef<Path>, T: serde::ser::Serialize>(path: P, value: &T) -> Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    write_file(path, &content)
}

pub fn read_yaml_file<P: AsRef<Path>, T: for<'de> serde::de::Deserialize<'de>>(
    path: P,
) -> Result<T> {
    let content = read_file(path)?;
    serde_yaml::from_str(&content).map_err(crate::error::Error::YamlError)
}

pub fn write_yaml_file<P: AsRef<Path>, T: serde::ser::Serialize>(path: P, value: &T) -> Result<()> {
    let content = serde_yaml::to_string(value)?;
    write_file(path, &content)
}

pub fn format_as_table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let mut output = String::new();

    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Print headers
    for (i, header) in headers.iter().enumerate() {
        output.push_str(&format!("{:<width$} ", header, width = widths[i]));
    }
    output.push('\n');

    // Print separator
    for width in &widths {
        output.push_str(&"-".repeat(width + 1));
    }
    output.push('\n');

    // Print rows
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                output.push_str(&format!("{:<width$} ", cell, width = widths[i]));
            }
        }
        output.push('\n');
    }

    output
}
