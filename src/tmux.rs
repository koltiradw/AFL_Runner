use anyhow::{bail, Result};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::{env, path::PathBuf};

#[derive(Debug, Clone)]
pub struct Session {
    pub name: String,
    pub commands: Vec<String>,
    pub log_file: PathBuf,
}

impl Session {
    pub fn new(session_name: &str, cmds: &[String]) -> Self {
        let commands = cmds
            .iter()
            .map(|c| c.replace('"', "\\\""))
            .collect::<Vec<String>>();
        let log_file = PathBuf::from(format!("/tmp/tmux_{session_name}.log"));
        if log_file.exists() {
            std::fs::remove_file(&log_file).unwrap();
        }
        Self {
            name: session_name.to_string(),
            commands,
            log_file,
        }
    }

    fn in_tmux() -> bool {
        env::var("TMUX").is_ok()
    }

    fn create_bash_script(&self) -> Result<String> {
        let mut engine = upon::Engine::new();
        engine.add_template("afl_fuzz", include_str!("util/tmux_template.txt"))?;
        engine
            .template("afl_fuzz")
            .render(upon::value! {
                session_name :self.name.clone(),
                commands : self.commands.clone(),
                log_file : self.log_file.to_str().unwrap().to_string(),
            })
            .to_string()
            .map_err(|e| anyhow::anyhow!("Error creating bash script: {}", e))
    }

    pub fn kill_session(&self) -> Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.arg("kill-session")
            .arg("-t")
            .arg(&self.name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut child = cmd.spawn()?;
        let _ = child.wait()?;
        Ok(())
    }

    fn find_first_window_id(&self) -> Result<String> {
        let output = Command::new("tmux")
            .args(["list-windows", "-t", &self.name])
            .output()?;

        if !output.status.success() {
            bail!("Failed to list tmux windows");
        }

        let output_str = String::from_utf8(output.stdout)?;
        let first_window = output_str.chars().next().unwrap();
        if first_window == '0' || first_window == '1' {
            Ok(first_window.to_string())
        } else {
            self.kill_session()?;
            bail!("Failed to find first window id");
        }
    }

    pub fn attach(&self) -> Result<()> {
        let get_first_window_id = self.find_first_window_id()?;
        let target = format!("{}:{}", &self.name, get_first_window_id);
        let mut cmd = Command::new("tmux");
        cmd.args(["attach-session", "-t", &target])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let mut child = cmd.spawn()?;
        let _ = child.wait()?;
        Ok(())
    }

    fn mkdir_helper(dir: &Path, check_empty: bool) {
        assert!(!dir.is_file(), "{} is a file", dir.display());
        if check_empty {
            let is_empty = dir.read_dir().map_or(true, |mut i| i.next().is_none());
            if !is_empty {
                println!("Directory {} is not empty. Clean it [Y/n]? ", dir.display());
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read input");
                let input = input.trim().to_lowercase().chars().next().unwrap_or('y');
                if input == 'y' || input == '\n' {
                    fs::remove_dir_all(dir).expect("Failed to remove directory");
                }
            }
        }
        if !dir.exists() {
            fs::create_dir(dir).expect("Failed to create directory");
        }
    }

    pub fn run(&self) -> Result<()> {
        if Self::in_tmux() {
            bail!("Already in tmux session. Nested tmux sessions are not supported.");
        }

        // Find the input directory followed after -i in self.commands[0]
        let indir = PathBuf::from(
            self.commands[0]
                .split_whitespace()
                .skip_while(|&x| x != "-i")
                .skip(1)
                .next()
                .unwrap(),
        );
        let outdir = PathBuf::from(
            self.commands[0]
                .split_whitespace()
                .skip_while(|&x| x != "-o")
                .skip(1)
                .next()
                .unwrap(),
        );

        Self::mkdir_helper(&indir, false);
        if !indir.read_dir().unwrap().next().is_none() {
            fs::write(indir.join("1"), "fuzz").expect("Failed to write to file");
        }
        Self::mkdir_helper(&outdir, true);

        println!(
            "Starting tmux session '{}' for {} generated commands...",
            self.name,
            self.commands.len()
        );

        print!("Continue [Y/n]? ");
        std::io::stdout().flush()?;
        let user_input = std::io::stdin()
            .bytes()
            .next()
            .and_then(std::result::Result::ok)
            .map_or("y".to_string(), |byte| {
                let b = byte as char;
                if b.is_ascii_alphabetic() {
                    b.to_lowercase().to_string()
                } else if b == '\n' {
                    'y'.to_string()
                } else {
                    b.to_string()
                }
            });
        if user_input != "y" {
            bail!("Aborting");
        }

        if let Ok(templ) = self.create_bash_script() {
            let mut cmd = Command::new("bash");
            cmd.arg("-c")
                .arg(templ)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            let mut child = cmd.spawn()?;
            let _ = child.wait()?;
        }
        Ok(())
    }
}
