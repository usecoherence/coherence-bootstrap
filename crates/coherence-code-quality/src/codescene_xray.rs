use std::path::Path;
use std::process::Command;

use serde_json::Value;

const CS_API_URL: &str = "https://api.codescene.io/v2";

type FileDataResult = (Value, i64, Option<f64>, Option<f64>);

struct ApiConfig {
    url: String,
    project_id: String,
    token: String,
}

impl ApiConfig {
    fn from_env() -> Option<Self> {
        let token = std::env::var("CS_ACCESS_TOKEN").ok()?;
        let project_id = std::env::var("CS_PROJECT_ID").ok()?;
        if token.is_empty() || project_id.is_empty() {
            return None;
        }
        Some(Self {
            url: CS_API_URL.to_string(),
            project_id,
            token,
        })
    }
}

#[derive(Debug)]
pub struct XrayFileMetrics {
    pub code_health: Option<String>,
    pub loc: Option<String>,
    pub language: Option<String>,
    pub change_frequency: Option<i64>,
    pub defects: Option<i64>,
    pub authors: Option<i64>,
    pub revisions: Option<i64>,
    pub friction: Option<f64>,
    pub friction_month: Option<f64>,
    pub code_smells: Vec<CodeSmell>,
    pub health_now: Option<String>,
    pub health_month: Option<String>,
    pub health_year: Option<String>,
}

#[derive(Debug)]
pub struct CodeSmell {
    pub name: String,
    pub rule_set: String,
    pub count: Option<i64>,
}

#[derive(Debug)]
pub struct StructuralIssue {
    pub category: String,
    pub functions: Vec<IssueFunction>,
}

#[derive(Debug)]
pub struct IssueFunction {
    pub title: String,
    pub line: i64,
    pub details: Option<String>,
}

pub struct XrayReport {
    pub local_score: Option<String>,
    pub structural_issues: Vec<StructuralIssue>,
    pub metrics: Option<XrayFileMetrics>,
}

/// # Errors
/// Returns `Err` if `cs review` fails or the REST API call fails.
pub fn run_xray(file_path: &Path) -> Result<XrayReport, String> {
    let (score, issues) = run_cs_review(file_path)?;
    let metrics = fetch_file_metrics(file_path)?;

    Ok(XrayReport {
        local_score: score,
        structural_issues: issues,
        metrics,
    })
}

fn run_cs_review(file_path: &Path) -> Result<(Option<String>, Vec<StructuralIssue>), String> {
    let output = Command::new("cs")
        .arg("review")
        .arg(file_path.as_os_str())
        .arg("--output-format")
        .arg("json")
        .output()
        .map_err(|e| format!("failed to run cs CLI: {e} (is it installed?)"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cs review failed: {stderr}"));
    }

    let stdout =
        String::from_utf8(output.stdout).map_err(|e| format!("cs output not utf-8: {e}"))?;
    parse_cs_review_output(&stdout)
}

fn parse_cs_review_output(json: &str) -> Result<(Option<String>, Vec<StructuralIssue>), String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("cs review JSON parse: {e}"))?;

    let score = v
        .get("score")
        .and_then(Value::as_f64)
        .map(|s| format!("{s:.2}"));

    let mut issues = Vec::new();
    if let Some(review) = v.get("review").and_then(Value::as_array) {
        for entry in review {
            let category = entry
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("Unknown")
                .to_string();

            let functions = entry
                .get("functions")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| {
                            let title = f.get("title")?.as_str()?.to_string();
                            let line = f.get("start-line")?.as_i64()?;
                            let details =
                                f.get("details").and_then(Value::as_str).map(str::to_string);
                            Some(IssueFunction {
                                title,
                                line,
                                details,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if !functions.is_empty() {
                issues.push(StructuralIssue {
                    category,
                    functions,
                });
            }
        }
    }

    Ok((score, issues))
}

fn repo_relative_path(file_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8(output.stdout).ok()?;
    let root = root.trim().to_string();
    if root.is_empty() {
        return None;
    }
    let abs = if file_path.is_relative() {
        std::env::current_dir().ok().map(|cwd| cwd.join(file_path))
    } else {
        Some(file_path.to_path_buf())
    }?;
    let abs_str = abs.to_string_lossy();
    let without_root = abs_str
        .strip_prefix(&root)
        .or_else(|| abs_str.strip_prefix(format!("{root}/").as_str()))
        .map(|s| s.trim_start_matches('/').to_string());
    without_root
}

fn curl_get(url: &str, token: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("curl")
        .arg("-s")
        .arg("--fail")
        .arg("--header")
        .arg("Accept: application/json")
        .arg("--header")
        .arg(format!("Authorization: Bearer {token}"))
        .arg(url)
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "API call failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output.stdout)
}

fn fetch_file_entry(config: &ApiConfig, suffix: &str) -> Result<Option<Value>, String> {
    let file_url = format!(
        "{}/projects/{}/analyses/latest/files?page=1&page_size=500&fields=path,code_health,loc,change_frequency,number_of_defects,language,number_of_authors,code_health_rule_violations",
        config.url, config.project_id
    );

    let body = curl_get(&file_url, &config.token)?;
    let file_json: Value =
        serde_json::from_slice(&body).map_err(|e| format!("API JSON parse: {e}"))?;

    Ok(file_json
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|f| {
                f.get("path")
                    .and_then(Value::as_str)
                    .is_some_and(|p| p.ends_with(suffix))
            })
        })
        .cloned())
}

fn fetch_tech_debt_data(
    config: &ApiConfig,
    suffix: &str,
) -> (Option<i64>, Option<f64>, Option<f64>) {
    let td_url = format!(
        "{}/projects/{}/analyses/latest/technical-debt?page=1&page_size=500",
        config.url, config.project_id
    );

    let output = Command::new("curl")
        .arg("-s")
        .arg("--fail")
        .arg("--header")
        .arg("Accept: application/json")
        .arg("--header")
        .arg(format!("Authorization: Bearer {}", config.token))
        .arg(&td_url)
        .output();

    let td_output = match output {
        Ok(o) if o.status.success() => o,
        _ => return (None, None, None),
    };

    let td_json: Value = match serde_json::from_slice(&td_output.stdout) {
        Ok(v) => v,
        Err(_) => return (None, None, None),
    };

    td_json
        .get("result")
        .and_then(Value::as_array)
        .and_then(|results| {
            results.iter().find(|r| {
                r.get("file_name")
                    .and_then(Value::as_str)
                    .is_some_and(|p| p.ends_with(suffix))
            })
        })
        .map_or((None, None, None), |entry| {
            let rev = entry.get("revisions").and_then(Value::as_i64);
            let fric = entry.get("friction").and_then(Value::as_f64);
            let fric_mo = entry.get("friction_last_month").and_then(Value::as_f64);
            (rev, fric, fric_mo)
        })
}

fn fetch_file_data(config: &ApiConfig, suffix: &str) -> Result<Option<FileDataResult>, String> {
    let file_entry = fetch_file_entry(config, suffix)?;
    let (revisions, friction, friction_month) = fetch_tech_debt_data(config, suffix);

    Ok(file_entry.map(|entry| (entry, revisions.unwrap_or(0), friction, friction_month)))
}

#[allow(clippy::cast_possible_truncation)]
fn extract_file_metrics(
    entry: &Value,
    revisions: i64,
    friction: Option<f64>,
    friction_month: Option<f64>,
) -> XrayFileMetrics {
    let ch = entry.get("code_health");

    let code_smells = entry
        .get("code_health_rule_violations")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("code_smell")?.as_str()?.to_string();
                    let rule_set = item.get("rule_set")?.as_str()?.to_string();
                    let count = item.get("count").and_then(Value::as_i64);
                    Some(CodeSmell {
                        name,
                        rule_set,
                        count,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let authors = entry.get("number_of_authors").and_then(|v| {
        if v.is_number() {
            v.as_f64().map(|f| f as i64)
        } else {
            None
        }
    });

    let health_now = ch
        .and_then(|h| h.get("current_score").and_then(Value::as_f64))
        .map(|v| format!("{v:.2}"));
    let health_month = ch
        .and_then(|h| h.get("month_score").and_then(Value::as_f64))
        .map(|v| format!("{v:.2}"));
    let health_year = ch
        .and_then(|h| h.get("year_score").and_then(Value::as_str))
        .map(str::to_string);

    XrayFileMetrics {
        code_health: health_now.clone(),
        loc: entry.get("loc").and_then(Value::as_str).map(str::to_string),
        language: entry
            .get("language")
            .and_then(Value::as_str)
            .map(str::to_string),
        change_frequency: entry.get("change_frequency").and_then(Value::as_i64),
        defects: entry.get("number_of_defects").and_then(Value::as_i64),
        authors,
        revisions: Some(revisions),
        friction,
        friction_month,
        code_smells,
        health_now,
        health_month,
        health_year,
    }
}

fn fetch_file_metrics(file_path: &Path) -> Result<Option<XrayFileMetrics>, String> {
    let Some(config) = ApiConfig::from_env() else {
        return Ok(None);
    };

    let rel = repo_relative_path(file_path).unwrap_or_default();

    let file_data = fetch_file_data(&config, &rel)?;
    Ok(
        file_data.map(|(entry, revisions, friction, friction_month)| {
            extract_file_metrics(&entry, revisions, friction, friction_month)
        }),
    )
}

fn print_local_analysis(report: &XrayReport) {
    println!();
    println!("── Local Analysis ──────────────────────────────");
    if let Some(ref score) = report.local_score {
        println!("  Code Health:  {score}");
    } else {
        println!("  Code Health:  (unavailable)");
    }

    if !report.structural_issues.is_empty() {
        println!();
        println!("  Structural Issues:");
        for issue in &report.structural_issues {
            println!(
                "    [{}] {} function(s)",
                issue.category,
                issue.functions.len()
            );
            for f in &issue.functions {
                let detail = f
                    .details
                    .as_ref()
                    .map(|d| format!("  {d}"))
                    .unwrap_or_default();
                println!("      - {}  (line {}){detail}", f.title, f.line);
            }
        }
    }
}

fn print_server_metrics(report: &XrayReport) {
    println!();
    println!("── Server Metrics ──────────────────────────────");

    let Some(ref m) = report.metrics else {
        println!("  (set CS_ACCESS_TOKEN and CS_PROJECT_ID for server metrics)");
        return;
    };

    let loc_display = m.loc.as_deref().unwrap_or("?");
    println!("  LOC:          {loc_display}");
    println!("  Language:     {}", m.language.as_deref().unwrap_or("?"));
    println!(
        "  Change freq:  {}",
        m.change_frequency.map_or("?".into(), |v| v.to_string())
    );
    println!(
        "  Defects:      {}",
        m.defects.map_or("?".into(), |v| v.to_string())
    );
    println!(
        "  Authors:      {}",
        m.authors.map_or("?".into(), |v| v.to_string())
    );
    println!(
        "  Revisions:    {}",
        m.revisions.map_or("?".into(), |v| v.to_string())
    );
    println!(
        "  Friction:     {}",
        m.friction.map_or("?".into(), |v| format!("{v:.3}"))
    );
    println!(
        "  Friction/mo:  {}",
        m.friction_month.map_or("?".into(), |v| format!("{v:.3}"))
    );

    if !m.code_smells.is_empty() {
        println!("  Code smells:");
        for smell in &m.code_smells {
            let count = smell.count.map(|c| format!(" x{c}")).unwrap_or_default();
            println!("    - {} ({}){count}", smell.name, smell.rule_set);
        }
    }

    println!();
    println!("  Code Health History:");
    println!("    now:    {}", m.health_now.as_deref().unwrap_or("-"));
    println!("    month:  {}", m.health_month.as_deref().unwrap_or("-"));
    println!("    year:   {}", m.health_year.as_deref().unwrap_or("-"));
}

fn print_report(report: &XrayReport) {
    println!("╔═══════════════════════════════════════════════");
    println!("║ CodeScene X-Ray");
    println!("╚═══════════════════════════════════════════════");

    print_local_analysis(report);
    print_server_metrics(report);

    println!();
}

fn trigger_analysis(config: &ApiConfig) -> Result<i64, String> {
    let url = format!("{}/projects/{}/run-analysis", config.url, config.project_id);

    let output = Command::new("curl")
        .arg("-s")
        .arg("--fail")
        .arg("-X")
        .arg("POST")
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-H")
        .arg(format!("Authorization: Bearer {}", config.token))
        .arg(&url)
        .output()
        .map_err(|e| format!("curl (trigger) failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "trigger failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let v: Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("trigger JSON parse: {e}"))?;

    v.get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| "trigger response missing id".to_string())
}

fn wait_for_analysis(config: &ApiConfig, analysis_id: i64) -> Result<(), String> {
    let url = format!(
        "{}/projects/{}/analyses/{analysis_id}",
        config.url, config.project_id
    );

    for _ in 0..120 {
        let output = Command::new("curl")
            .arg("-s")
            .arg("--fail")
            .arg("-H")
            .arg("Accept: application/json")
            .arg("-H")
            .arg(format!("Authorization: Bearer {}", config.token))
            .arg(&url)
            .output()
            .map_err(|e| format!("curl (poll) failed: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "poll failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let body = String::from_utf8_lossy(&output.stdout);
        let v: Value = serde_json::from_str(&body).map_err(|e| format!("poll JSON parse: {e}"))?;

        if v.get("error")
            .and_then(Value::as_str)
            .is_some_and(|e| e == "Analysis is running.")
        {
            std::thread::sleep(std::time::Duration::from_secs(5));
            continue;
        }

        return Ok(());
    }

    Err("timed out waiting for analysis to complete".to_string())
}

fn trigger_and_wait() -> Result<(), String> {
    let config = ApiConfig::from_env()
        .ok_or_else(|| "--trigger requires CS_ACCESS_TOKEN and CS_PROJECT_ID".to_string())?;
    eprintln!("codescene-xray: triggering analysis...");
    let analysis_id = trigger_analysis(&config)?;
    eprintln!("codescene-xray: analysis #{analysis_id} scheduled, waiting for completion...");
    wait_for_analysis(&config, analysis_id)?;
    eprintln!("codescene-xray: analysis complete.{:>8}", "");
    Ok(())
}

fn display_xray(file_path: &Path) -> i32 {
    match run_xray(file_path) {
        Ok(report) => {
            print_report(&report);
            0
        }
        Err(err) => {
            eprintln!("codescene-xray: {err}");
            1
        }
    }
}

#[must_use]
pub fn run(args: &[String]) -> i32 {
    let file_path = if args.is_empty() {
        eprintln!("usage: coherence-bootstrap codescene-xray [--trigger] <file-path>");
        return 1;
    } else if args[0] == "--trigger" {
        if args.len() < 2 {
            eprintln!("usage: coherence-bootstrap codescene-xray --trigger <file-path>");
            return 1;
        }
        if let Err(e) = trigger_and_wait() {
            eprintln!("codescene-xray: {e}");
            return 1;
        }
        &args[1]
    } else {
        &args[0]
    };

    display_xray(Path::new(file_path))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn parse_or_panic(json: &str) -> (Option<String>, Vec<StructuralIssue>) {
        parse_cs_review_output(json).expect("parse should succeed")
    }

    #[test]
    fn parse_cs_review_empty() {
        let json = r#"{"score":10.0,"review":[]}"#;
        let (score, issues) = parse_or_panic(json);
        assert_eq!(score, Some("10.00".to_string()));
        assert!(issues.is_empty());
    }

    #[test]
    fn parse_cs_review_bumpy_road() {
        let json = r#"{
            "score": 7.78,
            "review": [{
                "category": "Bumpy Road Ahead",
                "functions": [{
                    "title": "manifest_bound_catalog_name",
                    "details": "bumps = 2",
                    "start-line": 210,
                    "end-line": 234
                }]
            }]
        }"#;
        let (_score, issues) = parse_or_panic(json);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].category, "Bumpy Road Ahead");
        assert_eq!(issues[0].functions.len(), 1);
        assert_eq!(issues[0].functions[0].title, "manifest_bound_catalog_name");
        assert_eq!(issues[0].functions[0].line, 210);
        assert!(issues[0].functions[0].details.is_some());
    }

    #[test]
    fn parse_cs_review_unknown_category_ok() {
        let json = r#"{
            "score": 5.0,
            "review": [{
                "category": "Some New Smell",
                "functions": [{
                    "title": "foo",
                    "start-line": 42,
                    "end-line": 50
                }]
            }]
        }"#;
        let (_score, issues) = parse_or_panic(json);
        assert_eq!(issues[0].category, "Some New Smell");
    }

    #[test]
    fn parse_cs_review_invalid_json_errors() {
        let result = parse_cs_review_output("not json");
        assert!(result.is_err());
    }
}
