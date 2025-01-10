use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, REFERER, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{Error as IoError, Read};
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::ptr::{self, null_mut};
use std::time::Duration;
use std::time::Instant;

use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

#[cfg(windows)]
use std::os::windows::io::{FromRawHandle, RawHandle};
#[cfg(windows)]
use std::os::windows::prelude::*;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use winapi::ctypes::c_void;
#[cfg(windows)]
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
#[cfg(windows)]
use winapi::um::handleapi::CloseHandle;
#[cfg(windows)]
use winapi::um::minwinbase::SECURITY_ATTRIBUTES;
#[cfg(windows)]
use winapi::um::processthreadsapi::{
    CreateProcessW, GetProcessId, ResumeThread, PROCESS_INFORMATION, STARTUPINFOW,
};
#[cfg(windows)]
use winapi::um::securitybaseapi::{InitializeSecurityDescriptor, SetSecurityDescriptorDacl};
#[cfg(windows)]
use winapi::um::winbase::CREATE_SUSPENDED;
#[cfg(windows)]
use winapi::um::winnt::{
    HANDLE, HANDLE as WINAPI_HANDLE, PROCESS_ALL_ACCESS, SECURITY_DESCRIPTOR,
    SECURITY_DESCRIPTOR_REVISION,
};

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

fn default_dx11() -> bool {
    true
}
fn default_language() -> u32 {
    1
}
fn default_region() -> u32 {
    3
}
fn default_expansion_level() -> u32 {
    4
}
fn default_dpi_awareness() -> String {
    "Aware".to_string()
}
fn default_injection_delay() -> u64 {
    5000
}


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
        let args_wide: Vec<u16> = OsString::from(args).encode_wide().chain(once(0)).collect();

        let mut startup_info: STARTUPINFOW = std::mem::zeroed();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

        let mut process_info: PROCESS_INFORMATION = std::mem::zeroed();
        let mut security_attributes: SECURITY_ATTRIBUTES = std::mem::zeroed();
        security_attributes.nLength = std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32;
        security_attributes.bInheritHandle = TRUE;

        let mut security_descriptor: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let security_descriptor_ptr = &mut security_descriptor as *mut _ as *mut c_void;

        if InitializeSecurityDescriptor(security_descriptor_ptr, SECURITY_DESCRIPTOR_REVISION) == 0
        {
            return Err(format!(
                "Failed to initialize security descriptor: {}",
                IoError::last_os_error()
            ));
        }

        // Set a NULL DACL to allow access to everyone
        if SetSecurityDescriptorDacl(security_descriptor_ptr, TRUE, null_mut(), FALSE) == 0 {
            return Err(format!(
                "Failed to set security descriptor DACL: {}",
                IoError::last_os_error()
            ));
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
            &mut process_info,
        );

        if result == 0 {
            return Err(format!(
                "Failed to create process: {}",
                IoError::last_os_error()
            ));
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
    let total_start_time = Instant::now();
    let mut metrics = Vec::new();
    info!("Starting game launch process with config: {:?}", config);

    // Set up Dalamud first if enabled
    if config.enable_dalamud {
        info!("Dalamud is enabled, starting Dalamud setup");
        let dalamud_start = Instant::now();
        match setup_dalamud(&config).await {
            Ok(_) => {
                let dalamud_duration = dalamud_start.elapsed();
                metrics.push(format!("Dalamud setup: {:.2?}", dalamud_duration));

                info!(
                    "Dalamud setup completed successfully in {:.2?}",
                    dalamud_duration
                );

            }
            Err(e) => {
                error!("Dalamud setup failed: {}", e);
                return Err(format!("Dalamud setup failed: {}", e));
            }
        }
    }

    // Prepare game path
    let path_start = Instant::now();
    let game_path = if config.dx11 {
        format!("{}/game/ffxiv_dx11.exe", config.game_path)
    } else {
        format!("{}/game/ffxiv.exe", config.game_path)
    };
    info!("Using game executable: {}", game_path);

    // Verify executable exists
    if !Path::new(&game_path).exists() {
        error!("Game executable not found at {}", game_path);
        return Err(format!("Game executable not found at {}", game_path));
    }
    metrics.push(format!("Path preparation: {:.2?}", path_start.elapsed()));
    info!("Game executable found");

    // Get a fresh session ID right before launching
    info!("Getting fresh session ID");
    let sid_start = Instant::now();
    let sid = match get_session_id(&config).await {
        Ok(s) => {
            let sid_duration = sid_start.elapsed();
            metrics.push(format!("Session ID retrieval: {:.2?}", sid_duration));

            info!(
                "Successfully obtained fresh session ID in {:.2?}",
                sid_duration
            );

            s
        }
        Err(e) => {
            error!("Failed to get session ID: {}", e);
            return Err(format!("Failed to get session ID: {}", e));
        }
    };

    // Prepare launch arguments with fresh session ID
    let args_start = Instant::now();
    let args = format!(
        "DEV.DataPathType=1 DEV.MaxEntitledExpansionID={} DEV.TestSID={} DEV.UseSqPack=1 SYS.Region={} language={}",
        config.expansion_level,
        sid,
        config.region,
        config.language
    );

    metrics.push(format!(
        "Arguments preparation: {:.2?}",
        args_start.elapsed()
    ));

    info!("Launch arguments prepared: {}", args);

    // Launch the game with or without Dalamud
    let launch_start = Instant::now();
    if config.enable_dalamud {
        info!("Starting game with Dalamud entrypoint injection");
        match inject_dalamud(&config, &sid).await {
            Ok(_) => {
                let launch_duration = launch_start.elapsed();

                metrics.push(format!(
                    "Dalamud injection and launch: {:.2?}",
                    launch_duration
                ));
                info!(
                    "Game launched with Dalamud successfully in {:.2?}",
                    launch_duration
                );

            }
            Err(e) => {
                error!("Failed to launch game with Dalamud: {}", e);
                return Err(format!("Failed to launch game with Dalamud: {}", e));
            }
        }
    } else {
        info!("Attempting to create game process without Dalamud");
        match create_suspended_game_process(&game_path, &args) {
            Ok(p) => {
                let launch_duration = launch_start.elapsed();
                metrics.push(format!("Game process creation: {:.2?}", launch_duration));

                info!(
                    "Game process created successfully with PID: {} in {:.2?}",
                    p, launch_duration
                );

            }
            Err(e) => {
                error!("Failed to create game process: {}", e);
                return Err(format!("Failed to launch game: {}", e));
            }
        }
    }

    let total_elapsed = total_start_time.elapsed();
    metrics.push(format!("Total launch time: {:.2?}", total_elapsed));

    // Join all metrics into a single string
    let metrics_str = metrics.join("\n");
    info!("Launch performance metrics:\n{}", metrics_str);

    Ok(format!(
        "Game launched successfully. Performance metrics:\n{}",
        metrics_str
    ))

}

async fn get_session_id(config: &LaunchConfig) -> Result<String, String> {
    let start_time = Instant::now();
    info!("Starting session ID retrieval");


    let client = Client::builder()
        .timeout(Duration::from_secs(200)) // Add a 200 second timeout - 30 seconds would fail before square gives session id as their server for login are famously slow

        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    info!("HTTP client created in {:?}", start_time.elapsed());

    let stored_start = Instant::now();
    info!("Getting stored value");
    let stored = match get_stored(config.is_steam).await {
        Ok(s) => {

            info!(
                "Successfully retrieved stored value in {:?}",
                stored_start.elapsed()
            );
            s
        }
        Err(e) => {
            error!(
                "Failed to get stored value after {:?}: {}",
                stored_start.elapsed(),
                e
            );

            return Err(e);
        }
    };

    let form_start = Instant::now();
    let mut form = HashMap::new();
    form.insert("_STORED_", stored);
    form.insert("sqexid", config.username.clone());
    form.insert("password", config.password.clone());
    form.insert("otppw", config.otp.clone().unwrap_or_default());
    info!("Form prepared in {:?}", form_start.elapsed());

    let login_start = Instant::now();
    info!("Sending login request to Square Enix");
    let response = match client.post("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/login.send")
        .header(USER_AGENT, get_user_agent())
        .header(REFERER, format!("https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", 
            if config.is_steam { "1" } else { "0" }))
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form)
        .send()
        .await {
            Ok(r) => {
                info!("Login request sent successfully in {:?}", login_start.elapsed());
                r
            }
            Err(e) => {
                error!("Failed to send login request after {:?}: {}", login_start.elapsed(), e);
                return Err(format!("Failed to send login request: {}", e));
            }
        };

    let body_start = Instant::now();
    info!("Reading response body");
    let body = match response.text().await {
        Ok(b) => {

            info!(
                "Successfully received response body in {:?}",
                body_start.elapsed()
            );
            b
        }
        Err(e) => {
            error!(
                "Failed to read response body after {:?}: {}",
                body_start.elapsed(),
                e
            );

            return Err(format!("Failed to read response: {}", e));
        }
    };

    let parse_start = Instant::now();
    info!("Parsing response for session ID");
    let re = regex::Regex::new(r"sid,(?P<sid>.*),terms").unwrap();
    let result = match re.captures(&body) {
        Some(caps) => {
            let sid = caps["sid"].to_string();

            info!(
                "Successfully extracted session ID in {:?}",
                parse_start.elapsed()
            );
            Ok(sid)
        }
        None => {
            error!(
                "Failed to extract session ID after {:?}. Response body: {}",
                parse_start.elapsed(),
                body
            );

            Err("Failed to extract session ID".to_string())
        }
    };

    info!("Total session ID retrieval took {:?}", start_time.elapsed());
    result
}

async fn get_stored(is_steam: bool) -> Result<String, String> {
    let start_time = Instant::now();
    info!("Starting stored value retrieval");


    let client = Client::builder()
        .timeout(Duration::from_secs(30)) // Add a 30 second timeout

        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!(
        "https://ffxiv-login.square-enix.com/oauth/ffxivarr/login/top?lng=en&rgn=3&isft=0&issteam={}", 
        if is_steam { "1" } else { "0" }
    );
    info!("Requesting stored value from: {}", url);

    let response = match client
        .get(&url)
        .header(USER_AGENT, get_user_agent())
        .send()

        .await
    {
        Ok(r) => {
            info!(
                "Received stored value response in {:?}",
                start_time.elapsed()
            );
            r
        }
        Err(e) => {
            error!(
                "Failed to get stored value after {:?}: {}",
                start_time.elapsed(),
                e
            );
            return Err(format!("Failed to get stored value: {}", e));
        }
    };

    let body = match response.text().await {
        Ok(b) => {
            info!("Received stored value body in {:?}", start_time.elapsed());
            b
        }
        Err(e) => {
            error!(
                "Failed to read stored value response after {:?}: {}",
                start_time.elapsed(),
                e
            );
            return Err(format!("Failed to read response: {}", e));
        }
    };


    let re = regex::Regex::new(r#"<input.*?name="_STORED_".*?value="([^"]*)"#).unwrap();
    match re.captures(&body) {
        Some(caps) => {
            let stored = caps.get(1).unwrap().as_str().to_string();

            info!(
                "Successfully extracted stored value in {:?}",
                start_time.elapsed()
            );
            Ok(stored)
        }
        None => {
            error!(
                "Could not find _STORED_ value in response after {:?}. Response body: {}",
                start_time.elapsed(),
                body
            );

            Err("Could not find _STORED_ value".to_string())
        }
    }
}

fn get_user_agent() -> String {
    format!(
        "SQEXAuthor/2.0.0(Windows 6.2; ja-jp; {})",
        make_computer_id()
    )
}

fn make_computer_id() -> String {
    let machine_name = env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string());
    let user_name = env::var("USERNAME").unwrap_or_default();
    let os_version = "Windows 10.0";
    let processor_count = num_cpus::get();

    let hash_string = format!(
        "{}{}{}{}",
        machine_name, user_name, os_version, processor_count
    );
    let mut hasher = Sha1::new();
    hasher.update(hash_string.as_bytes());
    let hash = hasher.finalize();

    let mut bytes = [0u8; 5];
    bytes[1..].copy_from_slice(&hash[0..4]);

    let checksum = !(bytes[1]
        .wrapping_add(bytes[2])
        .wrapping_add(bytes[3])
        .wrapping_add(bytes[4]));
    bytes[0] = checksum;

    hex::encode(bytes)
}

#[derive(Debug, Serialize, Deserialize)]
struct DalamudVersionInfo {
    key: String,
    track: String,
    #[serde(rename = "assemblyVersion")]
    assembly_version: String,
    #[serde(rename = "runtimeVersion")]
    runtime_version: String,
    #[serde(rename = "runtimeRequired")]
    runtime_required: bool,
    #[serde(rename = "supportedGameVer")]
    supported_game_ver: String,
    #[serde(rename = "isApplicableForCurrentGameVer")]
    is_applicable_for_current_game_ver: bool,
    changelog: DalamudChangelog,
    #[serde(rename = "downloadUrl")]
    download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DalamudChangelog {
    date: String,
    version: String,
    changes: Vec<DalamudChange>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DalamudChange {
    message: String,
    author: String,
    sha: String,
    date: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AssetInfo {
    version: i32,
    #[serde(rename = "packageUrl")]
    package_url: String,
    assets: Vec<AssetFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AssetFile {
    url: String,
    #[serde(rename = "fileName")]
    file_name: String,
    hash: Option<String>,
}

async fn check_dalamud_version(
    client: &Client,
    is_staging: bool,
) -> Result<DalamudVersionInfo, String> {
    let url = format!(
        "https://kamori.goats.dev/Dalamud/Release/VersionInfo?track={}",
        if is_staging { "staging" } else { "release" }
    );

    let response = client
        .get(&url)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to get version info: {}", e))?;

    response
        .json::<DalamudVersionInfo>()
        .await
        .map_err(|e| format!("Failed to parse version info: {}", e))
}

async fn check_asset_version(client: &Client) -> Result<AssetInfo, String> {
    let response = client
        .get("https://kamori.goats.dev/Dalamud/Asset/Meta")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to get asset info: {}", e))?;

    response
        .json::<AssetInfo>()
        .await
        .map_err(|e| format!("Failed to parse asset info: {}", e))
}

async fn setup_dalamud(config: &LaunchConfig) -> Result<String, String> {
    info!("Setting up Dalamud with base path: {}", config.dalamud_path);
    let start_time = Instant::now();

    // Normalize base path - ensure we don't have duplicate /addon

    let base_path =
        if config.dalamud_path.ends_with("/addon") || config.dalamud_path.ends_with("\\addon") {
            info!(
                "Base path already ends with addon, using as is: {}",
                config.dalamud_path
            );
            config.dalamud_path.clone()
        } else {
            let path = format!("{}/addon", config.dalamud_path);
            info!("Adding /addon to base path: {}", path);
            path
        };


    // Fast version check first
    let client = Client::new();
    let version_info = check_dalamud_version(&client, false).await?;
    info!("Remote Dalamud version: {}", version_info.assembly_version);

    // Check local version and integrity before any downloads
    let current_version_path = format!("{}/Hooks/{}", base_path, version_info.assembly_version);
    let needs_dalamud_update = if Path::new(&current_version_path).exists() {
        info!("Found existing Dalamud installation, checking integrity");
        !check_dalamud_integrity(&current_version_path)?
    } else {
        info!("No existing Dalamud installation found");
        true
    };

    // Fast asset version check
    let asset_info = check_asset_version(&client).await?;
    let asset_ver_path = format!("{}/dalamudAssets/asset.ver", config.dalamud_path);

    let current_asset_ver = fs::read_to_string(&asset_ver_path)
        .unwrap_or_else(|_| "0".to_string())
        .parse::<i32>()
        .unwrap_or(0);

    let needs_asset_update = current_asset_ver < asset_info.version;

    // Create required directories only if we need to update something
    if needs_dalamud_update || needs_asset_update {
        // Create base directories
        fs::create_dir_all(&base_path)
            .map_err(|e| format!("Failed to create Dalamud base directory: {}", e))?;


        // Required directories relative to XIVLOADER root (not addon)
        let root_directories = [
            "dalamudAssets",
            "dalamudAssets/UIRes",
            "installedPlugins",
            "pluginConfigs",
            "runtime",
            "logs",
        ];

        for dir in root_directories {
            let path = format!("{}/{}", config.dalamud_path, dir);
            if !Path::new(&path).exists() {
                fs::create_dir_all(&path)
                    .map_err(|e| format!("Failed to create directory {}: {}", dir, e))?;
                info!("Created directory: {}", path);

                #[cfg(windows)]
                {
                    let metadata = fs::metadata(&path)
                        .map_err(|e| format!("Failed to get metadata for {}: {}", path, e))?;
                    let mut perms = metadata.permissions();
                    perms.set_readonly(false);
                    fs::set_permissions(&path, perms)
                        .map_err(|e| format!("Failed to set permissions for {}: {}", path, e))?;
                }
            }
        }
    }

    // Update Dalamud if needed
    if needs_dalamud_update {

        info!(
            "Updating Dalamud to version {}",
            version_info.assembly_version
        );


        // Create Hooks directory
        let hooks_dir = format!("{}/Hooks", base_path);
        fs::create_dir_all(&hooks_dir)
            .map_err(|e| format!("Failed to create Hooks directory: {}", e))?;

        // Download and extract Dalamud
        let temp_path = format!("{}/dalamud_temp.zip", config.dalamud_path);
        download_file(&client, &version_info.download_url, &temp_path).await?;


        // Create version directory
        fs::create_dir_all(&current_version_path)
            .map_err(|e| format!("Failed to create version directory: {}", e))?;

        // Extract to version directory
        extract_zip(&temp_path, &current_version_path)?;
        fs::remove_file(&temp_path).map_err(|e| format!("Failed to remove temp file: {}", e))?;

        // Write version info
        fs::write(
            format!("{}/version.json", current_version_path),
            serde_json::to_string(&version_info)
                .map_err(|e| format!("Failed to serialize version info: {}", e))?,
        )
        .map_err(|e| format!("Failed to write version info: {}", e))?;


        info!("Dalamud update completed");
    } else {
        info!("Dalamud is up to date");
    }

    // Update assets if needed
    if needs_asset_update {

        info!(
            "Updating assets from version {} to {}",
            current_asset_ver, asset_info.version
        );

        // Download and extract the package
        let temp_path = format!("{}/asset_package_temp.zip", config.dalamud_path);
        download_file(&client, &asset_info.package_url, &temp_path).await?;


        // Extract package to assets directory
        let assets_dir = format!("{}/dalamudAssets", config.dalamud_path);
        extract_zip(&temp_path, &assets_dir)?;
        fs::remove_file(&temp_path).map_err(|e| format!("Failed to remove temp file: {}", e))?;

        // Verify all required files exist and check hashes
        for asset in &asset_info.assets {
            let target_path = format!("{}/dalamudAssets/{}", config.dalamud_path, asset.file_name);
            if !Path::new(&target_path).exists() {

                error!(
                    "Required asset file not found after extraction: {}",
                    asset.file_name
                );

                return Err(format!("Missing required asset file: {}", asset.file_name));
            }

            if let Some(expected_hash) = &asset.hash {
                let contents = fs::read(&target_path)
                    .map_err(|e| format!("Failed to read file {}: {}", asset.file_name, e))?;


                let mut hasher = Sha1::new();
                hasher.update(&contents);
                let file_hash = hex::encode(hasher.finalize()).to_uppercase();

                if file_hash != *expected_hash {
                    error!(
                        "Hash mismatch for {}: expected {}, got {}",
                        asset.file_name, expected_hash, file_hash
                    );

                    return Err(format!("Hash verification failed for {}", asset.file_name));
                }
            }
        }

        // Update version file
        fs::write(&asset_ver_path, asset_info.version.to_string())
            .map_err(|e| format!("Failed to write asset version: {}", e))?;

        info!("Asset update completed");
    } else {
        info!("Assets are up to date");
    }

    // Verify critical files exist
    let injector_path = format!("{}/Dalamud.Injector.exe", current_version_path);
    if !Path::new(&injector_path).exists() {
        error!("Dalamud injector not found at: {}", injector_path);

        return Err(format!(
            "Dalamud injector not found at {}. Please ensure Dalamud is properly installed.",
            injector_path
        ));
    }

    let fasm_dll = format!(
        "{}/FASM{}.DLL",
        current_version_path,
        if cfg!(target_arch = "x86_64") {
            "X64"
        } else {
            ""
        }
    );
    if !Path::new(&fasm_dll).exists() {
        error!("FASM DLL not found at: {}", fasm_dll);
        return Err(format!(
            "FASM DLL not found at {}. Please ensure Dalamud is properly installed.",
            fasm_dll
        ));

    }

    // Handle font files
    let uires_path = format!("{}/dalamudAssets/UIRes", config.dalamud_path);
    let font_files = [
        ("FontAwesomeFreeSolid.otf", "FontAwesomeFreeSolid"),
        ("NotoSansCJKjp-Medium.otf", "NotoSansJpMedium"),
    ];

    for (file_name, required_name) in font_files {
        let font_path = format!("{}/{}", uires_path, file_name);
        let required_path = format!("{}/{}", uires_path, required_name);

        if Path::new(&font_path).exists() && !Path::new(&required_path).exists() {
            info!("Creating font link from {} to {}", font_path, required_path);
            #[cfg(windows)]
            {
                match fs::copy(&font_path, &required_path) {
                    Ok(_) => info!("Copied {} to {}", file_name, required_name),
                    Err(e) => error!("Failed to copy font file: {}", e),
                }
            }
        }
    }

    let elapsed = start_time.elapsed();
    info!("Dalamud setup completed in {:.2?}", elapsed);
    Ok(format!("Dalamud setup completed in {:.2?}", elapsed))
}

async fn download_file(client: &Client, url: &str, path: &str) -> Result<(), String> {
    info!("Starting download from: {}", url);

    let mut current_url = url.to_string();
    let mut retries = 0;
    const MAX_RETRIES: u32 = 15;

    while retries < MAX_RETRIES {
        info!("Attempting download from: {}", current_url);

        let response = client
            .get(&current_url)
            .timeout(Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| format!("Failed to download file: {}", e))?;

        // Check if we got redirected
        if response.status().is_redirection() {
            if let Some(new_url) = response.headers().get("location") {

                current_url = new_url
                    .to_str()

                    .map_err(|e| format!("Invalid redirect URL: {}", e))?
                    .to_string();
                info!("Following redirect to: {}", current_url);
                retries += 1;
                continue;
            }
        }

        // If we got a successful response, download the file
        if response.status().is_success() {
            info!("Download started, writing to: {}", path);
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to get response bytes: {}", e))?;


            fs::write(path, bytes).map_err(|e| format!("Failed to write file: {}", e))?;


            info!("Download completed successfully");
            return Ok(());
        }

        // If we got here, the response wasn't a redirect or success

        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));

    }

    Err(format!("Too many redirects while downloading from {}", url))
}

fn extract_zip(zip_path: &str, extract_path: &str) -> Result<(), String> {
    use std::fs::File;
    use zip::ZipArchive;

    let file = File::open(zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;

    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    archive
        .extract(extract_path)
        .map_err(|e| format!("Failed to extract zip: {}", e))?;

    Ok(())
}

fn check_dalamud_integrity(path: &str) -> Result<bool, String> {
    let hashes_path = format!("{}/hashes.json", path);
    if !Path::new(&hashes_path).exists() {
        return Ok(false);
    }

    let hashes: HashMap<String, String> = serde_json::from_str(
        &fs::read_to_string(&hashes_path)
            .map_err(|e| format!("Failed to read hashes.json: {}", e))?,
    )
    .map_err(|e| format!("Failed to parse hashes.json: {}", e))?;

    for (file, hash) in hashes {
        let file_path = format!("{}/{}", path, file);
        if !Path::new(&file_path).exists() {
            return Ok(false);
        }

        let contents =
            fs::read(&file_path).map_err(|e| format!("Failed to read file {}: {}", file, e))?;

        let mut hasher = Sha1::new();
        hasher.update(&contents);
        let file_hash = hex::encode(hasher.finalize());

        if file_hash != hash {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(windows)]
async fn inject_dalamud(config: &LaunchConfig, sid: &str) -> Result<String, String> {
    // Get Dalamud version info first to construct correct paths
    let client = Client::new();
    let version_info = check_dalamud_version(&client, false).await?;
    info!("Using Dalamud version: {}", version_info.assembly_version);

    // Normalize base path for injection

    let base_path =
        if config.dalamud_path.ends_with("/addon") || config.dalamud_path.ends_with("\\addon") {
            config.dalamud_path.clone()
        } else {
            format!("{}/addon", config.dalamud_path)
        };

    info!("Using Dalamud base path for injection: {}", base_path);

    // Construct version-specific paths
    let version_path = format!("{}/Hooks/{}", base_path, version_info.assembly_version);
    let injector_path = format!("{}/Dalamud.Injector.exe", version_path);
    info!("Using version-specific injector at: {}", injector_path);

    // Wait for the configured injection delay
    if config.injection_delay > 0 {

        info!(
            "Waiting {}ms before injecting Dalamud",
            config.injection_delay
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(config.injection_delay)).await;
    }

    let start_info = DalamudStartInfo {
        working_directory: version_path.clone(), // Use version-specific path
        configuration_path: format!("{}/config", config.dalamud_path),
        plugin_directory: format!("{}/installedPlugins", config.dalamud_path),
        asset_directory: format!("{}/dalamudAssets", config.dalamud_path),
        client_language: config.language,
        delay_initialize: false,
        game_version: get_game_version(&config.game_path)?,
        logging_path: format!("{}/logs", config.dalamud_path),
        troubleshooting_pack: Some("{}".to_string()),
        delay_initialize_ms: config.injection_delay as i32,
    };

    let start_info_json = serde_json::to_string(&start_info)
        .map_err(|e| format!("Failed to serialize start info: {}", e))?;

    let start_info_b64 = base64::encode(start_info_json.as_bytes());
    info!("Dalamud start info (base64): {}", start_info_b64);

    if !Path::new(&injector_path).exists() {
        error!("Dalamud injector not found at: {}", injector_path);
        return Err(format!(
            "Dalamud injector not found at {}. Please ensure Dalamud is properly installed.",
            injector_path
        ));
    }
    info!("Verified injector exists at: {}", injector_path);

    let game_path = if config.dx11 {
        format!("{}/game/ffxiv_dx11.exe", config.game_path)
    } else {
        format!("{}/game/ffxiv.exe", config.game_path)
    };

    // Prepare all argument strings
    let game_arg = format!("--game={}", game_path);
    let working_dir_arg = format!("--dalamud-working-directory={}", version_path); // Use version-specific path

    let config_path_arg = format!(
        "--dalamud-configuration-path={}/config",
        config.dalamud_path
    );
    let plugin_dir_arg = format!(
        "--dalamud-plugin-directory={}/installedPlugins",
        config.dalamud_path
    );
    let asset_dir_arg = format!(
        "--dalamud-asset-directory={}/dalamudAssets",
        config.dalamud_path
    );

    let log_path_arg = format!("--logpath={}/logs", config.dalamud_path);
    let lang_arg = format!("--dalamud-client-language={}", config.language);
    let delay_arg = format!("--dalamud-delay-initialize={}", config.injection_delay);
    let tspack_arg = format!("--dalamud-tspack-b64={}", start_info_b64);

    // Prepare game arguments
    let game_args = format!(
        "DEV.DataPathType=1 DEV.MaxEntitledExpansionID={} DEV.TestSID={} DEV.UseSqPack=1 SYS.Region={} language={}",
        config.expansion_level,
        sid,
        config.region,
        config.language
    );

    // Build arguments for entrypoint injection
    let args = vec![
        "launch",
        "--mode=entrypoint",
        &game_arg,
        &working_dir_arg,
        &config_path_arg,
        &plugin_dir_arg,
        &asset_dir_arg,
        &log_path_arg,
        &lang_arg,
        &delay_arg,
        &tspack_arg,
        "--", // Separator for game arguments
        &game_args,
    ];

    // Set up the command with proper working directory and environment
    let mut command = Command::new(&injector_path);
    command
        .current_dir(&version_path) // Use version-specific path
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Add DALAMUD_RUNTIME environment variable if needed
    let runtime_path = format!("{}/runtime", config.dalamud_path);
    if Path::new(&runtime_path).exists() {
        info!("Setting DALAMUD_RUNTIME to: {}", runtime_path);
        command.env("DALAMUD_RUNTIME", &runtime_path);
        command.env("__COMPAT_LAYER", "RunAsInvoker HighDPIAware");
    }

    info!("Running Dalamud injector with command: {:?}", command);

    let output = command
        .output()
        .map_err(|e| format!("Failed to run injector: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        error!("Injector failed with error: {}", error);
        error!("Injector stdout: {}", stdout);
        return Err(format!("Injector failed: {}", error));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("Dalamud injector stdout: {}", stdout);

    info!("Dalamud injection completed successfully");
    Ok("Dalamud injection completed successfully".to_string())
}

fn get_game_version(game_path: &str) -> Result<String, String> {
    let ver_path = format!("{}/game/ffxivgame.ver", game_path);
    fs::read_to_string(&ver_path).map_err(|e| format!("Failed to read game version: {}", e))
}

#[derive(Debug, Serialize, Deserialize)]
struct DalamudStartInfo {
    working_directory: String,
    configuration_path: String,
    plugin_directory: String,
    asset_directory: String,
    client_language: u32,
    delay_initialize: bool,
    game_version: String,
    logging_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    troubleshooting_pack: Option<String>,
    delay_initialize_ms: i32,
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
            game_version: String::new(),
            logging_path: String::new(),
            troubleshooting_pack: None,
            delay_initialize_ms: 0,
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Headlines {
    pub news: Vec<News>,
    pub topics: Vec<News>,
    pub pinned: Vec<News>,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(i32),
    }

    match Option::<StringOrNumber>::deserialize(deserializer)? {
        Some(StringOrNumber::String(s)) => {
            if s.is_empty() || s == "0" {
                Ok(Some(0))
            } else {
                s.parse().map(Some).map_err(D::Error::custom)
            }
        }
        Some(StringOrNumber::Number(n)) => Ok(Some(n)),
        None => Ok(None),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Banner {
    #[serde(rename = "lsb_banner")]
    pub lsb_banner: String,
    pub link: String,
    #[serde(
        rename = "order_priority",
        deserialize_with = "deserialize_string_or_number"
    )]
    pub order_priority: Option<i32>,
    #[serde(
        rename = "fix_order",
        deserialize_with = "deserialize_string_or_number"
    )]
    pub fix_order: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct News {
    pub date: String,
    pub title: String,
    pub url: String,
    pub id: String,
    pub tag: Option<String>,
}

#[tauri::command]
pub async fn get_news(language: u32, force_na: bool) -> Result<Headlines, String> {
    let unix_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let lang_code = match language {
        1 => "en-us",
        2 => "de-de",
        3 => "fr-fr",
        _ => "en-us",
    };

    let url = format!(
        "https://frontier.ffxiv.com/news/headline.json?lang={}&media=pcapp&_={}",
        lang_code, unix_timestamp
    );

    let client = Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", get_user_agent())
        .send()
        .await
        .map_err(|e| format!("Failed to get news: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to get response text: {}", e))?;

    println!("{:?}", text); // Log the response text in plain text

    serde_json::from_str(&text).map_err(|e| format!("Failed to parse news JSON: {}", e))
}

#[tauri::command]
pub async fn get_banners(language: u32, force_na: bool) -> Result<Vec<Banner>, String> {
    let unix_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let lang_code = match language {
        1 => "en-us",
        2 => "de-de",
        3 => "fr-fr",
        _ => "en-us",
    };

    let url = format!(
        "https://frontier.ffxiv.com/v2/topics/{}/banner.json?lang={}&media=pcapp&_={}",
        lang_code, lang_code, unix_timestamp
    );

    let client = Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", get_user_agent())
        .send()
        .await
        .map_err(|e| format!("Failed to get banners: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to get response text: {}", e))?;

    println!("{:?}", text); // Log the response text in plain text

    #[derive(Deserialize)]
    struct BannerRoot {
        banner: Vec<Banner>,
    }

    let root: BannerRoot =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse banner JSON: {}", e))?;

    Ok(root.banner)
}

