use regex::Regex;
use std::collections::HashMap;
use std::{
    fmt,
    io::{self},
    path::PathBuf,
};

#[derive(Debug)]
pub enum EntryType {
    Bool,
    Str,
    Enum,
    Filepath,
    Dirpath,
    Int,
    INTERNAL,
}

impl EntryType{
    fn from_str(s: &str) -> Option<EntryType> {
        match s {
            "BOOL" => Some(EntryType::Bool),
            "FILEPATH" => Some(EntryType::Filepath),
            "STRING" => Some(EntryType::Str),
            "STATIC" => Some(EntryType::INTERNAL),
            "PATH" => Some(EntryType::Dirpath),
            _ => None,
        }
    }
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct CacheEntry {
    name: String,
    entry_type: EntryType,
    desc: String,
    value: String,
    values: Vec<String>,
    // advanced: bool
}

impl CacheEntry {
    fn new(name: String, entry_type: EntryType, desc: String, value: String) -> Self {
        Self {
            name,
            entry_type,
            desc,
            value,
            values: Vec::new(),
        }
    }

    fn set_enum_values(&mut self, values_str: &str) {
        self.values = values_str.split(';').map(|s| s.to_string()).collect();
    }
}


impl fmt::Display for CacheEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CacheEntry {{ name: {}, type: {}, desc: {}, value: {}, values: {:?} }}",
            self.name, self.entry_type, self.desc, self.value, self.values
        )
    }
}

fn parse_entry_type(s: &str) -> Option<EntryType> {
    match s {
        "BOOL" => Some(EntryType::Bool),
        "FILEPATH" => Some(EntryType::Filepath),
        "STRING" => Some(EntryType::Str),
        "STATIC" => Some(EntryType::INTERNAL),
        "PATH" => Some(EntryType::Dirpath),
        _ => None,
    }
}

pub struct CacheParser {
    var_regex: regex::Regex,
    enum_regex: regex::Regex
}

impl CacheParser{
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            var_regex: regex::Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)\:([A-Z]+)\=(.*)$")?,
            enum_regex: regex::Regex::new(r"^([^-]+)-STRINGS:INTERNAL=(.+)$")?,
        })
    }

    fn parse_external_section(&self, external: &str) -> HashMap<String, CacheEntry> {
        let mut var_map = HashMap::new();
        let mut current_desc = String::new();

        for line in external.lines() {
            if line.starts_with("//"){
                current_desc.push_str(line.trim_start_matches("//"));
                continue;
            }

            if let Some(caps) = self.var_regex.captures(line){
                let name = &caps[1];
                let entry_type = match parse_entry_type(&caps[2]) {
                    Some(t) => t,
                    None => EntryType::Str,
                };
                let value = &caps[3];

                let entry = CacheEntry::new(
                    name.to_string(),
                    entry_type,
                    current_desc.to_string(),
                    value.to_string()
                );

                var_map.insert(name.to_string(), entry);
                current_desc.clear();
            }
        }
        var_map
    }

    fn parse_internal_section(&self, internal: &str, var_map: &mut HashMap<String, CacheEntry>){
        for line in internal.lines(){
            if let Some(caps) = self.enum_regex.captures(line) {
                let name = &caps[1];
                let values = &caps[2];

                if let Some(entry) = var_map.get_mut(name){
                    entry.entry_type = EntryType::Enum;
                    entry.set_enum_values(&values);
               }
            }
        }
    }

    fn parse_cache(&self, content: &str) -> HashMap<String, CacheEntry> {
        let mut var_map = match content.split_once("# Internal cache entries") {
            Some((external, internal)) => {
                let mut var_map = self.parse_external_section(external);
                self.parse_internal_section(internal, &mut var_map);
                var_map
            }
            None => self.parse_external_section(content),
        };
        var_map
    }
}

pub fn parse_cmake_cache(build_dir: &str) -> io::Result<HashMap<String, CacheEntry>> {
    let mut cmake_cache_path = PathBuf::from(build_dir);
    cmake_cache_path.push("CMakeCache.txt");

    println!("Reading CMake cache from: {:?}", cmake_cache_path);

    let cache_content = std::fs::read_to_string(&cmake_cache_path)?;

    let parser = CacheParser::new()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(parser.parse_cache(&cache_content))
}

