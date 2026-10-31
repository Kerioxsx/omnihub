//! Windows Task Scheduler tasks that run with the user's administrator
//! rights without a UAC prompt each time. Creating (and deleting) one needs
//! a single approval; running it later does not. OmniHub uses one for fast
//! drive scans when the user turns that on.

use std::path::Path;

pub const SCAN_TASK: &str = "OmniHub Fast Scan";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Task definition: run `command arguments` on demand, as the signed-in
/// user with highest privileges, never on a schedule.
pub fn task_xml(command: &Path, arguments: &str, description: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>{}</Description>
  </RegistrationInfo>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>Queue</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <ExecutionTimeLimit>PT2H</ExecutionTimeLimit>
    <Priority>5</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{}</Command>
      <Arguments>{}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        xml_escape(description),
        xml_escape(&command.to_string_lossy()),
        xml_escape(arguments)
    )
}

/// Task Scheduler reads task files as UTF-16 with a byte-order mark.
pub fn utf16_with_bom(s: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn schtasks(args: &[&str]) -> Result<(), String> {
        let out = Command::new("schtasks").args(args).creation_flags(CREATE_NO_WINDOW).output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(())
        } else {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if msg.is_empty() { format!("schtasks exited with {}", out.status) } else { msg })
        }
    }

    pub fn installed(name: &str) -> bool {
        schtasks(&["/Query", "/TN", name]).is_ok()
    }

    /// Create or replace a task (needs administrator rights).
    pub fn install(name: &str, xml: &str) -> Result<(), String> {
        let file = std::env::temp_dir().join(format!("omnihub-task-{}.xml", std::process::id()));
        std::fs::write(&file, super::utf16_with_bom(xml)).map_err(|e| e.to_string())?;
        let r = schtasks(&["/Create", "/TN", name, "/XML", &file.to_string_lossy(), "/F"]);
        let _ = std::fs::remove_file(&file);
        r
    }

    pub fn remove(name: &str) -> Result<(), String> {
        schtasks(&["/Delete", "/TN", name, "/F"])
    }

    /// Start a task now (no prompt; the task decides its own rights).
    pub fn run(name: &str) -> Result<(), String> {
        schtasks(&["/Run", "/TN", name])
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn installed(_name: &str) -> bool {
        false
    }
    pub fn install(_name: &str, _xml: &str) -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn remove(_name: &str) -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn run(_name: &str) -> Result<(), String> {
        Err("Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_definition() {
        let x = task_xml(Path::new(r"C:\Users\A & B\AppData\Local\OmniHub\OmniHub.exe"), r#"--omnihub-helper scan-queue "C:\x\scans""#, "Fast scans <without> prompts");
        assert!(x.contains("<RunLevel>HighestAvailable</RunLevel>"));
        assert!(x.contains(r"<Command>C:\Users\A &amp; B\AppData\Local\OmniHub\OmniHub.exe</Command>"));
        assert!(x.contains("<Arguments>--omnihub-helper scan-queue &quot;C:\\x\\scans&quot;</Arguments>"));
        assert!(x.contains("Fast scans &lt;without&gt; prompts"));
        assert!(!x.contains("<Triggers>"), "runs only on demand");
        let b = utf16_with_bom("<a/>");
        assert_eq!(&b[..4], &[0xFF, 0xFE, b'<', 0]);
    }
}
