use serde_json::{Value, from_reader};
use std::fs::File;
use std::io::Write;
use slint::include_modules;

include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let main_window = AppWindow::new()?;

    main_window.on_request_process(|chip_path, text_path| {
        // Read the JSON file
        let file = match File::open(&chip_path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to open JSON file: {}", e);
                return;
            }
        };

        let mut json: Value = match from_reader(file) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Failed to parse JSON: {}", e);
                return;
            }
        };

        // Process JSON (display info)
        if let Err(e) = process_json(&json) {
            eprintln!("JSON Error: {}", e);
        }

        // Process text file and update JSON
        match process_text_and_update_json(&mut json, &text_path) {
            Ok(_) => {
                // Save the modified JSON back to chip_path
                let file = match File::create(&chip_path) {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("Failed to create JSON file: {}", e);
                        return;
                    }
                };
                if let Err(e) = serde_json::to_writer_pretty(file, &json) {
                    eprintln!("Failed to write JSON: {}", e);
                    return;
                }
                println!("Successfully updated JSON file with text data");
            },
            Err(e) => eprintln!("Text File Error: {}", e),
        }
    });

    main_window.run()
}

fn process_json(json: &Value) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Processing JSON File ===");

    if let Some(sub_chips) = json.get("SubChips").and_then(|v| v.as_array()) {
        for (i, chip) in sub_chips.iter().enumerate() {
            println!("\nSubChip #{}:", i + 1);

            if let Some(name) = chip.get("Name").and_then(|v| v.as_str()) {
                println!("\tName: {}", name);
            } else {
                println!("\tName: Not found or invalid");
            }

            if let Some(id) = chip.get("ID").and_then(|v| v.as_str()) {
                println!("\tID: {}", id);
            } else {
                println!("\tID: Not found or invalid");
            }

            if let Some(internal_data) = chip.get("InternalData") {
                if let Some(arr) = internal_data.as_array() {
                    println!("InternalData values:");
                    for (idx, value) in arr.iter().enumerate() {
                        if let Some(n) = value.as_i64() {
                            println!("  [{}]: {}", idx, n);
                        } else {
                            println!("  [{}]: Invalid format", idx);
                        }
                    }
                } else {
                    println!("InternalData: Not an array");
                }
            } else {
                println!("No InternalData found");
            }

            println!("\n{}", "-".repeat(40));
        }
    } else {
        println!("No SubChips found in JSON");
    }

    Ok(())
}

fn process_text_and_update_json(json: &mut Value, text_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Processing Text File ===");
    println!("File: {}", text_path);

    let content = std::fs::read_to_string(text_path)?;
    println!("\nFile content (first 100 chars):");
    println!("{}", content.chars().take(100).collect::<String>());

    // Validate and parse the text content
    let numbers = parse_text_content(&content)?;

    // Write numbers to SubChips
    write_numbers_to_subchips(json, &numbers)?;

    Ok(())
}

fn parse_text_content(content: &str) -> Result<Vec<i64>, Box<dyn std::error::Error>> {
    let mut numbers = Vec::new();
    for part in content.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let num = if part.starts_with("0x") {
            i64::from_str_radix(&part[2..], 16)
        } else {
            part.parse::<i64>()
        };
        match num {
            Ok(n) => numbers.push(n),
            Err(e) => return Err(format!("Invalid number '{}': {}", part, e).into()),
        }
    }
    Ok(numbers)
}

fn write_numbers_to_subchips(json: &mut Value, numbers: &[i64]) -> Result<(), Box<dyn std::error::Error>> {
    let subchips = json.get_mut("SubChips")
        .and_then(|v| v.as_array_mut())
        .ok_or("SubChips array not found or not an array")?;

    let mut remaining_numbers = numbers;
    for subchip in subchips.iter_mut() {
        if remaining_numbers.is_empty() {
            break;
        }
        let internal_data = subchip.get_mut("InternalData")
            .and_then(|v| v.as_array_mut())
            .ok_or("InternalData not found or not an array")?;

        let capacity = internal_data.len();
        let write_count = std::cmp::min(capacity, remaining_numbers.len());

        for (i, num) in remaining_numbers[..write_count].iter().enumerate() {
            if i < internal_data.len() {
                internal_data[i] = Value::from(*num);
            } else {
                break;
            }
        }

        remaining_numbers = &remaining_numbers[write_count..];
    }

    if !remaining_numbers.is_empty() {
        return Err(format!("Insufficient SubChips to write all numbers, {} remaining", remaining_numbers.len()).into());
    }

    Ok(())
}