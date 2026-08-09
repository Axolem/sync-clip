//! Sync Clip Windows Shell entrypoint.

#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod windows_clipboard;

fn main() {
    #[cfg(windows)]
    {
        if let Err(err) = tray::run() {
            eprintln!("sync-clip: {err}");
            std::process::exit(1);
        }
    }

    #[cfg(not(windows))]
    {
        eprintln!(
            "sync-clip-shell: the Windows Shell binary only runs on Windows.\n\
             Core library tests: cargo test -p windows-shell\n\
             See apps/windows-shell/README.md"
        );
        std::process::exit(1);
    }
}
