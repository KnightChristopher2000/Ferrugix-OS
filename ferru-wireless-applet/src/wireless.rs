use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::process::Command;

use gtk::gio;

#[derive(Clone, Debug)]
pub struct AccessPoint {
    pub ssid: String,
    pub signal: u8, // 0..=100
    pub secure: bool,
    pub active: bool,
}

fn io_other(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg.into())
}

/* -------------------- sync runner -------------------- */

fn run_sync(program: &str, args: &[&str]) -> Result<String, io::Error> {
    let out = Command::new(program).args(args).output()?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(io_other(if stderr.is_empty() {
            format!("{program} returned a failure status.")
        } else {
            stderr
        }));
    }

    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn nmcli_sync(args: &[&str]) -> Result<String, io::Error> {
    run_sync("nmcli", args)
}

/* -------------------- async runner (gio subprocess) -------------------- */

fn run_async<F>(program: &'static str, args: Vec<String>, cb: F)
where
    F: FnOnce(Result<(i32, String, String), io::Error>) + 'static,
{
    // argv needs &[&OsStr]
    let mut argv_os: Vec<OsString> = Vec::with_capacity(args.len() + 1);
    argv_os.push(OsString::from(program));
    argv_os.extend(args.into_iter().map(OsString::from));

    let argv_refs: Vec<&OsStr> = argv_os.iter().map(|s| s.as_os_str()).collect();

    let flags = gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE;

    let proc = match gio::Subprocess::newv(&argv_refs, flags) {
        Ok(p) => p,
        Err(e) => {
            cb(Err(io_other(format!("Failed to start {program}: {e}"))));
            return;
        }
    };

    let proc_for_cb = proc.clone();

    proc.communicate_utf8_async(
        None::<String>,
        None::<&gio::Cancellable>,
        move |res| match res {
            Ok((stdout, stderr)) => {
                let status = proc_for_cb.exit_status();
                let out = stdout.unwrap_or_default().to_string();
                let err = stderr.unwrap_or_default().to_string();
                cb(Ok((status, out, err)));
            }
            Err(e) => cb(Err(io_other(format!("Failed to read {program} output: {e}")))),
        },
    );
}

fn nmcli_async<F>(args: &[&str], cb: F)
where
    F: FnOnce(Result<String, io::Error>) + 'static,
{
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_async("nmcli", argv, move |res| match res {
        Ok((st, out, _err)) if st == 0 => cb(Ok(out)),
        Ok((_st, _out, err)) => {
            let msg = if err.trim().is_empty() {
                "nmcli returned a failure status.".to_string()
            } else {
                err
            };
            cb(Err(io_other(msg)));
        }
        Err(e) => cb(Err(e)),
    });
}

/* -------------------- Wi-Fi state -------------------- */

pub fn wifi_enabled() -> Result<bool, io::Error> {
    let txt = nmcli_sync(&["-t", "-f", "WIFI", "g"])?;
    Ok(txt.trim().eq_ignore_ascii_case("enabled"))
}

pub fn wifi_enabled_async<F>(cb: F)
where
    F: FnOnce(Result<bool, io::Error>) + 'static,
{
    nmcli_async(&["-t", "-f", "WIFI", "g"], move |res| {
        cb(res.map(|s| s.trim().eq_ignore_ascii_case("enabled")));
    });
}

pub fn set_wifi_enabled(enable: bool) -> Result<(), io::Error> {
    let onoff = if enable { "on" } else { "off" };
    let _ = nmcli_sync(&["radio", "wifi", onoff])?;
    Ok(())
}

pub fn set_wifi_enabled_async<F>(enable: bool, cb: F)
where
    F: FnOnce(Result<(), io::Error>) + 'static,
{
    let onoff = if enable { "on" } else { "off" };
    nmcli_async(&["radio", "wifi", onoff], move |res| cb(res.map(|_| ())));
}

/* -------------------- scanning -------------------- */

pub fn scan_access_points() -> Result<Vec<AccessPoint>, io::Error> {
    // NOTE: your nmcli does NOT support --separator, so do not use it.
    // Terse mode escapes ':' as '\:' which we handle.
    let args1 = [
        "-t",
        "-f",
        "IN-USE,SSID,SECURITY,SIGNAL",
        "dev",
        "wifi",
        "list",
        "--rescan",
        "auto",
    ];

    let txt = match nmcli_sync(&args1) {
        Ok(t) => t,
        Err(e) => {
            // fallback: older nmcli may not support --rescan auto
            let msg = e.to_string();
            if msg.contains("rescan") || msg.contains("--rescan") {
                nmcli_sync(&[
                    "-t",
                    "-f",
                    "IN-USE,SSID,SECURITY,SIGNAL",
                    "dev",
                    "wifi",
                    "list",
                ])?
            } else {
                return Err(e);
            }
        }
    };

    Ok(parse_ap_list(&txt))
}

pub fn scan_access_points_async<F>(cb: F)
where
    F: FnOnce(Result<Vec<AccessPoint>, io::Error>) + 'static,
{
    let args1 = [
        "-t",
        "-f",
        "IN-USE,SSID,SECURITY,SIGNAL",
        "dev",
        "wifi",
        "list",
        "--rescan",
        "auto",
    ];

    nmcli_async(&args1, move |res| match res {
        Ok(txt) => cb(Ok(parse_ap_list(&txt))),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("rescan") || msg.contains("--rescan") {
                nmcli_async(
                    &[
                        "-t",
                        "-f",
                        "IN-USE,SSID,SECURITY,SIGNAL",
                        "dev",
                        "wifi",
                        "list",
                    ],
                    move |res2| match res2 {
                        Ok(txt2) => cb(Ok(parse_ap_list(&txt2))),
                        Err(e2) => cb(Err(e2)),
                    },
                );
            } else {
                cb(Err(e));
            }
        }
    });
}

/* -------------------- nmcli parsing -------------------- */

fn split_escaped(line: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut esc = false;

    for ch in line.chars() {
        if esc {
            cur.push(ch);
            esc = false;
            continue;
        }
        if ch == '\\' {
            esc = true;
            continue;
        }
        if ch == sep {
            out.push(cur);
            cur = String::new();
            continue;
        }
        cur.push(ch);
    }

    out.push(cur);
    out
}

fn parse_ap_list(txt: &str) -> Vec<AccessPoint> {
    // Deduplicate SSIDs: nmcli often returns multiple BSSIDs per SSID.
    let mut map: HashMap<String, AccessPoint> = HashMap::new();

    for raw in txt.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
        let fields = split_escaped(raw, ':');
        if fields.len() < 4 {
            continue;
        }

        let in_use = fields[0].trim();
        let ssid_raw = fields[1].trim();
        let security = fields[2].trim();
        let signal_s = fields[3].trim();

        let active = in_use.contains('*');
        let ssid = if ssid_raw.is_empty() {
            "(Hidden network)".to_string()
        } else {
            ssid_raw.to_string()
        };

        let secure = !(security.is_empty() || security == "--");
        let signal: u8 = signal_s.parse::<u8>().unwrap_or(0).min(100);

        map.entry(ssid.clone())
            .and_modify(|ap| {
                ap.signal = ap.signal.max(signal);
                ap.secure |= secure;
                ap.active |= active;
            })
            .or_insert(AccessPoint {
                ssid,
                signal,
                secure,
                active,
            });
    }

    let mut aps: Vec<AccessPoint> = map.into_values().collect();

    aps.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then(b.signal.cmp(&a.signal))
            .then(a.ssid.to_lowercase().cmp(&b.ssid.to_lowercase()))
    });

    aps
}

pub fn signal_icon(signal: u8) -> &'static str {
    match signal {
        80..=100 => "network-wireless-signal-excellent-symbolic",
        60..=79 => "network-wireless-signal-good-symbolic",
        40..=59 => "network-wireless-signal-ok-symbolic",
        20..=39 => "network-wireless-signal-weak-symbolic",
        _ => "network-wireless-signal-none-symbolic",
    }
}

