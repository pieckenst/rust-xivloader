import { writable } from 'svelte/store';
import { Store } from '@tauri-apps/plugin-store';
import { logStore } from './log-store'; // Assuming logStore is imported from another file

interface AppSettings {
  useCustomTitlebar: boolean;
  theme: 'light' | 'dark' | 'system';
  centerTitle: boolean;
  showMinimize: boolean;
  showMaximize: boolean;
  // Cloud backup settings
  cloudBackupEnabled: boolean;
  cloudBackupAutoSync: boolean;
  cloudBackupCredentialsSync: boolean;
}

const defaultSettings: AppSettings = {
  useCustomTitlebar: true,
  theme: 'system',
  centerTitle: true,
  showMinimize: true,
  showMaximize: true,
  // Default cloud backup settings
  cloudBackupEnabled: false,
  cloudBackupAutoSync: false,
  cloudBackupCredentialsSync: false
};

let store: Store;

// Create a writable store with the default settings
const settings = writable<AppSettings>(defaultSettings);

// Load settings from storage
async function loadSettings() {
  try {
    store = await Store.load('settings.json');
    const storedSettings = await store.get<AppSettings>('settings');
    if (storedSettings) {
      settings.set(storedSettings);
      logStore.addLog('Settings loaded successfully');
    } else {
      await store.set('settings', defaultSettings);
      logStore.addLog('Default settings applied');
    }
  } catch (error) {
    console.error('Failed to load settings:', error);
    logStore.addLog(`Failed to load settings: ${error}`);
  }
}

// Save settings to storage
async function saveSettings(newSettings: AppSettings) {
  try {
    if (!store) {
      store = await Store.load('settings.json');
    }
    console.log('New settings:', newSettings); // Log the new settings
    await store.set('settings', newSettings);
    settings.set(newSettings);
    await store.save(); // Ensure changes are persisted
    logStore.addLog('Settings saved successfully');
  } catch (error) {
    console.error('Failed to save settings:', error);
    logStore.addLog(`Failed to save settings: ${error}`);
  }
}

// Initialize settings
loadSettings();

export { settings, saveSettings, type AppSettings }; 