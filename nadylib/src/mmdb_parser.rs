use byteorder::{LittleEndian, ReadBytesExt};

use std::{
    collections::HashMap,
    io::{Cursor, Seek, SeekFrom},
};

const MMDB: &[u8] = include_bytes!("../data/text.mdb");

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub offset: u64,
    pub entry_id: u32,
}

pub struct MmdbParser<'a> {
    cache: HashMap<u32, HashMap<u32, String>>,
    buf: Cursor<&'a [u8]>,
}

impl MmdbParser<'_> {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            buf: Cursor::new(MMDB),
        }
    }

    pub fn get_message(&mut self, category_id: u32, instance_id: u32) -> Option<String> {
        let cached = self
            .cache
            .get(&category_id)
            .map(|i| i.get(&instance_id))
            .flatten();
        if cached.is_some() {
            return cached.cloned();
        }

        let category = self.find_entry(category_id, 8)?;
        let instance = self.find_entry(instance_id, category.offset)?;
        self.buf.seek(SeekFrom::Start(instance.offset)).unwrap();
        let result = self.read_string();

        match self.cache.get_mut(&category_id) {
            Some(v) => {
                v.insert(instance_id, result.clone());
            }
            None => {
                let mut map = HashMap::with_capacity(1);
                map.insert(instance_id, result.clone());
                self.cache.insert(category_id, map);
            }
        };

        Some(result)
    }

    fn find_entry(&mut self, entry_id: u32, offset: u64) -> Option<Entry> {
        self.buf.seek(SeekFrom::Start(offset)).unwrap();

        let mut previous_id = None;
        let mut current_entry = self.read_entry();

        while previous_id.is_none() || current_entry.entry_id > previous_id.unwrap() {
            if current_entry.entry_id == entry_id {
                return Some(current_entry);
            }

            previous_id = Some(current_entry.entry_id);
            current_entry = self.read_entry();
        }

        None
    }

    fn read_entry(&mut self) -> Entry {
        Entry {
            entry_id: self.buf.read_u32::<LittleEndian>().unwrap(),
            offset: self.buf.read_u32::<LittleEndian>().unwrap() as u64,
        }
    }

    fn read_string(&mut self) -> String {
        let mut msg = String::new();

        let mut byte = match self.buf.read_u8() {
            Ok(v) => v,
            Err(_) => return msg,
        };
        while byte != 0 {
            msg.push(byte as char);
            match self.buf.read_u8() {
                Ok(v) => byte = v,
                Err(_) => break,
            }
        }

        msg
    }

    pub fn find_all_instances_in_category(&mut self, category_id: u32) -> Option<Vec<Entry>> {
        let category = self.find_entry(category_id, 8)?;
        self.buf.seek(SeekFrom::Start(category.offset)).unwrap();
        let mut instances = Vec::with_capacity(1);
        let mut instance = self.read_entry();
        let mut previous_instance: Option<Entry> = None;

        while previous_instance.is_none() || instance.entry_id > previous_instance.unwrap().entry_id
        {
            instances.push(instance.clone());
            previous_instance = Some(instance);
            instance = self.read_entry();
        }

        Some(instances)
    }

    pub fn get_categories(&mut self) -> Vec<Entry> {
        self.buf.seek(SeekFrom::Start(8)).unwrap();
        let mut categories = Vec::with_capacity(1);
        let mut category = self.read_entry();
        let mut previous_category: Option<Entry> = None;

        while previous_category.is_none() || category.entry_id > previous_category.unwrap().entry_id
        {
            categories.push(category.clone());
            previous_category = Some(category);
            category = self.read_entry();
        }

        categories
    }
}

fn rewrite_c_formatting_to_rust(input: String) -> String {
    // WIP
    // TODO: Rethink with Regex
    // Format is
    // %[flags][width][.precision][length]specifier
    let mut output = String::new();
    let mut iter = input.chars().peekable();
    while iter.peek().is_some() {
        let character = iter.next().unwrap();
        // If escaped, do nothing
        if character == '%' && (iter.peek() == Some(&'%') || iter.peek().is_none()) {
            output.push('%');
            let _ = iter.next();
        } else if character == '%' {
            let next = iter.next().unwrap();
            match next {
                's' | 'i' | 'u' | 'd' => {
                    output += "{}";
                }
                '.' => {}
                _ => output.push('%'),
            }
        } else {
            output.push(character);
        }
    }
    output
}

#[test]
fn test_rewriting() {
    let f1 = String::from("Hello, %s, you got %i credits!");
    println!("{}", rewrite_c_formatting_to_rust(f1));
    let f2 = String::from("The tower is down %i%%!");
    println!("{}", rewrite_c_formatting_to_rust(f2));
}

#[test]
fn test_can_find_offline_msg() {
    let mut parser = MmdbParser::new();
    let string = parser.get_message(20000, 172363154);
    assert!(string.is_some());
}

#[test]
fn test_cache_works() {
    let mut parser = MmdbParser::new();
    let string = parser.get_message(20000, 172363154);
    assert!(string.is_some());
    assert_eq!(parser.cache.len(), 1);
}

#[test]
fn test_full_db() {
    use serde_json::to_string;

    let mut parser = MmdbParser::new();
    let mut all = HashMap::<String, HashMap<String, String>>::new();
    let categories = parser.get_categories();
    for category in categories {
        let category_id = category.entry_id;
        let entries = parser
            .find_all_instances_in_category(category.entry_id)
            .unwrap();
        for entry in entries {
            let instance_id = entry.entry_id;
            let result = parser.get_message(category_id, instance_id).unwrap();

            match all.get_mut(&category_id.to_string()) {
                Some(v) => {
                    v.insert(instance_id.to_string(), result);
                }
                None => {
                    let mut map = HashMap::with_capacity(1);
                    map.insert(instance_id.to_string(), result);
                    all.insert(category_id.to_string(), map);
                }
            };
        }
    }
    assert!(to_string(&all).is_ok());
}
