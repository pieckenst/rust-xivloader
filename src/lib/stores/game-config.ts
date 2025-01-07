import { writable } from 'svelte/store';
import { join } from '@tauri-apps/api/path';
import { appDataDir, homeDir } from '@tauri-apps/api/path';

export interface GameConfig {
  username: string;
  password: string;
  otp: string;
  gamePath: string;
  isSteam: boolean;
  language: number;
  dx11: boolean;
  expansionLevel: number;
  region: number;
  
  // Game settings
  isFreeTrial: boolean;
  dpiAwareness: 'Aware' | 'Unaware';
  additionalLaunchArgs: string;
  encryptArguments: boolean;
  savedLogin: boolean;
  autoLogin: boolean;
  directXVersion: '11' | '9';
  clientLanguage: 'Japanese' | 'English' | 'German' | 'French';

  // Dalamud settings
  dalamudEnabled: boolean;
  dalamudPath: string;
  dalamudInjectDelay: number;
  dalamudConfigPath: string;
  dalamudPluginPath: string;
  dalamudDevPluginPath: string;
  dalamudAssetPath: string;
}

// Default paths based on platform
const BASE_PATHS = {
  windows: {
    dalamud: '%APPDATA%/XIVLoader/Dalamud',
    config: '%APPDATA%/XIVLoader/config',
    plugins: '%APPDATA%/XIVLoader/plugins',
    devPlugins: '%APPDATA%/XIVLoader/devPlugins',
    assets: '%APPDATA%/XIVLoader/assets',
  },
  linux: {
    dalamud: '$HOME/.local/share/XIVLoader/Dalamud',
    config: '$HOME/.config/XIVLoader',
    plugins: '$HOME/.local/share/XIVLoader/plugins',
    devPlugins: '$HOME/.local/share/XIVLoader/devPlugins',
    assets: '$HOME/.local/share/XIVLoader/assets',
  }
};

// Platform-specific paths
const PATHS = {
  dalamudBase: BASE_PATHS.windows.dalamud, // Default to Windows, will be updated at runtime
  configDir: BASE_PATHS.windows.config,
  pluginsDir: BASE_PATHS.windows.plugins,
  devPluginsDir: BASE_PATHS.windows.devPlugins,
  assetsDir: BASE_PATHS.windows.assets,
};

// Initialize paths based on platform
async function initializePaths() {
  try {
    const appData = await appDataDir();
    const home = await homeDir();
    
    // Check if we're on Windows by looking for a drive letter
    const isWindows = appData.includes(':');
    
    if (isWindows) {
      PATHS.dalamudBase = await join(appData, 'XIVLoader', 'Dalamud');
      PATHS.configDir = await join(appData, 'XIVLoader', 'config');
      PATHS.pluginsDir = await join(appData, 'XIVLoader', 'plugins');
      PATHS.devPluginsDir = await join(appData, 'XIVLoader', 'devPlugins');
      PATHS.assetsDir = await join(appData, 'XIVLoader', 'assets');
    } else {
      PATHS.dalamudBase = await join(home, '.local', 'share', 'XIVLoader', 'Dalamud');
      PATHS.configDir = await join(home, '.config', 'XIVLoader');
      PATHS.pluginsDir = await join(home, '.local', 'share', 'XIVLoader', 'plugins');
      PATHS.devPluginsDir = await join(home, '.local', 'share', 'XIVLoader', 'devPlugins');
      PATHS.assetsDir = await join(home, '.local', 'share', 'XIVLoader', 'assets');
    }
  } catch (error) {
    console.error('Failed to initialize paths:', error);
  }
}

// Call initialization
initializePaths();

const initialConfig: GameConfig = {
  username: '',
  password: '',
  otp: '',
  gamePath: '',
  isSteam: false,
  language: 1,
  dx11: true,
  expansionLevel: 4,
  region: 3,
  
  // Game settings
  isFreeTrial: false,
  dpiAwareness: 'Aware',
  additionalLaunchArgs: '',
  encryptArguments: true,
  savedLogin: false,
  autoLogin: false,
  directXVersion: '11',
  clientLanguage: 'English',

  // Dalamud settings
  dalamudEnabled: true,
  dalamudPath: PATHS.dalamudBase,
  dalamudInjectDelay: 0,
  dalamudConfigPath: PATHS.configDir,
  dalamudPluginPath: PATHS.pluginsDir,
  dalamudDevPluginPath: PATHS.devPluginsDir,
  dalamudAssetPath: PATHS.assetsDir,
};

export const gameConfig = writable<GameConfig>(initialConfig);

// Export paths for use in other components
export const XIVLOADER_PATHS = PATHS;
