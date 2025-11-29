use regex::Regex;
use std::collections::HashMap;
use std::{
    fmt,
    io::{self},
    path::PathBuf,
};

#[derive(Debug, PartialEq, Clone)]
pub enum EntryType {
    Bool,
    Str,
    Enum,
    Filepath,
    Dirpath,
    Int,
    INTERNAL,
    Static,
}

impl EntryType{
    fn from_str(s: &str) -> Option<EntryType> {
        match s {
            "BOOL" => Some(EntryType::Bool),
            "FILEPATH" => Some(EntryType::Filepath),
            "STRING" => Some(EntryType::Str),
            "STATIC" => Some(EntryType::Static),
            // "INTERNAL" => Some(EntryType::INTERNAL),
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

#[derive(Clone)]
pub struct CacheEntry {
    pub name: String,
    pub entry_type: EntryType,
    pub desc: String,
    pub value: String,
    pub values: Vec<String>,
    pub advanced: bool
}

impl CacheEntry {
    fn new(name: String, entry_type: EntryType, desc: String, value: String) -> Self {
        Self {
            name,
            entry_type,
            desc,
            value,
            values: Vec::new(),
            advanced: false,
        }
    }

    fn set_enum_values(&mut self, values_str: &str) {
        self.values = values_str.split(';').map(|s| s.to_string()).collect();
    }

    pub fn toggle_bool(&mut self) {
        let new_value = match self.value.to_lowercase().as_str() {
            "on" => Some("OFF".to_string()),
            "true" => Some("FALSE".to_string()),
            "yes" => Some("NO".to_string()),
            "y" => Some("N".to_string()),
            "1" => Some("0".to_string()),
            "off" => Some("ON".to_string()),
            "false" => Some("TRUE".to_string()),
            "no" => Some("YES".to_string()),
            "n" => Some("Y".to_string()),
            "ignore" => Some("ON".to_string()),
            "notfound" => Some("ON".to_string()),
            "" => Some("ON".to_string()),
            _ => None
        };

        self.value = new_value.unwrap_or(self.value.to_string());
    }

    pub fn cycle_enum(&mut self) {
        if self.values.is_empty() {
            return; // nothing to cycle
        }

        // Find the current index of `self.value` in `self.values`
        let current_index = self
            .values
            .iter()
            .position(|v| v == &self.value)
            .unwrap_or(0); // default to 0 if not found

        // Compute the next index, wrapping around
        let next_index = (current_index + 1) % self.values.len();

        // Update `self.value`
        self.value = self.values[next_index].clone();
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

pub struct CacheParser {
    var_regex: regex::Regex,
    enum_regex: regex::Regex,
    advanced_regex: regex::Regex,
}

impl CacheParser{
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            var_regex: regex::Regex::new(r"^([A-Za-z_][A-Za-z0-9_]*)\:([A-Z]+)\=(.*)$")?,
            enum_regex: regex::Regex::new(r"^([^-]+)-STRINGS:INTERNAL=(.+)$")?,
            advanced_regex: regex::Regex::new(r"^([^-]+)-ADVANCED:INTERNAL=1$")?,
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
                let entry_type = match EntryType::from_str(&caps[2]) {
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

                if entry.entry_type != EntryType::Static{
                    var_map.insert(name.to_string(), entry);
                }
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

            if let Some(caps) = self.advanced_regex.captures(line) {
                let name = &caps[1];
                if let Some(entry) = var_map.get_mut(name){
                    entry.advanced = true;
               }
            }
        }
    }

    fn parse_cache(&self, content: &str) -> HashMap<String, CacheEntry> {
        let var_map = match content.split_once("# INTERNAL cache entries") {
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

pub fn parse_cmake_cache(build_dir: &str) -> io::Result<Vec<CacheEntry>> {
    let mut cmake_cache_path = PathBuf::from(build_dir);
    cmake_cache_path.push("CMakeCache.txt");

    // println!("Reading CMake cache from: {:?}", cmake_cache_path);

    let cache_content = std::fs::read_to_string(&cmake_cache_path)?;

    let parser = CacheParser::new()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Parse into HashMap<String, CacheEntry>
    let mut entries: Vec<CacheEntry> = parser.parse_cache(&cache_content)
        .into_iter()
        .map(|(name, mut entry)| {
            entry.name = name; // ensure the struct contains the key
            entry
        })
        .collect();

    // Sort by key (name)
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(entries)
}

