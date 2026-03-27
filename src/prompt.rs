use nix::unistd;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use std::path::Path;

/// Context needed for rendering dynamic prompt elements.
pub struct PromptContext {
    /// Exit status of the last command.
    pub last_status: i32,
    /// Duration of the last command in milliseconds.
    pub last_duration_ms: u64,
    /// Environment variables (subset needed for prompt).
    pub env: std::collections::HashMap<String, String>,
}

impl Default for PromptContext {
    fn default() -> Self {
        Self {
            last_status: 0,
            last_duration_ms: 0,
            env: std::env::vars().collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Prompt {
    spans: Vec<Span>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Color {
    Red,
    Blue,
    Yellow,
    Green,
    Cyan,
    Magenta,
    Bold,
    Underline,
    Reset,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Condition {
    InRepo,
    InRemote,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Span {
    Literal(String),
    Color(Color),
    Username,
    Hostname,
    CurrentDir,
    Newline,
    RepoStatus,
    If {
        condition: Condition,
        then_part: Vec<Span>,
        else_part: Vec<Span>,
    },
    // New spans
    LastStatus,
    LastDuration,
    Time,
    Date,
    GitBranch,
    GitStatus,
    Load,
    EnvVar(String),
}

#[derive(Parser)]
#[grammar = "prompt.pest"]
struct PromptParser;

fn visit_prompt(pair: Pair<Rule>) -> Prompt {
    let mut spans = Vec::new();
    for pair in pair.into_inner() {
        let span = match pair.as_rule() {
            Rule::username_span => Span::Username,
            Rule::hostname_span => Span::Hostname,
            Rule::current_dir_span => Span::CurrentDir,
            Rule::repo_status_span => Span::RepoStatus,
            Rule::newline_span => Span::Newline,
            Rule::reset_span => Span::Color(Color::Reset),
            Rule::bold_span => Span::Color(Color::Bold),
            Rule::underline_span => Span::Color(Color::Underline),
            Rule::red_span => Span::Color(Color::Red),
            Rule::blue_span => Span::Color(Color::Blue),
            Rule::green_span => Span::Color(Color::Green),
            Rule::yellow_span => Span::Color(Color::Yellow),
            Rule::cyan_span => Span::Color(Color::Cyan),
            Rule::magenta_span => Span::Color(Color::Magenta),
            Rule::literal_span => Span::Literal(pair.as_span().as_str().to_owned()),
            // New spans
            Rule::last_status_span => Span::LastStatus,
            Rule::last_duration_span => Span::LastDuration,
            Rule::time_span => Span::Time,
            Rule::date_span => Span::Date,
            Rule::git_branch_span => Span::GitBranch,
            Rule::git_status_span => Span::GitStatus,
            Rule::load_span => Span::Load,
            Rule::env_var_span => {
                // Extract variable name from the span
                let var_name = pair.as_str()
                    .trim_start_matches("\\{env:")
                    .trim_end_matches('}')
                    .to_string();
                Span::EnvVar(var_name)
            }
            Rule::if_span => {
                let mut inner = pair.into_inner();
                let condition = match inner.next().unwrap().as_span().as_str() {
                    "in_repo" => Condition::InRepo,
                    "in_remote" => Condition::InRemote,
                    _ => unreachable!(),
                };

                let then_part = visit_prompt(inner.next().unwrap()).spans;
                let else_part = visit_prompt(inner.next().unwrap()).spans;
                Span::If {
                    condition,
                    then_part,
                    else_part,
                }
            }
            _ => unreachable!(),
        };

        spans.push(span);
    }

    Prompt { spans }
}

fn pairs2prompt(mut pairs: Pairs<Rule>) -> Prompt {
    visit_prompt(pairs.next().unwrap())
}

pub fn parse_prompt(prompt: &str) -> Result<Prompt, pest::error::Error<Rule>> {
    PromptParser::parse(Rule::prompt, prompt).map(pairs2prompt)
}

// FIXME: remove unsafe or use external crate
fn get_current_username() -> String {
    let mut passwd_buf = Vec::with_capacity(512);
    let mut passwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut result = std::ptr::null_mut();
    unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut passwd,
            passwd_buf.as_mut_ptr(),
            passwd_buf.capacity(),
            &mut result,
        );
    }

    if result.is_null() {
        "".to_owned()
    } else {
        let ptr = passwd.pw_name as *const _;
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(ptr);
            cstr.to_string_lossy().into_owned()
        }
    }
}

fn get_hostname() -> String {
    let hostname_cstr = unistd::gethostname().expect("failed to get hostname");
    let hostname = hostname_cstr
        .to_str()
        .expect("Hostname is not valid utf-8 string");
    hostname.to_owned()
}

fn get_repo_branch(git_dir: &str) -> String {
    let rebase_i_file = Path::new(git_dir).join("rebase-merge/head-name");
    if rebase_i_file.exists() {
        return std::fs::read_to_string(rebase_i_file)
            .unwrap()
            .trim()
            .to_owned();
    }

    let result = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output();

    if let Ok(output) = result {
        String::from_utf8_lossy(&output.stdout)
            .into_owned()
            .trim()
            .to_owned()
    } else {
        "".to_owned()
    }
}

fn get_git_dir() -> String {
    let result = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .output();

    if let Ok(output) = result {
        String::from_utf8_lossy(&output.stdout)
            .into_owned()
            .trim()
            .to_owned()
    } else {
        warn!("failed to get the git dir");
        "".to_owned()
    }
}

fn is_repo_modified() -> bool {
    std::process::Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .stderr(std::process::Stdio::null())
        .output()
        .map(|output| !output.stdout.is_empty())
        .unwrap_or(false)
}

fn get_repo_action(git_dir: &str) -> Option<&'static str> {
    let git_dir = Path::new(git_dir);
    if git_dir.join("rebase-merge/interactive").exists() {
        return Some("rebase-i");
    }

    if git_dir.join("MERGE_HEAD").exists() {
        return Some("merge");
    }

    if git_dir.join("BISECT_LOG").exists() {
        return Some("bisect");
    }

    None
}

// TODO: Support other systems like SVN.
fn get_repo_info() -> String {
    let git_dir = get_git_dir();
    let mut columns = Vec::with_capacity(2);

    let mut branch = get_repo_branch(&git_dir);
    if is_repo_modified() {
        branch.push('*');
    }
    columns.push(branch);

    if let Some(action) = get_repo_action(&git_dir) {
        columns.push(action.to_owned());
    }

    columns.join("|")
}

/// Get just the git branch name.
fn get_git_branch_only() -> String {
    get_repo_branch(&get_git_dir())
}

/// Get git status indicator (* if modified, empty otherwise).
fn get_git_status_indicator() -> String {
    if is_repo_modified() { "*".to_string() } else { "".to_string() }
}

lazy_static! {
    // Use lazy_static to cache the result.
    static ref IN_REMOTE: bool = {
        std::env::var("SSH_CLIENT").is_ok()
    };
}

fn evaluate_condition(cond: &Condition) -> bool {
    match cond {
        Condition::InRepo => {
            std::process::Command::new("git")
                .arg("rev-parse")
                .arg("--is-inside-work-tree")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        }
        Condition::InRemote => *IN_REMOTE,
    }
}

/// Format duration in human-readable form.
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60000;
        let secs = (ms % 60000) / 1000;
        format!("{}m{}s", mins, secs)
    }
}

/// Get current time as HH:MM:SS.
fn get_current_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let hours = (now / 3600) % 24;
    let mins = (now / 60) % 60;
    let secs = now % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

/// Get current date as YYYY-MM-DD.
fn get_current_date() -> String {
    // Use chrono-like formatting with std
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Simple date calculation (approximate, doesn't handle leap seconds)
    let days = now / 86400;
    // Unix epoch was 1970-01-01
    let mut year = 1970;
    let mut remaining_days = days;
    
    loop {
        let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
            366
        } else {
            365
        };
        
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }
    
    let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    
    let mut month = 1;
    for &days_in_month in &days_in_months {
        let dim = if month == 2 && is_leap { 29 } else { days_in_month };
        if remaining_days < dim {
            break;
        }
        remaining_days -= dim;
        month += 1;
    }
    
    let day = remaining_days + 1;
    
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Get system load average.
fn get_load_average() -> String {
    // Linux: read from /proc/loadavg
    if let Ok(contents) = std::fs::read_to_string("/proc/loadavg") {
        if let Some(load1) = contents.split_whitespace().next() {
            return load1.to_string();
        }
    }
    
    // macOS: use sysctl
    let output = std::process::Command::new("sysctl")
        .arg("-n")
        .arg("vm.loadavg")
        .output();
    
    if let Ok(output) = output {
        let s = String::from_utf8_lossy(&output.stdout);
        // Output format: "{ 0.12 0.15 0.10 }"
        if let Some(load) = s.split_whitespace().nth(1) {
            return load.to_string();
        }
    }
    
    "".to_string()
}

/// Returns the length of the last line excluding escape sequences.
pub fn draw_prompt(prompt: &Prompt) -> (String, usize) {
    draw_prompt_with_context(prompt, &PromptContext::default())
}

/// Draw prompt with context for dynamic elements.
pub fn draw_prompt_with_context(prompt: &Prompt, ctx: &PromptContext) -> (String, usize) {
    let mut len = 0;
    let mut buf = String::new();
    for span in &prompt.spans {
        match span {
            Span::Literal(s) => {
                len += s.len();
                buf.push_str(s)
            }
            Span::Color(c) => match c {
                Color::Red => buf.push_str("\x1b[31m"),
                Color::Blue => buf.push_str("\x1b[34m"),
                Color::Yellow => buf.push_str("\x1b[33m"),
                Color::Green => buf.push_str("\x1b[32m"),
                Color::Cyan => buf.push_str("\x1b[36m"),
                Color::Magenta => buf.push_str("\x1b[35m"),
                Color::Bold => buf.push_str("\x1b[1m"),
                Color::Underline => buf.push_str("\x1b[4m"),
                Color::Reset => buf.push_str("\x1b[0m"),
            },
            Span::Newline => {
                len = 0;
                buf.push('\n')
            }
            Span::Username => {
                let username = get_current_username();
                len += username.len();
                buf.push_str(&username)
            }
            Span::Hostname => {
                let hostname = get_hostname();
                len += hostname.len();
                buf.push_str(&hostname)
            }
            Span::CurrentDir => {
                if let Ok(current_dir) = std::env::current_dir() {
                    let mut path = current_dir.to_str().unwrap().to_string();

                    if let Some(home_dir) = dirs::home_dir() {
                        let home_dir = home_dir.to_str().unwrap();
                        if path.starts_with(&home_dir) {
                            path = path.replace(home_dir, "~");
                        }
                    }

                    len += path.len();
                    buf.push_str(&path);
                }
            }
            Span::RepoStatus => {
                let info = get_repo_info();
                len += info.len();
                buf.push_str(&info)
            }
            Span::If {
                condition,
                then_part,
                else_part,
            } => {
                let spans = if evaluate_condition(condition) {
                    then_part
                } else {
                    else_part
                };

                let (result, result_len) = draw_prompt_with_context(
                    &Prompt { spans: spans.to_vec() },
                    ctx,
                );
                len += result_len;
                buf.push_str(&result)
            }
            // New spans
            Span::LastStatus => {
                let status = if ctx.last_status == 0 {
                    format!("{}", ctx.last_status)
                } else {
                    // Red for non-zero exit
                    format!("\x1b[31m{}\x1b[0m", ctx.last_status)
                };
                len += ctx.last_status.to_string().len();
                buf.push_str(&status);
            }
            Span::LastDuration => {
                let dur = format_duration(ctx.last_duration_ms);
                len += dur.len();
                buf.push_str(&dur);
            }
            Span::Time => {
                let time = get_current_time();
                len += time.len();
                buf.push_str(&time);
            }
            Span::Date => {
                let date = get_current_date();
                len += date.len();
                buf.push_str(&date);
            }
            Span::GitBranch => {
                let branch = get_git_branch_only();
                len += branch.len();
                buf.push_str(&branch);
            }
            Span::GitStatus => {
                let status = get_git_status_indicator();
                len += status.len();
                buf.push_str(&status);
            }
            Span::Load => {
                let load = get_load_average();
                len += load.len();
                buf.push_str(&load);
            }
            Span::EnvVar(name) => {
                let value = ctx.env.get(name)
                    .cloned()
                    .unwrap_or_else(|| std::env::var(&name).unwrap_or_default());
                len += value.len();
                buf.push_str(&value);
            }
        }
    }

    (buf, len)
}

#[test]
fn test_prompt_parser() {
    assert_eq!(
        parse_prompt("\\{username} at \\{hostname} in \\{current_dir}\\n$ "),
        Ok(Prompt {
            spans: vec![
                Span::Username,
                Span::Literal(" at ".into()),
                Span::Hostname,
                Span::Literal(" in ".into()),
                Span::CurrentDir,
                Span::Newline,
                Span::Literal("$ ".into()),
            ]
        })
    );

    assert_eq!(
        parse_prompt("\\{current_dir} \\if{in_repo}{[\\{repo_status}]}{} $ "),
        Ok(Prompt {
            spans: vec![
                Span::CurrentDir,
                Span::Literal(" ".into()),
                Span::If {
                    condition: Condition::InRepo,
                    then_part: vec![
                        Span::Literal("[".into()),
                        Span::RepoStatus,
                        Span::Literal("]".into()),
                    ],
                    else_part: vec![]
                },
                Span::Literal(" $ ".into()),
            ]
        })
    );
    
    // Test new spans
    assert_eq!(
        parse_prompt("\\{git_branch}\\{git_status} $ "),
        Ok(Prompt {
            spans: vec![
                Span::GitBranch,
                Span::GitStatus,
                Span::Literal(" $ ".into()),
            ]
        })
    );
    
    assert_eq!(
        parse_prompt("[\\{last_status}] "),
        Ok(Prompt {
            spans: vec![
                Span::Literal("[".into()),
                Span::LastStatus,
                Span::Literal("] ".into()),
            ]
        })
    );
}

#[cfg(test)]
mod benchmarks {
    use super::*;
    use test::Bencher;

    #[bench]
    fn simple_prompt_rendering(b: &mut Bencher) {
        let prompt = parse_prompt("$ ").unwrap();
        b.iter(|| {
            draw_prompt(&prompt);
        });
    }

    #[bench]
    fn complex_prompt_rendering(b: &mut Bencher) {
        let prompt_fmt =
            "\\{cyan}\\{bold}\\{username}@\\{hostname}:\\{reset} \\{current_dir} $\\{reset} ";
        let prompt = parse_prompt(prompt_fmt).unwrap();

        b.iter(|| {
            draw_prompt(&prompt);
        });
    }
}
