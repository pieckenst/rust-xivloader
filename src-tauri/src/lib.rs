mod ffxiv;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Trace)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![ 
            // fully standard tauri handling
            // ui cannot access any commands without putting it here
            greet,
            ffxiv::launch_game,
            ffxiv::get_news,
            ffxiv::get_banners
        ])
        .run(tauri::generate_context!())
// standard tauri error handler
       // not like rust supports any other way
        .expect("error while running tauri application");
}
