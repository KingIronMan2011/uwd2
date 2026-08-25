use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const TASK_NAME: &str = "UWD2";
const INSTALL_SUBDIR: &str = r"Programs\UWD2";
const EXE_NAME: &str = "uwd2.exe";

pub fn install_dir() -> PathBuf {
    PathBuf::from(env::var("LOCALAPPDATA").expect("LOCALAPPDATA not set")).join(INSTALL_SUBDIR)
}

pub fn installed_exe() -> PathBuf {
    install_dir().join(EXE_NAME)
}

pub fn is_running_from_install_dir() -> bool {
    let current = env::current_exe().unwrap_or_default();
    let target = installed_exe();
    let current = fs::canonicalize(&current).unwrap_or(current);
    let target = fs::canonicalize(&target).unwrap_or(target);
    current == target
}

pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

/// Re-launches the current exe elevated via UAC, forwarding current args.
pub fn relaunch_elevated() {
    let exe = env::current_exe().unwrap();
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str = args
        .iter()
        .map(|arg| quote_windows_arg(arg))
        .collect::<Vec<_>>()
        .join(" ");

    let exe_wide: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();

    unsafe {
        if args_str.is_empty() {
            ShellExecuteW(
                None,
                w!("runas"),
                PCWSTR(exe_wide.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOW,
            );
        } else {
            let args_wide: Vec<u16> = args_str.encode_utf16().chain(Some(0)).collect();
            ShellExecuteW(
                None,
                w!("runas"),
                PCWSTR(exe_wide.as_ptr()),
                PCWSTR(args_wide.as_ptr()),
                PCWSTR::null(),
                SW_SHOW,
            );
        }
    }
}

fn quote_windows_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_owned();
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for character in arg.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(character);
                backslashes = 0;
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

pub fn install() {
    install_self();
    add_to_user_path();
    create_scheduled_task();
    println!("Installed! UWD2 will run at logon with high priority.");
}

pub fn remove() {
    delete_scheduled_task();
    remove_from_user_path();
    let dir = install_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir).ok();
        println!("Removed install directory.");
    }
    println!("Uninstalled.");
}

fn install_self() {
    let dir = install_dir();
    fs::create_dir_all(&dir).expect("failed to create install directory");
    let src = env::current_exe().expect("failed to get current exe path");
    let dst = installed_exe();
    fs::copy(&src, &dst).expect("failed to copy exe to install directory");
    println!("Copied to {}.", dst.display());
}

fn add_to_user_path() {
    let dir = install_dir();
    let dir_str = dir.to_string_lossy();
    let script = format!(
        "$p=[Environment]::GetEnvironmentVariable('PATH','User'); \
         if($p -notlike '*{dir_str}*'){{ \
           [Environment]::SetEnvironmentVariable('PATH',\"$p;{dir_str}\",'User') \
         }}",
        dir_str = dir_str
    );
    let ok = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("Added to user PATH.");
    } else {
        eprintln!("Warning: failed to add to PATH.");
    }
}

fn remove_from_user_path() {
    let dir = install_dir();
    let dir_str = dir.to_string_lossy();
    let script = format!(
        "$p=[Environment]::GetEnvironmentVariable('PATH','User'); \
         $new=($p -split ';' | Where-Object {{ $_ -ne '{dir_str}' }}) -join ';'; \
         [Environment]::SetEnvironmentVariable('PATH',$new,'User')",
        dir_str = dir_str
    );
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .ok();
    println!("Removed from user PATH.");
}

pub fn create_scheduled_task() {
    let exe_quoted = format!("\"{}\"", installed_exe().display());
    let ok = Command::new("schtasks")
        .args([
            "/create",
            "/tn",
            TASK_NAME,
            "/tr",
            &exe_quoted,
            "/sc",
            "onlogon",
            "/rl",
            "highest",
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("Scheduled task created — runs at logon with highest privileges.");
    } else {
        eprintln!("Warning: failed to create scheduled task.");
    }
}

fn delete_scheduled_task() {
    Command::new("schtasks")
        .args(["/delete", "/tn", TASK_NAME, "/f"])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .ok();
    println!("Removed scheduled task.");
}

#[cfg(test)]
mod tests {
    use super::quote_windows_arg;

    #[test]
    fn quotes_windows_command_line_arguments() {
        assert_eq!(quote_windows_arg("inject"), "inject");
        assert_eq!(quote_windows_arg("two words"), "\"two words\"");
        assert_eq!(quote_windows_arg("a\"b"), "\"a\\\"b\"");
        assert_eq!(quote_windows_arg(r"C:\some dir\"), r#""C:\some dir\\""#);
    }
}
