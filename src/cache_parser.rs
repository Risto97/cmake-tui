use std::collections::HashMap;
use std::{
    fmt,
    io::{self},
    path::PathBuf,
};

#[derive(Debug, PartialEq, Clone)]
pub enum VarType {
    Bool,
    Str,
    Enum,
    Filepath,
    Dirpath,
    // Int,
    // INTERNAL,
    Static,
}

impl VarType{
    fn from_str(s: &str) -> Option<VarType> {
        match s {
            "BOOL" => Some(VarType::Bool),
            "FILEPATH" => Some(VarType::Filepath),
            "STRING" => Some(VarType::Str),
            "STATIC" => Some(VarType::Static),
            // "INTERNAL" => Some(VarType::INTERNAL),
            "PATH" => Some(VarType::Dirpath),
            _ => None,
        }
    }
}

impl fmt::Display for VarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct CacheVar {
    pub name: String,
    pub typ: VarType,
    pub desc: String,
    pub value: String,
    pub values: Vec<String>,
    pub advanced: bool
}

impl CacheVar {
    fn new(name: String, typ: VarType, desc: String, value: String) -> Self {
        Self {
            name,
            typ,
            desc,
            value,
            values: Vec::new(),
            advanced: false,
        }
    }

    fn set_enum_values(&mut self, values_str: &str) {
        self.values = values_str.split(';').map(|s| s.to_string()).collect();
    }

    pub fn cycle_enum(&self, val: &String) -> String {
        if self.values.is_empty() {
            return val.clone(); // nothing to cycle
        }

        // Find the current index of `self.value` in `self.values`
        let current_index = self
            .values
            .iter()
            .position(|v| v == val)
            .unwrap_or(0); // default to 0 if not found

        // Compute the next index, wrapping around
        let next_index = (current_index + 1) % self.values.len();

        // Update `self.value`
        self.values[next_index].clone()
    }

    pub fn toggle_bool(val: &String) -> String {
        let new_value = match val.to_lowercase().as_str() {
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
        // self.value = new_value.unwrap_or(self.value.to_string());
        new_value.unwrap_or(val.clone())
    }

}


impl fmt::Display for CacheVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CacheVar {{ name: {}, type: {}, desc: {}, value: {}, values: {:?} }}",
            self.name, self.typ, self.desc, self.value, self.values
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

    fn parse_external_section(&self, external: &str) -> HashMap<String, CacheVar> {
        let mut var_map = HashMap::new();
        let mut current_desc = String::new();

        for line in external.lines() {
            if line.starts_with("//"){
                current_desc.push_str(line.trim_start_matches("//"));
                continue;
            }

            if let Some(caps) = self.var_regex.captures(line){
                let name = &caps[1];
                let typ = match VarType::from_str(&caps[2]) {
                    Some(t) => t,
                    None => VarType::Str,
                };
                let value = &caps[3];

                let var = CacheVar::new(
                    name.to_string(),
                    typ,
                    current_desc.to_string(),
                    value.to_string()
                );

                if var.typ != VarType::Static{
                    var_map.insert(name.to_string(), var);
                }
                current_desc.clear();
            }
        }
        var_map
    }

    fn parse_internal_section(&self, internal: &str, var_map: &mut HashMap<String, CacheVar>){
        for line in internal.lines(){
            if let Some(caps) = self.enum_regex.captures(line) {
                let name = &caps[1];
                let values = &caps[2];

                if let Some(var) = var_map.get_mut(name){
                    var.typ = VarType::Enum;
                    var.set_enum_values(&values);
               }
            }

            if let Some(caps) = self.advanced_regex.captures(line) {
                let name = &caps[1];
                if let Some(var) = var_map.get_mut(name){
                    var.advanced = true;
               }
            }
        }
    }

    fn parse_cache(&self, content: &str) -> HashMap<String, CacheVar> {
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

pub fn parse_cmake_cache(build_dir: PathBuf) -> io::Result<Vec<CacheVar>> {
    let mut cmake_cache_path = build_dir.clone();
    cmake_cache_path.push("CMakeCache.txt");

    // println!("Reading CMake cache from: {:?}", cmake_cache_path);

    let cache_content = std::fs::read_to_string(&cmake_cache_path)?;

    let parser = CacheParser::new()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    // Parse into HashMap<String, CacheVar>
    let mut entries: Vec<CacheVar> = parser.parse_cache(&cache_content)
        .into_iter()
        .map(|(name, mut var)| {
            var.name = name; // ensure the struct contains the key
            var
        })
        .collect();

    // Sort by key (name)
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(entries)
}

