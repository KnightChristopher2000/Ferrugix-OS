use std::process::Command;

pub fn open_wifi_settings() -> Result<(), String> {
    let tries: &[(&str, &[&str])] = &[
        ("ferru-control-center", &["network", "wifi"]),
        ("ferru-control-center", &["wifi"]),
        ("ferru-control-center", &["network"]),
        ("gnome-control-center", &["wifi"]),
        ("gnome-control-center", &["network"]),
    ];

    for (bin, args) in tries {
        if Command::new(bin).args(*args).spawn().is_ok() {
            return Ok(());
        }
    }

    Err("Could not open Wi-Fi settings (no compatible control center found).".to_string())
}
