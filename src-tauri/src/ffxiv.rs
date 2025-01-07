use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use std::fs;
use std::process::{Command, Child, Stdio};
use std::path::Path;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, REFERER, CONTENT_TYPE};
use std::io::{Read, Error as IoError};
use tracing::{info, warn, error, debug};
use std::time::Instant;
use reqwest::Client;
use std::collections::HashMap;
use std::env;
use std::ptr::{self, null_mut};
use std::iter::once;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use std::os::windows::io::{FromRawHandle, RawHandle};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use winapi::um::processthreadsapi::{STARTUPINFOW, PROCESS_INFORMATION, CreateProcessW, GetProcessId, ResumeThread};
#[cfg(windows)]
use winapi::um::minwinbase::SECURITY_ATTRIBUTES;
#[cfg(windows)]
use winapi::um::securitybaseapi::{InitializeSecurityDescriptor, SetSecurityDescriptorDacl};
#[cfg(windows)]
use winapi::um::handleapi::CloseHandle;
#[cfg(windows)]
use winapi::um::winnt::{SECURITY_DESCRIPTOR, PROCESS_ALL_ACCESS, HANDLE, SECURITY_DESCRIPTOR_REVISION, HANDLE as WINAPI_HANDLE};
#[cfg(windows)]
use winapi::shared::minwindef::{BOOL, FALSE, TRUE, DWORD};
#[cfg(windows)]
use winapi::um::winbase::CREATE_SUSPENDED;
#[cfg(windows)]
use winapi::ctypes::c_void;

#[derive(Debug)]
pub struct GameLaunchMetrics {
    login_time_ms: u64,
    sid_fetch_time_ms: u64,
    game_start_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchConfig {
    pub game_path: String,
    pub username: String,
    pub password: String,
    pub otp: Option<String>,
    #[serde(default = "default_dx11")]
    pub dx11: bool,
    #[serde(default = "default_language")]
    pub language: u32,
    #[serde(default = "default_region")]
    pub region: u32,
    #[serde(default = "default_expansion_level")]
    pub expansion_level: u32,
    #[serde(default)]
    pub is_steam: bool,
    #[serde(default = "default_dpi_awareness")]
    pub dpi_awareness: String,
    #[serde(default)]
    pub additional_launch_args: String,
    #[serde(default)]
    pub enable_dalamud: bool,
    #[serde(default)]
    pub dalamud_path: String,
    #[serde(default = "default_injection_delay")]
    pub injection_delay: u64,
}

fn default_dx11() -> bool { true }
fn default_language() -> u32 { 1 }
fn default_region() -> u32 { 3 }
fn default_expansion_level() -> u32 { 4 }
fn default_dpi_awareness() -> String { "Aware".to_string() }
fn default_injection_delay() -> u64 { 5000 }

#[derive(Debug)]
struct ProcessHandles {
    pid: u32,
    process_handle: WINAPI_HANDLE,
    thread_handle: WINAPI_HANDLE,
}

impl Drop for ProcessHandles {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.thread_handle);
            CloseHandle(self.process_handle);
        }
    }
}

#[cfg(windows)]
fn create_suspended_game_process(game_path: &str, args: &str) -> Result<u32, String> {
    unsafe {
        let game_path_wide: Vec<u16> = OsString::from(game_path)
            .encode_wide()
            .chain(once(0))
            .collect();
        let args_wide: Vec<u16> = OsString::from(args)
            .encode_wide()
            .chain(once(0))
            .collect();

        let mut startup_info: STARTUPINFOW = std::mem::zeroed();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

        let mut process_info: PROCESS_INFORMATION = std::mem::zeroed();
        let mut security_attributes: SECURITY_ATTRIBUTES = std::mem::zeroed();
        security_attributes.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        security_attributes.bInheritHandle = TRUE;

        let mut security_descriptor: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let security_descriptor_ptr = &mut security_descriptor as *mut _ as *mut c_void;
        
        if InitializeSecurityDescriptor(security_descriptor_ptr, SECURITY_DESCRIPTOR_REVISION) == 0 {
            return Err(format!("Failed to initialize security descriptor: {}", IoError::last_os_error()));
        }

        // Set a NULL DACL to allow access to everyone
        if SetSecurityDescriptorDacl(
            security_descriptor_ptr,
            TRUE,
            null_mut(),
            FALSE
        ) == 0 {
            return Err(format!("Failed to set security descriptor DACL: {}", IoError::last_os_error()));
        }

        security_attributes.lpSecurityDescriptor = security_descriptor_ptr;

        let result = CreateProcessW(
            game_path_wide.as_ptr(),
            args_wide.as_ptr() as *mut _,
            &mut security_attributes,
            &mut security_attributes,
            TRUE,
            CREATE_SUSPENDED,
            null_mut(),
            null_mut(),
            &mut startup_info,
            &mut process_info
        );

        if result == 0 {
            return Err(format!("Failed to create process: {}", IoError::last_os_error()));
        }

        // Get the PID before we clean up handles
        let pid = GetProcessId(process_info.hProcess);

        // Resume the thread
        if ResumeThread(process_info.hThread) == u32::MAX {
            let err = format!("Failed to resume process: {}", IoError::last_os_error());
            CloseHandle(process_info.hThread);
            CloseHandle(process_info.hProcess);
            return Err(err);
        }

        // Clean up handles
        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);

        Ok(pid)
    }
}

#[tauri::command]
pub async fn launch_game(config: LaunchConfig) -> Result<String, String> {
    let start_time = Instant::now();
    info!("Starting game launch process");

    // Set up Dalamud first if enabled
    if config.enable_dalamud {
        info!("Setting up Dalamud");
        setup_dalamud(&config).await?;
    }

    // Get session ID
    let sid = get_session_id(&config).await?;
    info!("Got session ID: {}", sid);

    // Prepare game path
    let game_path = if config.dx11 {
        format!("{}/game/ffxiv_dx11.exe", config.game_path)
    } else {
        format!("{}/game/ffxiv.exe", config.game_path)
    };

    // Verify executable exists
    if !Path::new(&game_path).exists() {
        error!("Game executable not found at {}", game_path);
        return Err(format!("Game executable not found at {}", game_path));
    }

    // Prepare launch arguments
    let args = format!(
        "DEV.DataPathType=1 DEV.MaxEntitledExpansionID={} DEV.TestSID={} DEV.UseSqPack=1 SYS.Region={} language={}",
        config.expansion_level,
        sid,
        config.region,
        config.language
    );

    // Launch game process (handles are cleaned up inside this function)
    info!("Launching game from {}", game_path);
    let pid = create_suspended_game_process(&game_path, &args)
        .map_err(|e| format!("Failed to launch game: {}", e))?;

    // Inject Dalamud if enabled
    if config.enable_dalamud {
        info!("Injecting Dalamud into process {}", pid);
        inject_dalamud(&config, pid).await?;
    }

    let elapsed = start_time.elapsed();
    info!("Game launch completed in {:.2?}", elapsed);

    Ok(format!("Game launched successfully in {:.2?}", elapsed))
}

async fn get_session_id(config: &LaunchConfig) -> Result<String, String> {
    let client = Client::new();
    let stored = get_stored(config.is_steam).await?;

    let mut form = HashMap::new();
    form.insert("_STORED_", stored);
    form.insert("sqexid", config.username.clone());
    form.insert("password", config.password.clone());
    form.insert("otppw", config.otp.clone().unwrap_or_default());

    let response = client.post("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/login.send")
        .header(USER_AGENT, get_user_agent())
        .header(REFERER, format!("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", 
            if config.is_steam { "1" } else { "0" }))
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form)
        .send()
        .await
        .map_err(|e| format!("Failed to send login request: {}", e))?;

    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    
    let re = regex::Regex::new(r"sid,(?P<sid>.*),terms").unwrap();
    match re.captures(&body) {
        Some(caps) => Ok(caps["sid"].to_string()),
        None => Err("Failed to extract session ID".to_string())
    }
}

async fn get_stored(is_steam: bool) -> Result<String, String> {
    let client = Client::new();
    let url = format!(
        "https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", 
        if is_steam { "1" } else { "0" }
    );

    let response = client.get(&url)
        .header(USER_AGENT, get_user_agent())
        .send()
        .await
        .map_err(|e| format!("Failed to get stored value: {}", e))?;

    let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    
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

async fn setup_dalamud(config: &LaunchConfig) -> Result<String, String> {
    info!("Setting up Dalamud");
    let start_time = Instant::now();

    let base_path = &config.dalamud_path;
    fs::create_dir_all(base_path)
        .map_err(|e| format!("Failed to create Dalamud directory: {}", e))?;

    let directories = [
        "addon",
        "runtime",
        "dalamudAssets",
        "installedPlugins",
        "pluginConfigs",
        "devPlugins",
    ];

    for dir in directories {
        let path = format!("{}/{}", base_path, dir);
        fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create directory {}: {}", dir, e))?;
        info!("Created directory: {}", path);
    }

    let elapsed = start_time.elapsed();
    info!("Dalamud setup completed in {:.2?}", elapsed);
    Ok(format!("Dalamud setup completed in {:.2?}", elapsed))
}

#[cfg(windows)]
async fn inject_dalamud(config: &LaunchConfig, pid: u32) -> Result<String, String> {
    let start_info = DalamudStartInfo {
        working_directory: config.dalamud_path.clone(),
        configuration_path: format!("{}/config", config.dalamud_path),
        plugin_directory: format!("{}/installedPlugins", config.dalamud_path),
        asset_directory: format!("{}/dalamudAssets", config.dalamud_path),
        client_language: config.language,
        delay_initialize: false,
        troubleshooting_pack: None,
    };

    let start_info_json = serde_json::to_string(&start_info)
        .map_err(|e| format!("Failed to serialize start info: {}", e))?;

    info!("Dalamud start info: {}", start_info_json);

    let injector_path = format!("{}/addon/Dalamud.Injector.exe", config.dalamud_path);
    let output = Command::new(injector_path)
        .args(&[
            "--pid",
            &pid.to_string(),
            "--dalamud-start-info",
            &start_info_json,
        ])
        .output()
        .map_err(|e| format!("Failed to run injector: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Injector failed: {}", error));
    }

    info!("Dalamud injection completed successfully");
    Ok("Dalamud injection completed successfully".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct DalamudStartInfo {
    working_directory: String,
    configuration_path: String,
    plugin_directory: String,
    asset_directory: String,
    client_language: u32,
    delay_initialize: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    troubleshooting_pack: Option<String>,
}

impl Default for DalamudStartInfo {
    fn default() -> Self {
        Self {
            working_directory: String::new(),
            configuration_path: String::new(),
            plugin_directory: String::new(),
            asset_directory: String::new(),
            client_language: 1,
            delay_initialize: false,
            troubleshooting_pack: None,
        }
    }
}
