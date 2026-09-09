use std::io::{self, BufRead};
use std::str::FromStr;

#[derive(Debug, PartialEq)]
enum AllowedInstructions {
    INIT,
    ALLOC,
    USED,
    RESET,
}

// Concret struct to handle parsing str->enum errors
#[derive(Debug)]
struct ParseInstructionError;

impl FromStr for AllowedInstructions { // Mapper that converts input str into enum for match
    // iteration control
    type Err = ParseInstructionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_uppercase().as_str() {
            "INIT" => Ok(AllowedInstructions::INIT),
            "ALLOC"  => Ok(AllowedInstructions::ALLOC),
            "USED"   => Ok(AllowedInstructions::USED),
            "RESET"  => Ok(AllowedInstructions::RESET),
            _        => Err(ParseInstructionError),
        }
    }
}

fn main() {
    let stdin = io::stdin();
    let mut bump = 0; // Initialize dump controller
    let mut heap_size: i32 = 0; // Initialize heap size for allocator
    let mut result: Vec<String> = Vec::new(); // Initialize result vector that will contain partial solutions
    for line in stdin.lock().lines() {
        let l = line.unwrap();
        if l.is_empty() { continue; }
        let mut expression = l.split_whitespace(); // Get expression input
        // separated into whitespaces
        let instruction: Option<AllowedInstructions> = expression
            .next()
            .and_then(|s| s.parse().ok());// Get the instruction part parsed as valid enum
        let number: Option<i32> = if let Some(s) = expression.next() {
            s.trim().parse::<i32>().ok()
        } else {
            None
        }; // Get the Optional number part, i might not get it so thats why i need an option
        
        match instruction {
            Some(AllowedInstructions::INIT) => {
                if let Some(val) = number {
                    heap_size = val; // Sets heap size value
                    result.push("OK".to_string());
                } else {
                    println!("Inapropiate value for instruction INSERT, expected: num, got: {:?}", number);
                    return;
                }
            }
            Some(AllowedInstructions::ALLOC) => {
                if let Some(val) = number {
                    if bump + val > heap_size { // Critical point 
                        result.push("OOM".to_string()); // Out of memory
                    } else {
                        result.push(bump.to_string()); // Current size of memory
                        bump += val;
                    }
                }
                else {
                    println!("Inapropiate value for instruction ALLOC, expected: num, got: {:?}", number);
                    return; 
                }
            }
            Some(AllowedInstructions::USED) => {
                if number.is_some() {
                    println!("Unexpected value for instruction USED: got: {:?}", number);
                    return;
                } else {
                    result.push(bump.to_string()); // Current use of memory size
                }
            }
            Some(AllowedInstructions::RESET) => {
                if number.is_some() {
                    println!("Unexpected value for instruction RESET: got: {:?}", number);
                    return;
                } else {
                    bump = 0; // Rest memory use of size
                    result.push("OK".to_string());
                }
            }
            None => println!("Invalid or missing instruction"),
        }
    }
    for value in result.iter() {
        println!("{}", value);
    }
}
