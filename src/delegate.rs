use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};
use which::which_global;

pub struct OpDelegate {
    command: PathBuf,
}

impl OpDelegate {
    const ENV_VAR_NAME: &'static str = "OP_FAST_COMMAND";

    pub fn new() -> Result<Self> {
        if let Ok(var) = env::var(Self::ENV_VAR_NAME) {
            let path = fs::canonicalize(&var)
                .with_context(|| format!("Failed to resolve {}: {}", Self::ENV_VAR_NAME, var))?;
            return Ok(Self { command: path });
        }

        let command = which_global("op")
            .context("Could not find `op` command in PATH")?
            .canonicalize()
            .context("Failed to canonicalize `op` path")?;

        let command = match env::current_exe() {
            Ok(current_path) if current_path == command => {
                bail!(
                    "Detected self-reference. Set {} to the original `op` binary path",
                    Self::ENV_VAR_NAME
                )
            }
            _ => command,
        };

        Ok(Self { command })
    }

    pub fn read(&self, reference: &str) -> Result<String> {
        let output = Command::new(&self.command)
            .arg("read")
            .arg(reference)
            .output()
            .context("Failed to execute `op read`")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("op read failed: {}", stderr);
        }

        let value = String::from_utf8(output.stdout)
            .context("op read returned non-UTF8 output")?
            .trim_end()
            .to_string();

        Ok(value)
    }

    pub fn read_batch(&self, references: &[&str]) -> Result<HashMap<String, String>> {
        if references.is_empty() {
            return Ok(HashMap::new());
        }

        let tag = std::process::id();
        let template = build_batch_template(references, tag);

        let mut child = Command::new(&self.command)
            .arg("inject")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn `op inject`")?;

        let stdin = child.stdin.as_mut().context("Failed to open stdin")?;
        stdin
            .write_all(template.as_bytes())
            .context("Failed to write to op inject stdin")?;
        let _ = stdin;

        let output = child
            .wait_with_output()
            .context("Failed to wait for op inject")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("op inject failed: {}", stderr);
        }

        let stdout =
            String::from_utf8(output.stdout).context("op inject returned non-UTF8 output")?;

        let values = parse_batch_output(&stdout, references.len(), tag)?;

        let mut result = HashMap::new();
        for (reference, value) in references.iter().zip(values) {
            result.insert(reference.to_string(), value);
        }

        Ok(result)
    }

    pub fn command(&self) -> Command {
        Command::new(&self.command)
    }
}

fn sentinel(tag: u32, index: usize, end: bool) -> String {
    let kind = if end { "END" } else { "BEGIN" };
    format!("--OPFAST:{}:{}:{}--", tag, kind, index)
}

pub(crate) fn build_batch_template(references: &[&str], tag: u32) -> String {
    let mut template = String::new();
    for (i, reference) in references.iter().enumerate() {
        template.push_str(&sentinel(tag, i, false));
        template.push('\n');
        template.push_str(reference);
        template.push('\n');
        template.push_str(&sentinel(tag, i, true));
        template.push('\n');
    }
    template
}

pub(crate) fn parse_batch_output(output: &str, expected: usize, tag: u32) -> Result<Vec<String>> {
    let mut values: Vec<String> = Vec::with_capacity(expected);
    let mut current: Option<Vec<&str>> = None;

    for line in output.lines() {
        match &mut current {
            None => {
                if line == sentinel(tag, values.len(), false) {
                    current = Some(Vec::new());
                } else {
                    bail!("op inject output contains unexpected content outside a sentinel section");
                }
            }
            Some(lines) => {
                if line == sentinel(tag, values.len(), true) {
                    values.push(lines.join("\n"));
                    current = None;
                } else if line.starts_with(&format!("--OPFAST:{}:", tag)) {
                    // A sentinel for this tag in the wrong position means a
                    // value contained forged markers; refuse rather than risk
                    // shifted assignments.
                    bail!("op inject output contains a misplaced sentinel marker");
                } else {
                    lines.push(line);
                }
            }
        }
    }

    if current.is_some() {
        bail!("op inject output ended inside an unterminated sentinel section");
    }
    if values.len() != expected {
        bail!(
            "op inject returned {} sections for {} references",
            values.len(),
            expected
        );
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simulate what `op inject` produces for a template: each reference line
    // replaced by its value, all other lines passed through verbatim.
    fn simulate_inject(template: &str, values: &[&str]) -> String {
        let mut out = String::new();
        let mut value_iter = values.iter();
        for line in template.lines() {
            if line.starts_with("op://") {
                out.push_str(value_iter.next().expect("more refs than values"));
            } else {
                out.push_str(line);
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn template_wraps_each_reference_in_sentinels() {
        let template = build_batch_template(&["op://a/b/c", "op://d/e/f"], 42);
        assert_eq!(
            template,
            "--OPFAST:42:BEGIN:0--\nop://a/b/c\n--OPFAST:42:END:0--\n\
             --OPFAST:42:BEGIN:1--\nop://d/e/f\n--OPFAST:42:END:1--\n"
        );
    }

    #[test]
    fn parse_assigns_single_line_values_in_order() {
        let refs = ["op://a/b/c", "op://d/e/f"];
        let template = build_batch_template(&refs, 7);
        let output = simulate_inject(&template, &["first", "second"]);
        let values = parse_batch_output(&output, 2, 7).unwrap();
        assert_eq!(values, vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn parse_preserves_multi_line_values() {
        let refs = ["op://vault/ssh/key"];
        let template = build_batch_template(&refs, 7);
        let output = simulate_inject(&template, &["-----BEGIN KEY-----\nabc\n-----END KEY-----"]);
        let values = parse_batch_output(&output, 1, 7).unwrap();
        assert_eq!(values, vec!["-----BEGIN KEY-----\nabc\n-----END KEY-----".to_string()]);
    }

    #[test]
    fn parse_empty_value_yields_empty_string() {
        let refs = ["op://a/b/c"];
        let template = build_batch_template(&refs, 7);
        let output = simulate_inject(&template, &[""]);
        let values = parse_batch_output(&output, 1, 7).unwrap();
        assert_eq!(values, vec![String::new()]);
    }

    #[test]
    fn parse_regression_multiline_plus_empty_do_not_misassign() {
        // The upstream line-count bug: value "x\ny" plus value "" produced two
        // lines for two refs and mis-assigned "y" to the second reference.
        let refs = ["op://a/b/c", "op://d/e/f"];
        let template = build_batch_template(&refs, 7);
        let output = simulate_inject(&template, &["x\ny", ""]);
        let values = parse_batch_output(&output, 2, 7).unwrap();
        assert_eq!(values, vec!["x\ny".to_string(), String::new()]);
    }

    #[test]
    fn parse_fails_on_missing_end_marker() {
        let output = "--OPFAST:7:BEGIN:0--\nvalue\n";
        assert!(parse_batch_output(output, 1, 7).is_err());
    }

    #[test]
    fn parse_fails_on_section_count_mismatch() {
        let refs = ["op://a/b/c"];
        let template = build_batch_template(&refs, 7);
        let output = simulate_inject(&template, &["only"]);
        assert!(parse_batch_output(&output, 2, 7).is_err());
    }

    #[test]
    fn parse_fails_closed_when_a_value_forges_aligned_sentinels() {
        // Hostile value 0 forges an early END:0 plus a BEGIN:1 so that the
        // real markers land inside section 1 and the section count still
        // matches. Must error - never return shifted/garbage assignments.
        let refs = ["op://a/b/c", "op://d/e/f"];
        let template = build_batch_template(&refs, 7);
        let forged = "real\n--OPFAST:7:END:0--\n--OPFAST:7:BEGIN:1--\nPWNED";
        let output = simulate_inject(&template, &[forged, "actual"]);
        assert!(parse_batch_output(&output, 2, 7).is_err());
    }

    #[test]
    fn parse_fails_on_unexpected_content_outside_sections() {
        let refs = ["op://a/b/c"];
        let template = build_batch_template(&refs, 7);
        let mut output = String::from("unexpected preamble\n");
        output.push_str(&simulate_inject(&template, &["value"]));
        assert!(parse_batch_output(&output, 1, 7).is_err());
    }
}
