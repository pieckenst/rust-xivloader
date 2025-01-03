use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use std::fs;
use std::process::{Command, Child};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, REFERER, CONTENT_TYPE};
use std::io::Read;
use tracing::{info, warn, error, debug};
use std::time::Instant;

// Add to top of file
#[derive(Debug)]
pub struct GameLaunchMetrics {
    login_time_ms: u64,
    sid_fetch_time_ms: u64,
    game_start_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LaunchConfig {
    // Mandatory fields
    game_path: String,
    username: String,
    password: String,
    otp: Option<String>,
    language: i32,
    dx11: bool,
    expansion_level: i32,
    is_steam: bool,
    region: i32,
    
    // Optional fields
    #[serde(default)]
    is_free_trial: bool,
    #[serde(default = "default_dalamud_enabled")]
    dalamud_enabled: bool,
    #[serde(default = "default_dpi_awareness")]
    dpi_awareness: String,
    #[serde(default)]
    additional_launch_args: String,
    #[serde(default = "default_encrypt_arguments")]
    encrypt_arguments: bool,
}

fn default_dalamud_enabled() -> bool {
    false
}

fn default_dpi_awareness() -> String {
    "Aware".to_string()
}

fn default_encrypt_arguments() -> bool {
    false
}


#[tauri::command]
pub async fn launch_game(config: LaunchConfig) -> Result<String, String> {
    let start = Instant::now();
    info!("Starting game launch process with config: {:?}", config);
    
    
    
    let login_start = Instant::now();
    debug!("Attempting to get real SID for user {}", config.username);
    match get_real_sid(
        &config.game_path,
        &config.username,
        &config.password,
        config.otp.as_deref(),
        config.is_steam
    ).await {
        Ok(sid) => {
            let sid_time = login_start.elapsed().as_millis() as u64;
            info!("Successfully obtained SID in {}ms", sid_time);
            
            let game_start = Instant::now();
            debug!("Starting game process with SID");
            match start_game(
                &config.game_path,
                &sid,
                config.language,
                config.dx11,
                config.expansion_level,
                config.is_steam,
                config.region
            ) {
                Ok(_) => {
                    let metrics = GameLaunchMetrics {
                        login_time_ms: login_start.elapsed().as_millis() as u64,
                        sid_fetch_time_ms: sid_time,
                        game_start_time_ms: game_start.elapsed().as_millis() as u64,
                    };
                    info!("Game launched successfully. Launch metrics: {:?}", metrics);
                    Ok("Game launched successfully".to_string())
                },
                Err(e) => {
                    error!("Failed to start game process: {}", e);
                    Err(format!("Failed to start game: {}", e))
                }
            }
        },
        Err(e) => {
            error!("Failed to obtain SID: {}", e);
            Err(format!("Failed to get session ID: {}", e))
        }
    }
}

async fn get_real_sid(
    game_path: &str,
    username: &str,
    password: &str,
    otp: Option<&str>,
    is_steam: bool
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let stored = get_stored(is_steam).await?;
    
    let mut form = std::collections::HashMap::new();
    form.insert("_STORED_", stored);
    form.insert("sqexid", username.to_string());
    form.insert("password", password.to_string());
    form.insert("otppw", otp.unwrap_or("").to_string());

    let response = client.post("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/login.send")
        .header(USER_AGENT, get_user_agent())
        .header(REFERER, format!("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", 
            if is_steam { "1" } else { "0" }))
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body = response.text().await.map_err(|e| e.to_string())?;
    
    // Extract SID using regex
    let re = regex::Regex::new(r"sid,(?P<sid>.*),terms").unwrap();
    match re.captures(&body) {
        Some(caps) => Ok(caps["sid"].to_string()),
        None => Err("Failed to extract session ID".to_string())
    }
}

fn start_game(
    game_path: &str,
    sid: &str,
    language: i32,
    dx11: bool,
    expansion_level: i32,
    is_steam: bool,
    region: i32
) -> Result<Child, std::io::Error> {
    let executable = if dx11 {
        format!("{}/game/ffxiv_dx11.exe", game_path)
    } else {
        format!("{}/game/ffxiv.exe", game_path)
    };

    let mut command = Command::new(executable);
    
    command.args(&[
        &format!("DEV.TestSID={}", sid),
        &format!("DEV.MaxEntitledExpansionID={}", expansion_level),
        &format!("language={}", language),
        &format!("region={}", region)
    ]);

    if is_steam {
        command.env("IS_FFXIV_LAUNCH_FROM_STEAM", "1");
        command.arg("IsSteam=1");
    }

    command.spawn()
}

async fn get_stored(is_steam: bool) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", 
        if is_steam { "1" } else { "0" }
    );

    let response = client.get(&url)
        .header(USER_AGENT, get_user_agent())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body = response.text().await.map_err(|e| e.to_string())?;
    
    let re = regex::Regex::new(r#"<input.*?name="_STORED_".*?value="([^"]*)"#).unwrap();

    match re.captures(&body) {
        Some(caps) => Ok(caps.get(1).unwrap().as_str().to_string()),
        None => Err("Could not find _STORED_ value".to_string())
    }
}

fn get_user_agent() -> String {
    format!("SQEXAuthor/2.0.0(Windows 6.2; ja-jp; {})", make_computer_id())
}

fn make_computer_id() -> String {
    use std::env;
    
    let machine_name = env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
    let user_name = env::var("USERNAME").unwrap_or_default();
    let os_version = "Windows 10.0";
    let processor_count = num_cpus::get();

    let hash_string = format!("{}{}{}{}", machine_name, user_name, os_version, processor_count);
    let mut hasher = Sha1::new();
    hasher.update(hash_string.as_bytes());
    let hash = hasher.finalize();

    let mut bytes = [0u8; 5];
    bytes[1..].copy_from_slice(&hash[0..4]);
    
    let checksum = !(bytes[1].wrapping_add(bytes[2]).wrapping_add(bytes[3]).wrapping_add(bytes[4]));
    bytes[0] = checksum;

    hex::encode(bytes)
}
