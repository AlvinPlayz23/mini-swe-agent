use std::process::Command;
use std::env;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionResult {
    pub output: String,
    pub returncode: i32,
    pub exception_info: String,
}

pub struct LocalEnvironment {
    pub cwd: std::path::PathBuf,
}

impl LocalEnvironment {
    pub fn new(cwd: Option<std::path::PathBuf>) -> Self {
        Self {
            cwd: cwd.unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))),
        }
    }

    pub fn execute(&self, command: &str) -> ExecutionResult {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .current_dir(&self.cwd)
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.cwd)
                .output()
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined_output = if stderr.is_empty() {
                    stdout
                } else if stdout.is_empty() {
                    stderr
                } else {
                    format!("{}\n{}", stdout, stderr)
                };

                ExecutionResult {
                    output: combined_output,
                    returncode: output.status.code().unwrap_or(-1),
                    exception_info: String::new(),
                }
            }
            Err(e) => ExecutionResult {
                output: String::new(),
                returncode: -1,
                exception_info: format!("Failed to execute command: {}", e),
            },
        }
    }
}
