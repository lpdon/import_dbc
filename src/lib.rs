#[macro_use]
extern crate lazy_static;

use std::fs;
use std::error::Error;
use regex::Regex;

pub struct Config {
    pub filename: String,
}

#[derive(Debug)]
pub struct Dbc {
    pub nodes: Vec<Node>,
    pub messages: Vec<Message>
}

#[derive(Debug, PartialEq)]
enum DbcError {
    WrongType,
    InvalidContent
}

#[derive(Debug)]
pub struct Node {
    pub name: String
}

#[derive(Debug)]
pub struct Message {
    pub id: u32,
    pub name: String,
    pub size: u8,
    pub signals: Vec<Signal>
}

#[derive(Clone, Debug)]
pub struct Signal {
    pub name: String,
    pub start_bit: u16,
    pub size: u16,
    pub is_little_endian: bool,
    pub is_signed: bool,
    pub factor: String,
    pub offset: String,
    pub value_min: String,
    pub value_max: String,
    pub unit: String
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str>  {
        if args.len() < 1 {
            return Err("not enough arguments");
        }

        let filename = args[1].clone();

        Ok(Self { filename })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.filename)?;
    let dbc = parse(&contents);
    println!("{:?}", dbc);

    Ok(())
}

pub fn parse(contents: &str) -> Dbc {
    let mut nodes: Vec<Node> = Vec::new();
    let mut messages: Vec<Message> = Vec::new();
    let mut signals: Vec<Signal> = Vec::new();

    let mut in_message = false;
    for (i, line) in contents.lines().enumerate() {
        if !in_message {
            match parse_nodes(line) {
                Ok(new_nodes) => {
                    nodes = new_nodes;
                },
                Err(DbcError::InvalidContent) => {
                    panic!("Error when parsing line {}: {}. Invalid syntax for nodes.", i+1, line);
                },
                Err(_) => {},
            }
            
            match parse_message(line) {
                Ok(new_message) => {
                    in_message = true;
                    messages.push(new_message);
                },
                Err(DbcError::InvalidContent) => {
                    panic!("Error when parsing line {}: {}. Invalid message start.", i+1, line);
                },
                Err(_) => {},
            }
        }
        else {
            let current_message = messages.last_mut().unwrap();
            match parse_signal(line) {
                Ok(new_signal) => {
                    in_message = true;
                    signals.push(new_signal);
                },
                Err(DbcError::InvalidContent) => {
                    panic!("Error when parsing line {}: {}. Invalid signal.", i+1, line);
                },
                Err(_) => {
                    // In this case, the message block ended so the
                    // signals are pushed to the current message and
                    // the message is finished
                    in_message = false;
                    current_message.signals = signals.clone();
                    signals.clear();
                },
            }
        }
    }

    // If a message block is still open, add the remaining 
    // signals and finish it
    if in_message {
        let current_message = messages.last_mut().unwrap();
        current_message.signals = signals.clone();
    }

    Dbc{ nodes, messages }
}

fn parse_nodes(content: &str) -> Result<Vec<Node>, DbcError> {
    let content = content.trim();
    lazy_static! {
        static ref RE: Regex = Regex::new(r"BU_: (\w+\s*)+").unwrap();
        static ref RE_NODE: Regex = Regex::new(r"(\w+)").unwrap();
    }

    if !RE.is_match(content) {
        if !content.contains("BU_ ") {
            return Err(DbcError::WrongType);
        }
        else {
            return Err(DbcError::InvalidContent);
        }
    }
    
    let mut nodes: Vec<Node> = Vec::new();

    for cap in RE_NODE.captures_iter(content) {
        let name = cap[0].to_string();
        if name != "BU_" {
            let node = Node{ name: cap[0].to_string() }; 
            nodes.push(node);
        }
    }

    Ok(nodes)
}

fn parse_message(content: &str) -> Result<Message, DbcError> {
    let content = content.trim();
    lazy_static! {
        static ref RE: Regex = Regex::new(r"BO_ (\w+) (\w+) *: (\w+) (\w+).*").unwrap();
    }
    
    if !RE.is_match(content) {
        if !content.contains("BO_ ") {
            return Err(DbcError::WrongType);
        }
        else {
            return Err(DbcError::InvalidContent);
        }
    }
    
    let cap = RE.captures(content).unwrap();

    Ok (
        Message { 
            id: cap[1].parse::<u32>().unwrap(),
            name: cap[2].to_string(),
            size: cap[3].parse::<u8>().unwrap(),
            signals: Vec::new()
        }
    )
}

fn parse_signal(content: &str) -> Result<Signal, DbcError> {
    let content = content.trim();
    lazy_static! {
        static ref RE: Regex = Regex::new(r#"SG_ (\w+) : (\d+)\|(\d+)@(\d+)([\+|\-]) \(([0-9.+\-eE]+),([0-9.+\-eE]+)\) \[([0-9.+\-eE]+)\|([0-9.+\-eE]+)\] "(.*)" (.*)"#).unwrap();
    }

    if !RE.is_match(content) {
        if !content.contains("SG_ ") {
            return Err(DbcError::WrongType);
        }
        else {
            return Err(DbcError::InvalidContent);
        }
    }

    let cap = RE.captures(content).unwrap();

    Ok (
        Signal { 
            name: cap[1].to_string(),
            start_bit: cap[2].parse().unwrap(),
            size: cap[3].parse().unwrap(),
            is_little_endian: cap[4].to_string() == "1",
            is_signed: cap[5].to_string() == "-",
            factor: cap[6].to_string(),
            offset: cap[7].to_string(),
            value_min: cap[8].to_string(),
            value_max: cap[9].to_string(),
            unit: cap[10].to_string()
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Setup<'a>{
        test_messages: &'a str
    }

    impl Setup<'_> {
        fn new() -> Self {
            let test_messages = "
BU_: TCU VEHICLE

BO_ 2566117891 MsgDummy1: 8 Vector__XXX
 SG_ dummy1sg1 : 34|2@1+ (1,0) [0|3] \"kkk\" Vector__XXX
 SG_ dummy1sg2 : 18|16@1- (1,0) [0|65535] \"\" Vector__XXX
 SG_ dummy1sg3 : 2|16@1+ (1,0) [0|65535] \"\" Vector__XXX
 SG_ dummy1sg4 : 0|2@1+ (1,0) [0|3] \"\" Vector__XXX

BO_ 2565921559 MsgDummy2: 8 Vector__XXX
 SG_ gps_longitude : 39|32@0- (1E-007,0) [-214.7483648|214.7483647] \"deg\" Vector__XXX
 SG_ gps_latitude : 7|32@0- (1E-007,0) [-214.7483648|214.7483647] \"deg\" Vector__XXX

BO_ 2565986819 MsgDummy3: 8 TCU
 SG_ dummy3sg1 : 16|16@1+ (0.125,0) [0|8191.875] \"\" Vector__XXX
";
            Self { test_messages }
        }
    }

    #[test]
    fn valid_message_start() {
        let content = "BO_ 2566117891 MsgDummy1: 8 Vector__XXX";
        assert_eq!(parse_message(content).is_ok(), true);
    }

    #[test]
    fn invalid_message_start() {
        let content = "BO_ 2566117891 MsgDummy1: Vector__XXX";
        assert_eq!(parse_message(content).err().unwrap(), DbcError::InvalidContent);
    }

    #[test]
    fn not_message() {
        let content = "SG_ dummy1sg1 : 34|2@1+ (1,0) [0|3] \"kkk\" Vector__XXX";
        assert_eq!(parse_message(content).err().unwrap(), DbcError::WrongType);
    }

    #[test]
    fn valid_signal() {
        let content = "SG_ dummy1sg1 : 34|2@1+ (1,0) [0|3] \"kkk\" Vector__XXX";
        assert_eq!(parse_signal(content).is_ok(), true);
    }

    #[test]
    fn invalid_signal() {
        let content = "SG_ dummy1sg1 : 34|21+ (1,0) [0|3] \"kkk\" Vector__XXX";
        assert_eq!(parse_signal(content).err().unwrap(), DbcError::InvalidContent);
    }

    #[test]
    fn not_signal() {
        let content = "BO_ 2566117891 MsgDummy1: 8 Vector__XXX";
        assert_eq!(parse_signal(content).err().unwrap(), DbcError::WrongType);
    }

    #[test]
    fn num_signals() {
        let setup = Setup::new();
        let messages = parse(setup.test_messages).messages;
        assert_eq!(messages[0].signals.len(), 4);
        assert_eq!(messages[1].signals.len(), 2);
        assert_eq!(messages[2].signals.len(), 1);
    }

    #[test]
    fn signal_values() {
        let setup = Setup::new();
        let messages = parse(setup.test_messages).messages;
        assert_eq!(messages[1].signals[0].name, "gps_longitude");
        assert_eq!(messages[1].signals[0].start_bit, 39);
        assert_eq!(messages[1].signals[0].size, 32);
        assert_eq!(messages[1].signals[0].value_min, "-214.7483648");
        assert_eq!(messages[1].signals[0].value_max, "214.7483647");
        assert_eq!(messages[1].signals[0].unit, "deg");
        assert_eq!(messages[1].signals[0].is_little_endian, false);
        assert_eq!(messages[1].signals[0].is_signed, true);
    }

    #[test]
    fn nodes() {
        let content = "BU_: TCU VEHICLE";
        let nodes = parse_nodes(content).unwrap();
        assert_eq!(nodes[0].name, "TCU");
        assert_eq!(nodes[1].name, "VEHICLE");
    }
}
