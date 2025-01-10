import { writable } from 'svelte/store';
import { Store } from '@tauri-apps/plugin-store';

interface AppSettings {
  useCustomTitlebar: boolean;
  theme: 'light' | 'dark' | 'system';
  centerTitle: boolean;
  showMinimize: boolean;
  showMaximize: boolean;
}

const defaultSettings: AppSettings = {
  useCustomTitlebar: true,
  theme: 'system',
  centerTitle: true,
  showMinimize: true,
  showMaximize: true
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
    } else {
      await store.set('settings', defaultSettings);
    }
  } catch (error) {
    console.error('Failed to load settings:', error);
  }
}

// Save settings to storage
async function saveSettings(newSettings: AppSettings) {
  try {
    if (!store) {
      store = await Store.load('settings.json');
    }
    await store.set('settings', newSettings);
    settings.set(newSettings);
    await store.save(); // Ensure changes are persisted
  } catch (error) {
    console.error('Failed to save settings:', error);
  }
}

// Initialize settings
loadSettings();

export { settings, saveSettings, type AppSettings }; 