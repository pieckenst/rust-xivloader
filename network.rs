use std::process::{Command, Child};
use std::collections::HashMap;
use std::io::{self, Write};
use std::io::{self, Read};
use std::fs::File;
use regex::Regex;
use reqwest::blocking::{Client, Response};
use reqwest::header::{USER_AGENT, REFERER, CONTENT_TYPE};
use url::form_urlencoded;

use std::fs;
use regex::Regex;
use sha1::{Sha1, Digest};

// Constants for user agent template and generation
const USER_AGENT_TEMPLATE: &str = "SQEXAuthor/2.0.0(Windows 6.2; ja-jp; {})";
lazy_static! {
    static ref USER_AGENT: String = generate_user_agent();
}

fn launch_game(game_path: &str, realsid: &str, language: i32, dx11: bool, expansionlevel: i32, is_steam: bool, region: i32) -> Option<Child> {
    let mut game_command = if dx11 {
        Command::new(format!("{}/game/ffxiv_dx11.exe", game_path))
    } else {
        Command::new(format!("{}/game/ffxiv.exe", game_path))
    };

    game_command.args(&[
        &format!("DEV.TestSID={}", realsid),
        &format!("DEV.MaxEntitledExpansionID={}", expansionlevel),
        &format!("language={}", language),
        &format!("region={}", region),
    ]);

    if is_steam {
        game_command.env("IS_FFXIV_LAUNCH_FROM_STEAM", "1");
        game_command.arg("IsSteam=1");
    }

    match game_command.spawn() {
        Ok(child) => Some(child),
        Err(e) => {
            match language {
                0 => eprintln!("実行可能ファイルを起動できませんでした。 ゲームパスは正しいですか? {}", e),
                1 => eprintln!("Could not launch executable. Is your game path correct? {}", e),
                2 => eprintln!("Die ausführbare Datei konnte nicht gestartet werden. Ist dein Spielpfad korrekt? {}", e),
                3 => eprintln!("Impossible de lancer l'exécutable. Votre chemin de jeu est-il correct? {}", e),
                4 => eprintln!("Не удалось запустить файл. Ввели ли вы корректный путь к игре? {}", e),
                _ => eprintln!("Unknown language."),
            }
            None
        }
    }
}


fn generate_user_agent() -> String {
    let computer_id = make_computer_id();
    format!(USER_AGENT_TEMPLATE, computer_id)
}

fn make_computer_id() -> String {
    let machine_name = get_machine_name();
    let user_name = env::var("USERNAME").unwrap_or_default();
    let os_version = "Windows 10.0"; // Example, replace with actual OS version retrieval
    let processor_count = num_cpus::get();

    let hash_string = format!("{}{}{}{}", machine_name, user_name, os_version, processor_count);
    let mut hasher = Sha1::new();
    hasher.update(hash_string.as_bytes());
    let hash_bytes = hasher.finalize();

    let mut bytes = [0u8; 5];
    bytes[1..].copy_from_slice(&hash_bytes[0..4]);

    let check_sum = !(bytes[1].wrapping_add(bytes[2]).wrapping_add(bytes[3]).wrapping_add(bytes[4]));
    bytes[0] = check_sum;

    bytes.iter().map(|byte| format!("{:02x}", byte)).collect::<Vec<_>>().join("")
}

fn get_machine_name() -> String {
    // Placeholder for machine name retrieval logic
    // This could be a hostname or an IP address, depending on your needs
    // For example, using the local IP address as a machine name:
    match local_ipaddress::get() {
        Some(ip) => ip,
        None => "unknown".to_string(),
    }
}
fn get_real_sid(game_path: &str, username: &str, password: &str, otp: Option<&str>, is_steam: bool) -> String {
    let mut hash_str = String::new();

    // Hashing files to prove game version
    let files_to_hash = [
        "ffxivboot.exe",
        "ffxivboot64.exe",
        "ffxivlauncher.exe",
        "ffxivlauncher64.exe",
        "ffxivupdater.exe",
        "ffxivupdater64.exe",
    ];

    for file in files_to_hash.iter() {
        match generate_hash(format!("{}/boot/{}", game_path, file)) {
            Ok(hash) => hash_str.push_str(&format!("{}/{},", file, hash)),
            Err(e) => eprintln!("Could not generate hashes. Is your game path correct? {}", e),
        }
    }

    // Truncate the last comma
    hash_str.pop();

    // Rest of the network logic would go here...

    hash_str
}

fn generate_hash(file_path: String) -> io::Result<String> {
    let data = fs::read(file_path)?;
    let mut hasher = Sha1::new();
    hasher.update(data);
    Ok(format!("{:x}", hasher.finalize()))
}
fn get_stored(is_steam: bool) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = format!("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", if is_steam { 1 } else { 0 });
    let response = client.get(&url)
        .header(USER_AGENT, USER_AGENT.as_str())
        .send()?;

    let body = response.text()?;
    let re = Regex::new(r"\t<\s*input .* name=\"_STORED_\" value=\"(?P<stored>.*)\">")?;
    let stored = re.captures(&body)
        .and_then(|cap| cap.name("stored").map(|stored| stored.as_str().to_string()))
        .ok_or("No _STORED_ value found")?;

    Ok(stored)
}

fn get_sid(username: &str, password: &str, otp: Option<&str>, is_steam: bool) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::new();
    let stored = get_stored(is_steam)?;

    let params = form_urlencoded::Serializer::new(String::new())
        .append_pair("_STORED_", &stored)
        .append_pair("sqexid", username)
        .append_pair("password", password)
        .append_pair("otppw", otp.unwrap_or(""))
        .finish();

    let response = client.post("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/login.send")
        .header(USER_AGENT, USER_AGENT.as_str())
        .header(REFERER, &format!("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", if is_steam { 1 } else { 0 }))
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(params)
        .send()?;

    let body = response.text()?;
    let re = Regex::new(r"sid,(?P<sid>.*),terms")?;
    let sid = re.captures(&body)
        .and_then(|cap| cap.name("sid").map(|sid| sid.as_str().to_string()))
        .ok_or("No SID value found or incorrect username/password")?;

    Ok(sid)
}

fn get_local_gamever(game_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = File::open(format!("{}/game/ffxivgame.ver", game_path))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents.trim().to_string())
}

// Placeholder for SSL trust initiation
fn initiate_ssl_trust() {
    // SSL trust initiation logic...
}

// Remember to add the sha1 and lazy_static dependencies to your Cargo.toml
