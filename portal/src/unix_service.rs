//! Process-based stop/status for Unix hosts without a service manager.
//!
//! Windows uses its supervisor (`windows_start`), macOS uses launchd
//! (`macos_supervisor`). Desktop Linux is expected to run under systemd or
//! an equivalent. This module covers the remaining hosts — Termux on
//! Android, SSH boxes, containers — where Portal runs in the foreground and
//! `status`/`stop` must talk to the processes directly.

use anyhow::{bail, Result};

/// Match Portal processes by `/proc/<pid>/comm`. The kernel truncates comm
/// to 15 characters, which fits both historical binary names.
const COMM_NAMES: [&str; 2] = ["heart-portal", "portal"];

fn is_portal_comm(comm: &str) -> bool {
    COMM_NAMES.contains(&comm)
}

fn cmdline_of(pid_dir: &std::path::Path) -> String {
    std::fs::read_to_string(pid_dir.join("cmdline"))
        .map(|raw| {
            raw.split('\0')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Find Portal processes by scanning `/proc`, excluding the current process.
fn portal_pids() -> Vec<(u32, String)> {
    let own = std::process::id();
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return found;
    };
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().into_string().ok() else {
            continue;
        };
        if !name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if pid == own {
            continue;
        }
        let Ok(comm) = std::fs::read_to_string(entry.path().join("comm")) else {
            continue;
        };
        if is_portal_comm(comm.trim_end()) {
            found.push((pid, cmdline_of(&entry.path())));
        }
    }
    found.sort();
    found
}

fn pid_alive(pid: u32) -> bool {
    std::path::Path::new("/proc").join(pid.to_string()).exists()
}

pub async fn run(action: &str) -> Result<()> {
    let pids = portal_pids();
    match action {
        "status" => {
            if pids.is_empty() {
                println!("Portal is not running.");
                println!("No service manager on this platform; start Portal in the foreground with `heart-portal`.");
                return Ok(());
            }
            for (pid, cmd) in &pids {
                println!("Portal running: pid {pid}  {cmd}");
            }
            println!("{} Portal process(es) running.", pids.len());
            Ok(())
        }
        "stop" => {
            if pids.is_empty() {
                println!("Portal is not running.");
                return Ok(());
            }
            for (pid, _) in &pids {
                // Signal-only kill; the pid set comes from our own /proc scan.
                unsafe { libc::kill(*pid as i32, libc::SIGTERM) };
            }
            // Give the processes a grace period to exit before reporting.
            let deadline = tokio::time::Instant::now()
                + std::time::Duration::from_secs(5);
            let mut remaining: Vec<u32> = pids.iter().map(|(pid, _)| *pid).collect();
            while !remaining.is_empty() && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                remaining.retain(|pid| pid_alive(*pid));
            }
            if remaining.is_empty() {
                println!("Stopped {} Portal process(es).", pids.len());
            } else {
                let list = remaining
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("Sent SIGTERM but still running: {list} (try `kill -9 {list}`)");
            }
            Ok(())
        }
        other => bail!("unsupported action: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comm_matching() {
        assert!(is_portal_comm("heart-portal"));
        assert!(is_portal_comm("portal"));
        assert!(!is_portal_comm("heart-portal2"));
        assert!(!is_portal_comm("qemu-aarch64-sta"));
    }

    #[test]
    fn scan_excludes_self_and_returns_sorted() {
        // /proc always exists on the hosts this module compiles for.
        let pids = portal_pids();
        let mut sorted = pids.clone();
        sorted.sort();
        assert_eq!(pids, sorted);
        assert!(!pids.iter().any(|(pid, _)| *pid == std::process::id()));
    }
}
