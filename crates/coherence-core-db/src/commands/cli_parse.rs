use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ParsedArgs {
    pub flags: HashMap<String, Vec<String>>,
    pub positionals: Vec<String>,
}

/// Parse argv-like slice: `--key value` pairs and remaining positional tokens.
pub fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs::default();
    let mut i = 0;
    while i < args.len() {
        let token = &args[i];
        if let Some(key) = token.strip_prefix("--") {
            if key.is_empty() {
                return Err("empty flag name".into());
            }
            let value = args
                .get(i + 1)
                .ok_or_else(|| format!("missing value for --{key}"))?;
            if value.starts_with('-') {
                return Err(format!("missing value for --{key} (got another flag)"));
            }
            parsed
                .flags
                .entry(key.to_string())
                .or_default()
                .push(value.clone());
            i += 2;
        } else {
            parsed.positionals.push(token.clone());
            i += 1;
        }
    }
    Ok(parsed)
}

impl ParsedArgs {
    pub fn single_flag<'a>(&'a self, name: &str) -> Result<Option<&'a str>, String> {
        match self.flags.get(name) {
            None => Ok(None),
            Some(values) if values.len() == 1 => Ok(Some(values[0].as_str())),
            Some(_) => Err(format!("--{name} must appear at most once")),
        }
    }

    pub fn multi_flag<'a>(&'a self, name: &str) -> Vec<&'a str> {
        self.flags
            .get(name)
            .map(|v| v.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}
