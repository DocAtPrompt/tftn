// Unter Windows kein zusätzliches Konsolenfenster im Release-Bau.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    // `--mcp`: Brücke für KI-Assistenten über stdin/stdout, ohne Fenster.
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        return tftn_app_lib::mcp_bruecke();
    }
    tftn_app_lib::run();
    std::process::ExitCode::SUCCESS
}
