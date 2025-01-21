import { writable, get } from 'svelte/store';
import { Client, Account, Databases, ID, Query } from 'appwrite';
import { gameConfig, type GameConfig } from './game-config';
import { settings, saveSettings, type AppSettings } from './settings-store';
import { logStore } from './log-store';

// Debug logging helper
const DEBUG = true;
function debugLog(message: string, data?: any) {
    if (DEBUG) {
        console.log(`[CloudBackup] ${message}`, data || '');
        logStore.addLog(`[Debug] ${message}`);
        if (data) {
            logStore.addLog(`[Debug] Data: ${JSON.stringify(data, null, 2)}`);
        }
    }
}

// Initialize Appwrite client with configuration
const client = new Client()
    .setEndpoint('https://cloud.appwrite.io/v1') // Cloud endpoint
    .setProject('rstest002'); // Project ID

const account = new Account(client);
const databases = new Databases(client);

// Database configuration constants
const DATABASE_ID = '678e79520038a4eafdab';
const SETTINGS_COLLECTION_ID = '678e796a00225f076de9';
const CREDENTIALS_COLLECTION_ID = 'credentialscol';

// Type definitions for cloud backup settings
interface CloudBackupSettings {
    enabled: boolean;
    autoSync: boolean;
    lastSyncTimestamp: string | null;
    syncCredentials: boolean;
    isLoggedIn: boolean;
    initialSyncComplete: boolean;
}

// Default settings configuration
const defaultCloudSettings: CloudBackupSettings = {
    enabled: false,
    autoSync: false,
    lastSyncTimestamp: null,
    syncCredentials: false,
    isLoggedIn: false,
    initialSyncComplete: false,
};

// Rate limiting and retry configuration
export const SYNC_COOLDOWN = 2000; // Minimum time between syncs (ms)
export const MAX_RETRIES = 3; // Maximum number of retry attempts
export const RETRY_DELAY = 1000; // Delay between retries (ms)
export const AUTO_SYNC_COOLDOWN = 3600000; // Auto-sync cooldown period (1 hour)

// State tracking variables
let syncQueue: (() => Promise<void>)[] = [];
let isSyncing = false;
let isRestoring = false;

// Create readable stores for sync timestamps with initial values
export const lastSettingsSyncStore = writable(Date.now());
export const lastGameConfigSyncStore = writable(Date.now());
export const lastAutoSyncStore = writable(Date.now());
export const lastSyncStore = writable(Date.now());

// Update the stores when syncs occur
function updateLastSettingsSync() {
    const timestamp = Date.now();
    lastSettingsSyncStore.set(timestamp);
    logStore.addLog(`[CloudBackup] Settings sync timestamp updated: ${new Date(timestamp).toISOString()}`);
}

function updateLastGameConfigSync() {
    const timestamp = Date.now();
    lastGameConfigSyncStore.set(timestamp);
    logStore.addLog(`[CloudBackup] Game config sync timestamp updated: ${new Date(timestamp).toISOString()}`);
}

function updateLastSync() {
    const timestamp = Date.now();
    lastSyncStore.set(timestamp);
    const isAutoSync = new Error().stack?.includes('handleAutoSync');
    if (isAutoSync) {
        lastAutoSyncStore.set(timestamp);
        logStore.addLog(`[CloudBackup] Auto sync timestamp updated: ${new Date(timestamp).toISOString()}`);
    }
    logStore.addLog(`[CloudBackup] Last sync timestamp updated: ${new Date(timestamp).toISOString()}`);
}

// Utility function to create a delay
const wait = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

// Process queued sync operations
async function processSyncQueue() {
    logStore.addLog(`[CloudBackup] Processing sync queue - Items in queue: ${syncQueue.length}`);
    logStore.addLog(`[CloudBackup] Current sync status - isSyncing: ${isSyncing}`);

    if (isSyncing || syncQueue.length === 0) {
        debugLog('Sync queue skipped - already syncing or empty queue');
        return;
    }
    
    isSyncing = true;
    debugLog(`Processing sync queue - ${syncQueue.length} items remaining`);
    
    try {
        const nextSync = syncQueue.shift();
        if (nextSync) {
            logStore.addLog('[CloudBackup] Executing next sync operation from queue');
            await nextSync();
            debugLog('Sync queue item processed successfully');
            logStore.addLog('[CloudBackup] Sync queue item completed successfully');
        }
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        console.error('Error processing sync queue:', error);
        logStore.addLog(`[CloudBackup] Sync queue processing error: ${errorMessage}`);
        if (error instanceof Error && error.stack) {
            logStore.addLog(`[CloudBackup] Error stack: ${error.stack}`);
        }
    } finally {
        isSyncing = false;
        logStore.addLog(`[CloudBackup] Sync queue status - Remaining items: ${syncQueue.length}`);
        if (syncQueue.length > 0) {
            debugLog(`Waiting ${SYNC_COOLDOWN}ms before processing next queue item`);
            logStore.addLog(`[CloudBackup] Waiting ${SYNC_COOLDOWN}ms before processing next queue item`);
            await wait(SYNC_COOLDOWN);
            processSyncQueue();
        }
    }
}

// Create and export the cloud backup store
function createCloudBackupStore() {
    const { subscribe, set, update } = writable<CloudBackupSettings>(defaultCloudSettings);

    return {
        subscribe,
        set,
        update,

        // Restore only credentials for login page
        async restoreCredentials(): Promise<boolean> {
            logStore.addLog('[CloudBackup] Starting credentials restore operation');
            const cloudSettings = get(this);
            
            if (!cloudSettings.syncCredentials) {
                logStore.addLog('[CloudBackup] Credentials restore skipped - sync not enabled');
                return false;
            }

            try {
                logStore.addLog('[CloudBackup] Fetching credentials from cloud database');
                const credentialsResponse = await databases.listDocuments(
                    DATABASE_ID,
                    CREDENTIALS_COLLECTION_ID,
                    [Query.orderDesc('$createdAt'), Query.limit(1)]
                );

                logStore.addLog(`[CloudBackup] Found ${credentialsResponse.total} credential documents`);

                if (credentialsResponse.total > 0 && credentialsResponse.documents.length > 0) {
                    const latestDoc = credentialsResponse.documents[0];
                    logStore.addLog(`[CloudBackup] Processing latest credentials (ID: ${latestDoc.$id})`);
                    
                    if (!latestDoc.text || !Array.isArray(latestDoc.text) || latestDoc.text.length === 0) {
                        logStore.addLog('[CloudBackup] Invalid credential document format - missing or empty text array');
                        return false;
                    }

                    try {
                        const parsedCredentials = JSON.parse(latestDoc.text[0]);
                        logStore.addLog('[CloudBackup] Successfully parsed credentials data');
                        
                        if (!parsedCredentials.username || !parsedCredentials.password) {
                            logStore.addLog('[CloudBackup] Invalid credentials format - missing username or password');
                            return false;
                        }

                        try {
                            const decodedUsername = atob(parsedCredentials.username);
                            const decodedPassword = atob(parsedCredentials.password);
                            
                            if (!decodedUsername || !decodedPassword) {
                                logStore.addLog('[CloudBackup] Failed to decode credentials - empty after decode');
                                return false;
                            }
                            
                            logStore.addLog('[CloudBackup] Successfully decoded credentials');
                            logStore.addLog(`[CloudBackup] Username length: ${decodedUsername.length}, Password length: ${decodedPassword.length}`);
                            
                            gameConfig.update(config => {
                                logStore.addLog('[CloudBackup] Updating game config with restored credentials');
                                return {
                                    ...config,
                                    username: decodedUsername,
                                    password: decodedPassword
                                };
                            });
                            
                            logStore.addLog('[CloudBackup] Credentials restored and applied successfully');
                            return true;
                        } catch (decodeError) {
                            const errorMessage = decodeError instanceof Error ? decodeError.message : String(decodeError);
                            console.error('[CloudBackup] Credentials decode error:', decodeError);
                            logStore.addLog(`[CloudBackup] Failed to decode credentials: ${errorMessage}`);
                            if (decodeError instanceof Error && decodeError.stack) {
                                logStore.addLog(`[CloudBackup] Decode error stack: ${decodeError.stack}`);
                            }
                            return false;
                        }
                    } catch (parseError) {
                        const errorMessage = parseError instanceof Error ? parseError.message : String(parseError);
                        console.error('[CloudBackup] Credentials parse error:', parseError);
                        logStore.addLog(`[CloudBackup] Failed to parse credentials: ${errorMessage}`);
                        if (parseError instanceof Error && parseError.stack) {
                            logStore.addLog(`[CloudBackup] Parse error stack: ${parseError.stack}`);
                        }
                        return false;
                    }
                } else {
                    logStore.addLog('[CloudBackup] No credential documents found in cloud');
                    return false;
                }
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                console.error('[CloudBackup] Credentials restore error:', error);
                logStore.addLog(`[CloudBackup] Failed to restore credentials: ${errorMessage}`);
                if (error instanceof Error && error.stack) {
                    logStore.addLog(`[CloudBackup] Error stack: ${error.stack}`);
                }
                return false;
            }
        },

        // Initialize cloud backup system
        async initialize() {
            logStore.addLog('[CloudBackup] Starting cloud backup system initialization');
            debugLog('Initializing cloud backup system');
            
            try {
                // Verify user session
                logStore.addLog('[CloudBackup] Verifying user session');
                const session = await account.get();
                
                if (!session || !session.$id) {
                    logStore.addLog('[CloudBackup] No valid session found');
                    update(state => ({ ...state, isLoggedIn: false }));
                    return;
                }

                logStore.addLog(`[CloudBackup] Active session found - ID: ${session.$id}`);
                debugLog('Active session found', session);

                const currentSettings = get(settings);
                update(state => ({ 
                    ...state, 
                    isLoggedIn: true,
                    enabled: currentSettings.cloudBackupEnabled,
                    autoSync: currentSettings.cloudBackupAutoSync,
                    syncCredentials: currentSettings.cloudBackupCredentialsSync
                }));
                
                logStore.addLog('[CloudBackup] Updated store with current settings');
                logStore.addLog(`[CloudBackup] Enabled: ${currentSettings.cloudBackupEnabled}`);
                logStore.addLog(`[CloudBackup] AutoSync: ${currentSettings.cloudBackupAutoSync}`);
                logStore.addLog(`[CloudBackup] SyncCredentials: ${currentSettings.cloudBackupCredentialsSync}`);
                
                // Attempt to load cloud settings
                try {
                    logStore.addLog('[CloudBackup] Fetching cloud settings');
                    const response = await databases.listDocuments(
                        DATABASE_ID,
                        SETTINGS_COLLECTION_ID,
                        [Query.orderDesc('$createdAt'), Query.limit(1)]
                    );

                    if (response.documents.length > 0 && response.documents[0].text?.length > 0) {
                        try {
                            const cloudSettings = JSON.parse(response.documents[0].text[0]);
                            debugLog('Loaded cloud settings', cloudSettings);
                            logStore.addLog('[CloudBackup] Successfully parsed cloud settings');
                            
                            set({
                                enabled: cloudSettings.enabled ?? defaultCloudSettings.enabled,
                                autoSync: cloudSettings.autoSync ?? defaultCloudSettings.autoSync,
                                lastSyncTimestamp: cloudSettings.lastSyncTimestamp ?? defaultCloudSettings.lastSyncTimestamp,
                                syncCredentials: cloudSettings.syncCredentials ?? defaultCloudSettings.syncCredentials,
                                isLoggedIn: true,
                                initialSyncComplete: cloudSettings.initialSyncComplete ?? false
                            });
                            
                            logStore.addLog('[CloudBackup] Cloud settings applied to store');
                            
                            // Only restore credentials on first login
                            if (!cloudSettings.initialSyncComplete) {
                                logStore.addLog('[CloudBackup] First-time login detected, attempting credentials restore');
                                await this.restoreCredentials();
                            }
                        } catch (parseError) {
                            const errorMessage = parseError instanceof Error ? parseError.message : String(parseError);
                            console.error('[CloudBackup] Settings parse error:', parseError);
                            logStore.addLog(`[CloudBackup] Failed to parse cloud settings: ${errorMessage}`);
                            if (parseError instanceof Error && parseError.stack) {
                                logStore.addLog(`[CloudBackup] Parse error stack: ${parseError.stack}`);
                            }
                        }
                    } else {
                        debugLog('No existing cloud settings found');
                        logStore.addLog('[CloudBackup] No cloud settings found, using defaults');
                    }
                } catch (settingsError) {
                    const errorMessage = settingsError instanceof Error ? settingsError.message : String(settingsError);
                    console.error('[CloudBackup] Error loading cloud settings:', settingsError);
                    logStore.addLog(`[CloudBackup] Failed to load cloud settings: ${errorMessage}`);
                    if (settingsError instanceof Error && settingsError.stack) {
                        logStore.addLog(`[CloudBackup] Error stack: ${settingsError.stack}`);
                    }
                }
            } catch (error: unknown) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                
                if (errorMessage.includes('User not found')) {
                    debugLog('Session validation failed - user not found');
                    update(state => ({ ...state, isLoggedIn: false }));
                    logStore.addLog('[CloudBackup] No active session found - user not found');
                } else {
                    console.error('[CloudBackup] Initialization error:', error);
                    logStore.addLog(`[CloudBackup] Initialization failed: ${errorMessage}`);
                    if (error instanceof Error && error.stack) {
                        logStore.addLog(`[CloudBackup] Error stack: ${error.stack}`);
                    }
                }
            }
        },

        // Handle user login
        async login(email: string, password: string) {
            logStore.addLog('[CloudBackup] Starting login process');
            debugLog('Attempting login', { email });
            
            if (!email || !password) {
                logStore.addLog('[CloudBackup] Login failed - missing email or password');
                return false;
            }
            
            try {
                // Clear any existing session
                try {
                    await account.deleteSession('current');
                    debugLog('Cleared existing session');
                    logStore.addLog('[CloudBackup] Cleared existing session');
                } catch (e) {
                    debugLog('No existing session to clear');
                    logStore.addLog('[CloudBackup] No existing session found to clear');
                }

                // Create new session
                const session = await account.createEmailPasswordSession(email, password);
                logStore.addLog(`[CloudBackup] Created new session - ID: ${session.$id}`);
                
                await this.initialize();
                
                // Attempt to restore credentials after successful login
                const cloudBackupSettings = get(this);
                if (cloudBackupSettings.syncCredentials) {
                    logStore.addLog('[CloudBackup] Credentials sync enabled, attempting restore');
                    await this.restoreCredentials();
                } else {
                    logStore.addLog('[CloudBackup] Credentials sync disabled, skipping restore');
                }
                
                debugLog('Login successful');
                logStore.addLog('[CloudBackup] Login completed successfully');
                return true;
            } catch (error: unknown) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                console.error('[CloudBackup] Login error:', error);
                logStore.addLog(`[CloudBackup] Login failed: ${errorMessage}`);
                if (error instanceof Error && error.stack) {
                    logStore.addLog(`[CloudBackup] Error stack: ${error.stack}`);
                }
                return false;
            }
        },

        // Handle user logout
        async logout() {
            logStore.addLog('[CloudBackup] Starting logout process');
            debugLog('Attempting logout');
            
            try {
                await account.deleteSession('current');
                set({ ...defaultCloudSettings, isLoggedIn: false });
                debugLog('Logout successful');
                logStore.addLog('[CloudBackup] Logged out successfully');
                return true;
            } catch (error: unknown) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                console.error('[CloudBackup] Logout error:', error);
                logStore.addLog(`[CloudBackup] Logout failed: ${errorMessage}`);
                if (error instanceof Error && error.stack) {
                    logStore.addLog(`[CloudBackup] Error stack: ${error.stack}`);
                }
                return false;
            }
        },

        // Sync data to cloud
        async syncToCloud() {
            logStore.addLog('[CloudBackup] Starting cloud sync operation');
            const now = Date.now();
            const isAutoSync = new Error().stack?.includes('handleAutoSync');
            
            debugLog(`Starting sync operation (Auto: ${isAutoSync})`);
            logStore.addLog(`[CloudBackup] Sync type: ${isAutoSync ? 'Auto' : 'Manual'}`);
            
            // Handle auto-sync cooldown
            if (isAutoSync) {
                const lastAutoSync = get(lastAutoSyncStore);
                if (now - lastAutoSync < AUTO_SYNC_COOLDOWN) {
                    const remainingTime = AUTO_SYNC_COOLDOWN - (now - lastAutoSync);
                    debugLog(`Auto-sync skipped - within cooldown period (${remainingTime}ms remaining)`);
                    logStore.addLog(`[CloudBackup] Auto-sync skipped - Cooldown remaining: ${remainingTime}ms`);
                    return true;
                }
            }

            // Handle rate limiting
            const lastSync = get(lastSyncStore);
            if (now - lastSync < SYNC_COOLDOWN) {
                const waitTime = SYNC_COOLDOWN - (now - lastSync);
                debugLog(`Queueing sync operation - Rate limit cooldown: ${waitTime}ms`);
                logStore.addLog(`[CloudBackup] Queueing sync - Rate limit cooldown: ${waitTime}ms`);
                
                return new Promise<boolean>((resolve) => {
                    syncQueue.push(async () => {
                        const result = await this.syncToCloudWithRetry();
                        resolve(result);
                    });
                    processSyncQueue();
                });
            }

            updateLastSync();
            return this.syncToCloudWithRetry();
        },

        // Sync to cloud with retry logic
        async syncToCloudWithRetry(retryCount = 0): Promise<boolean> {
            logStore.addLog(`[CloudBackup] Starting sync attempt ${retryCount + 1}/${MAX_RETRIES + 1}`);
            
            if (isRestoring) {
                debugLog('Sync skipped - restore in progress');
                logStore.addLog('[CloudBackup] Sync skipped - restore operation in progress');
                return true;
            }
            
            const now = Date.now();
            const lastSettingsTime = get(lastSettingsSyncStore);
            const lastGameConfigTime = get(lastGameConfigSyncStore);
            
            // Skip if both settings and game config are within cooldown
            if (now - lastSettingsTime < SYNC_COOLDOWN && now - lastGameConfigTime < SYNC_COOLDOWN) {
                debugLog('Sync skipped - both settings and game config within cooldown');
                logStore.addLog('[CloudBackup] Sync skipped - all components within cooldown period');
                return true;
            }
            
            try {
                debugLog(`Starting sync attempt ${retryCount + 1}/${MAX_RETRIES + 1}`);
                
                const currentSettings = get(settings);
                const currentGameConfig = get(gameConfig);
                const cloudBackupSettings = get(this);

                // Sync settings if cooldown has passed
                if (now - lastSettingsTime >= SYNC_COOLDOWN) {
                    updateLastSettingsSync();
                    debugLog('Syncing settings to cloud');
                    logStore.addLog('[CloudBackup] Starting settings sync');
                    
                    const settingsData = {
                        text: [
                            JSON.stringify({
                                cloudBackup: {
                                    enabled: cloudBackupSettings.enabled,
                                    autoSync: cloudBackupSettings.autoSync,
                                    lastSyncTimestamp: new Date().toISOString(),
                                    syncCredentials: cloudBackupSettings.syncCredentials,
                                    initialSyncComplete: cloudBackupSettings.initialSyncComplete
                                },
                                appSettings: {
                                    useCustomTitlebar: currentSettings.useCustomTitlebar,
                                    centerTitle: currentSettings.centerTitle,
                                    showMinimize: currentSettings.showMinimize,
                                    showMaximize: currentSettings.showMaximize,
                                    theme: currentSettings.theme
                                },
                                gameConfig: {
                                    language: currentGameConfig.language,
                                    dx11: currentGameConfig.dx11,
                                    expansionLevel: currentGameConfig.expansionLevel,
                                    region: currentGameConfig.region,
                                    isFreeTrial: currentGameConfig.isFreeTrial,
                                    dpiAwareness: currentGameConfig.dpiAwareness,
                                    additionalLaunchArgs: currentGameConfig.additionalLaunchArgs,
                                    encryptArguments: currentGameConfig.encryptArguments,
                                    savedLogin: currentGameConfig.savedLogin,
                                    autoLogin: currentGameConfig.autoLogin,
                                    directXVersion: currentGameConfig.directXVersion,
                                    clientLanguage: currentGameConfig.clientLanguage,
                                    dalamudEnabled: currentGameConfig.dalamudEnabled,
                                    dalamudInjectDelay: currentGameConfig.dalamudInjectDelay
                                }
                            })
                        ]
                    };
                    
                    const settingsDoc = await databases.createDocument(
                        DATABASE_ID,
                        SETTINGS_COLLECTION_ID,
                        ID.unique(),
                        settingsData
                    );
                    
                    debugLog('Settings sync successful');
                    logStore.addLog(`[CloudBackup] Settings synced successfully - Document ID: ${settingsDoc.$id}`);
                }

                // Sync credentials if enabled and cooldown has passed
                if (cloudBackupSettings.syncCredentials && now - lastGameConfigTime >= SYNC_COOLDOWN) {
                    if (!currentGameConfig.username || !currentGameConfig.password) {
                        logStore.addLog('[CloudBackup] Credentials sync skipped - missing username or password');
                    } else {
                        updateLastGameConfigSync();
                        debugLog('Syncing credentials to cloud');
                        logStore.addLog('[CloudBackup] Starting credentials sync');
                        
                        try {
                            const encodedUsername = btoa(currentGameConfig.username);
                            const encodedPassword = btoa(currentGameConfig.password);
                            
                            logStore.addLog('[CloudBackup] Successfully encoded credentials');
                            logStore.addLog(`[CloudBackup] Encoded lengths - Username: ${encodedUsername.length}, Password: ${encodedPassword.length}`);
                            
                            const credentialsData = {
                                text: [
                                    JSON.stringify({
                                        username: encodedUsername,
                                        password: encodedPassword
                                    })
                                ]
                            };
                            
                            const credentialsDoc = await databases.createDocument(
                                DATABASE_ID,
                                CREDENTIALS_COLLECTION_ID,
                                ID.unique(),
                                credentialsData
                            );
                            
                            debugLog('Credentials sync successful');
                            logStore.addLog(`[CloudBackup] Credentials synced successfully - Document ID: ${credentialsDoc.$id}`);
                        } catch (encodeError) {
                            const errorMessage = encodeError instanceof Error ? encodeError.message : String(encodeError);
                            console.error('[CloudBackup] Credentials encode error:', encodeError);
                            logStore.addLog(`[CloudBackup] Failed to encode credentials: ${errorMessage}`);
                            if (encodeError instanceof Error && encodeError.stack) {
                                logStore.addLog(`[CloudBackup] Encode error stack: ${encodeError.stack}`);
                            }
                            throw encodeError;
                        }
                    }
                }

                update(state => ({
                    ...state,
                    lastSyncTimestamp: new Date().toISOString()
                }));

                logStore.addLog('[CloudBackup] Sync completed successfully');
                return true;
            } catch (error: any) {
                console.error('[CloudBackup] Sync error:', error);
                
                if (error.message?.includes('Rate limit') && retryCount < MAX_RETRIES) {
                    debugLog(`Rate limit hit, retrying (${retryCount + 1}/${MAX_RETRIES})`);
                    logStore.addLog(`[CloudBackup] Rate limit hit, retrying in ${RETRY_DELAY/1000} seconds... (Attempt ${retryCount + 1}/${MAX_RETRIES})`);
                    await wait(RETRY_DELAY);
                    return this.syncToCloudWithRetry(retryCount + 1);
                }

                logStore.addLog(`[CloudBackup] Sync failed: ${error.message}`);
                if (error.stack) {
                    logStore.addLog(`[CloudBackup] Error stack: ${error.stack}`);
                }
                return false;
            }
        },

        // Restore data from cloud
        async restoreFromCloud(retryCount = 0): Promise<boolean> {
            logStore.addLog(`[CloudRestore] Starting restore process (Attempt ${retryCount + 1}/${MAX_RETRIES + 1})`);
            
            if (isRestoring) {
                logStore.addLog('[CloudRestore] Restore already in progress, skipping');
                debugLog('Restore skipped - already in progress');
                return true;
            }
            
            try {
                isRestoring = true;
                // Set initial sync times to prevent immediate auto-sync after restore
                const now = Date.now();
                lastSettingsSyncStore.set(now);
                lastGameConfigSyncStore.set(now);
                lastAutoSyncStore.set(now);
                lastSyncStore.set(now);
                
                debugLog(`Starting restore attempt ${retryCount + 1}/${MAX_RETRIES + 1}`);
                
                const lastSync = get(lastSyncStore);
                if (now - lastSync < SYNC_COOLDOWN) {
                    const waitTime = SYNC_COOLDOWN - (now - lastSync);
                    logStore.addLog(`[CloudRestore] Rate limit cooldown - waiting ${waitTime}ms`);
                    debugLog(`Waiting ${waitTime}ms for rate limit`);
                    await wait(waitTime);
                }
                
                // Restore settings
                logStore.addLog('[CloudRestore] Fetching settings from cloud database');
                debugLog('Fetching settings from cloud');
                const settingsResponse = await databases.listDocuments(
                    DATABASE_ID,
                    SETTINGS_COLLECTION_ID,
                    [
                        Query.orderDesc('$createdAt'),
                        Query.limit(1)
                    ]
                );

                logStore.addLog(`[CloudRestore] Found ${settingsResponse.total} settings documents`);

                if (settingsResponse.total > 0 && settingsResponse.documents.length > 0) {
                    const latestDoc = settingsResponse.documents[0];
                    logStore.addLog(`[CloudRestore] Processing latest settings document (ID: ${latestDoc.$id})`);
                    
                    if (latestDoc.text && Array.isArray(latestDoc.text) && latestDoc.text.length > 0) {
                        try {
                            const parsedData = JSON.parse(latestDoc.text[0]);
                            debugLog('Parsed cloud settings', parsedData);
                            logStore.addLog('[CloudRestore] Successfully parsed settings data');

                            if (parsedData.cloudBackup) {
                                logStore.addLog('[CloudRestore] Restoring cloud backup settings');
                                logStore.addLog(`[CloudRestore] - Enabled: ${parsedData.cloudBackup.enabled}`);
                                logStore.addLog(`[CloudRestore] - AutoSync: ${parsedData.cloudBackup.autoSync}`);
                                logStore.addLog(`[CloudRestore] - SyncCredentials: ${parsedData.cloudBackup.syncCredentials}`);
                                
                                update(state => ({
                                    ...state,
                                    enabled: parsedData.cloudBackup.enabled ?? state.enabled,
                                    autoSync: parsedData.cloudBackup.autoSync ?? state.autoSync,
                                    lastSyncTimestamp: parsedData.cloudBackup.lastSyncTimestamp ?? state.lastSyncTimestamp,
                                    syncCredentials: parsedData.cloudBackup.syncCredentials ?? state.syncCredentials,
                                    isLoggedIn: true
                                }));
                                debugLog('Cloud backup settings restored');
                                logStore.addLog('[CloudRestore] Cloud backup settings restored successfully');
                            }

                            if (parsedData.appSettings) {
                                logStore.addLog('[CloudRestore] Restoring application settings');
                                try {
                                    const { getCurrentWindow } = await import('@tauri-apps/api/window');
                                    
                                    const window = await getCurrentWindow();
                                    const currentSettings = get(settings);

                                    const newSettings: AppSettings = {
                                        ...currentSettings,
                                        useCustomTitlebar: parsedData.appSettings.useCustomTitlebar ?? currentSettings.useCustomTitlebar,
                                        centerTitle: parsedData.appSettings.centerTitle ?? currentSettings.centerTitle,
                                        showMinimize: parsedData.appSettings.showMinimize ?? currentSettings.showMinimize,
                                        showMaximize: parsedData.appSettings.showMaximize ?? currentSettings.showMaximize,
                                        theme: parsedData.appSettings.theme ?? currentSettings.theme,
                                        // Keep existing cloud backup settings
                                        cloudBackupEnabled: currentSettings.cloudBackupEnabled,
                                        cloudBackupAutoSync: currentSettings.cloudBackupAutoSync,
                                        cloudBackupCredentialsSync: currentSettings.cloudBackupCredentialsSync
                                    };

                                    logStore.addLog('[CloudRestore] Applying new settings:');
                                    logStore.addLog(`[CloudRestore] - UseCustomTitlebar: ${newSettings.useCustomTitlebar}`);
                                    logStore.addLog(`[CloudRestore] - CenterTitle: ${newSettings.centerTitle}`);
                                    logStore.addLog(`[CloudRestore] - ShowMinimize: ${newSettings.showMinimize}`);
                                    logStore.addLog(`[CloudRestore] - ShowMaximize: ${newSettings.showMaximize}`);
                                    logStore.addLog(`[CloudRestore] - Theme: ${newSettings.theme}`);

                                    await saveSettings(newSettings);
                                    logStore.addLog('[CloudRestore] Settings saved to store');
                                    
                                    await window.emit('titlebar-settings-changed', newSettings);
                                    logStore.addLog('[CloudRestore] Titlebar settings event emitted');

                                    // Update window decorations based on settings
                                    if (newSettings.useCustomTitlebar) {
                                        logStore.addLog('[CloudRestore] Enabling custom titlebar');
                                        await window.setDecorations(false);
                                        document.documentElement.classList.add('custom-titlebar-enabled');
                                        document.body.classList.add('titlebar-enabled');
                                    } else {
                                        logStore.addLog('[CloudRestore] Disabling custom titlebar');
                                        await window.setDecorations(true);
                                        document.documentElement.classList.remove('custom-titlebar-enabled');
                                        document.body.classList.remove('titlebar-enabled');
                                    }

                                    // Update maximized state
                                    const isMaximized = await window.isMaximized();
                                    logStore.addLog(`[CloudRestore] Window maximized state: ${isMaximized}`);
                                    if (isMaximized) {
                                        document.documentElement.classList.add('maximized');
                                        document.body.classList.add('maximized');
                                    }

                                    debugLog('App settings restored and applied');
                                    logStore.addLog('[CloudRestore] App settings restored and applied successfully');
                                } catch (error) {
                                    console.error('Settings application error:', error);
                                    logStore.addLog(`[CloudRestore] Failed to apply settings: ${error}`);
                                    return false;
                                }
                            }

                            // Restore game config settings
                            if (parsedData.gameConfig) {
                                logStore.addLog('[CloudRestore] Restoring game configuration');
                                try {
                                    const currentGameConfig = get(gameConfig);
                                    
                                    const newGameConfig = {
                                        ...currentGameConfig,
                                        language: parsedData.gameConfig.language ?? currentGameConfig.language,
                                        dx11: parsedData.gameConfig.dx11 ?? currentGameConfig.dx11,
                                        expansionLevel: parsedData.gameConfig.expansionLevel ?? currentGameConfig.expansionLevel,
                                        region: parsedData.gameConfig.region ?? currentGameConfig.region,
                                        isFreeTrial: parsedData.gameConfig.isFreeTrial ?? currentGameConfig.isFreeTrial,
                                        dpiAwareness: parsedData.gameConfig.dpiAwareness ?? currentGameConfig.dpiAwareness,
                                        additionalLaunchArgs: parsedData.gameConfig.additionalLaunchArgs ?? currentGameConfig.additionalLaunchArgs,
                                        encryptArguments: parsedData.gameConfig.encryptArguments ?? currentGameConfig.encryptArguments,
                                        savedLogin: parsedData.gameConfig.savedLogin ?? currentGameConfig.savedLogin,
                                        autoLogin: parsedData.gameConfig.autoLogin ?? currentGameConfig.autoLogin,
                                        directXVersion: parsedData.gameConfig.directXVersion ?? currentGameConfig.directXVersion,
                                        clientLanguage: parsedData.gameConfig.clientLanguage ?? currentGameConfig.clientLanguage,
                                        dalamudEnabled: parsedData.gameConfig.dalamudEnabled ?? currentGameConfig.dalamudEnabled,
                                        dalamudInjectDelay: parsedData.gameConfig.dalamudInjectDelay ?? currentGameConfig.dalamudInjectDelay
                                    };

                                    logStore.addLog('[CloudRestore] Applying game configuration:');
                                    Object.entries(newGameConfig).forEach(([key, value]) => {
                                        if (value !== currentGameConfig[key as keyof GameConfig]) {
                                            logStore.addLog(`[CloudRestore] - ${key}: ${value}`);
                                        }
                                    });

                                    gameConfig.set(newGameConfig);
                                    debugLog('Game configuration restored and applied');
                                    logStore.addLog('[CloudRestore] Game configuration restored and applied successfully');
                                } catch (error) {
                                    console.error('Game config application error:', error);
                                    logStore.addLog(`[CloudRestore] Failed to apply game configuration: ${error}`);
                                    return false;
                                }
                            }
                        } catch (parseError) {
                            console.error('Settings parse error:', parseError);
                            logStore.addLog('[CloudRestore] Failed to parse settings from cloud');
                            return false;
                        }
                    }
                }

                // Restore credentials if enabled
                const cloudBackupSettings = get(this);
                if (cloudBackupSettings.syncCredentials) {
                    logStore.addLog('[CloudRestore] Starting credentials restore');
                    debugLog('Restoring credentials from cloud');
                    const credentialsResponse = await databases.listDocuments(
                        DATABASE_ID,
                        CREDENTIALS_COLLECTION_ID,
                        [
                            Query.orderDesc('$createdAt'),
                            Query.limit(1)
                        ]
                    );

                    logStore.addLog(`[CloudRestore] Found ${credentialsResponse.total} credentials documents`);

                    if (credentialsResponse.total > 0 && credentialsResponse.documents.length > 0) {
                        const latestDoc = credentialsResponse.documents[0];
                        logStore.addLog(`[CloudRestore] Processing latest credentials document (ID: ${latestDoc.$id})`);
                        
                        if (latestDoc.text && Array.isArray(latestDoc.text) && latestDoc.text.length > 0) {
                            try {
                                const parsedCredentials = JSON.parse(latestDoc.text[0]);
                                logStore.addLog('[CloudRestore] Successfully parsed credentials data');

                                if (parsedCredentials.username && parsedCredentials.password) {
                                    gameConfig.update(config => ({
                                        ...config,
                                        username: atob(parsedCredentials.username),
                                        password: atob(parsedCredentials.password)
                                    }));
                                    debugLog('Credentials restored successfully');
                                    logStore.addLog('[CloudRestore] Credentials restored and applied successfully');
                                }
                            } catch (parseError) {
                                console.error('Credentials parse error:', parseError);
                                logStore.addLog('[CloudRestore] Failed to parse credentials from cloud');
                                return false;
                            }
                        }
                    }
                }

                // After successful restore, update sync timestamps again to prevent immediate sync
                const completeTime = Date.now();
                lastSettingsSyncStore.set(completeTime);
                lastGameConfigSyncStore.set(completeTime);
                lastAutoSyncStore.set(completeTime);
                lastSyncStore.set(completeTime);
                
                logStore.addLog('[CloudRestore] Restore process completed successfully');
                return true;
            } catch (error: any) {
                console.error('Restore error:', error);
                
                if (error.message?.includes('Rate limit') && retryCount < MAX_RETRIES) {
                    logStore.addLog(`[CloudRestore] Rate limit hit, retrying (${retryCount + 1}/${MAX_RETRIES})`);
                    debugLog(`Rate limit hit during restore, retrying (${retryCount + 1}/${MAX_RETRIES})`);
                    await wait(RETRY_DELAY);
                    return this.restoreFromCloud(retryCount + 1);
                }

                logStore.addLog(`[CloudRestore] Failed to restore from cloud: ${error.message}`);
                return false;
            } finally {
                isRestoring = false;
                logStore.addLog('[CloudRestore] Restore process finished, cleanup complete');
            }
        },

        // Toggle cloud backup functionality
        async toggleCloudBackup(enabled: boolean) {
            debugLog(`Toggling cloud backup: ${enabled}`);
            update(state => ({ ...state, enabled }));
            const currentSettings = get(settings);
            await saveSettings({
                ...currentSettings,
                cloudBackupEnabled: enabled
            });
            if (enabled) {
                await this.syncToCloud();
            }
        },

        // Toggle auto-sync functionality
        async toggleAutoSync(enabled: boolean) {
            debugLog(`Toggling auto-sync: ${enabled}`);
            update(state => ({ ...state, autoSync: enabled }));
            const currentSettings = get(settings);
            await saveSettings({
                ...currentSettings,
                cloudBackupAutoSync: enabled
            });
        },

        // Sync only credentials to cloud
        async syncCredentialsToCloud(retryCount = 0): Promise<boolean> {
            logStore.addLog('[CloudBackup] Starting credentials sync');
            
            if (!get(this).syncCredentials) {
                logStore.addLog('[CloudBackup] Credentials sync skipped - sync not enabled');
                return false;
            }

            if (isRestoring) {
                logStore.addLog('[CloudBackup] Credentials sync skipped - restore in progress');
                return false;
            }

            try {
                const now = Date.now();
                const lastGameConfigTime = get(lastGameConfigSyncStore);
                
                // Check cooldown
                if (now - lastGameConfigTime < SYNC_COOLDOWN) {
                    const waitTime = SYNC_COOLDOWN - (now - lastGameConfigTime);
                    logStore.addLog(`[CloudBackup] Rate limit cooldown - waiting ${waitTime}ms`);
                    await wait(waitTime);
                }

                const currentGameConfig = get(gameConfig);
                
                // Verify we have credentials to sync
                if (!currentGameConfig.username || !currentGameConfig.password) {
                    logStore.addLog('[CloudBackup] No credentials to sync');
                    return false;
                }

                logStore.addLog('[CloudBackup] Preparing credentials for sync');
                const credentialsData = {
                    text: [
                        JSON.stringify({
                            username: btoa(currentGameConfig.username),
                            password: btoa(currentGameConfig.password)
                        })
                    ]
                };
                
                await databases.createDocument(
                    DATABASE_ID,
                    CREDENTIALS_COLLECTION_ID,
                    ID.unique(),
                    credentialsData
                );

                updateLastGameConfigSync();
                logStore.addLog('[CloudBackup] Credentials synced successfully');
                return true;
            } catch (error: any) {
                console.error('[CloudBackup] Credentials sync error:', error);
                
                if (error.message?.includes('Rate limit') && retryCount < MAX_RETRIES) {
                    logStore.addLog(`[CloudBackup] Rate limit hit, retrying (${retryCount + 1}/${MAX_RETRIES})`);
                    await wait(RETRY_DELAY);
                    return this.syncCredentialsToCloud(retryCount + 1);
                }

                logStore.addLog(`[CloudBackup] Failed to sync credentials: ${error.message}`);
                return false;
            }
        },

        // Toggle credentials sync functionality with immediate sync
        async toggleCredentialsSync(enabled: boolean) {
            debugLog(`Toggling credentials sync: ${enabled}`);
            update(state => ({ ...state, syncCredentials: enabled }));
            const currentSettings = get(settings);
            await saveSettings({
                ...currentSettings,
                cloudBackupCredentialsSync: enabled
            });
            
            if (enabled) {
                logStore.addLog('[CloudBackup] Credentials sync enabled, performing initial sync');
                await this.syncCredentialsToCloud();
            } else {
                logStore.addLog('[CloudBackup] Credentials sync disabled, removing stored credentials');
                try {
                    const credentialsResponse = await databases.listDocuments(
                        DATABASE_ID,
                        CREDENTIALS_COLLECTION_ID
                    );
                    for (const doc of credentialsResponse.documents) {
                        await databases.deleteDocument(
                            DATABASE_ID,
                            CREDENTIALS_COLLECTION_ID,
                            doc.$id
                        );
                    }
                    logStore.addLog('[CloudBackup] Stored credentials removed successfully');
                } catch (error: any) {
                    console.error('[CloudBackup] Error removing credentials:', error);
                    logStore.addLog(`[CloudBackup] Failed to remove stored credentials: ${error.message}`);
                }
            }
        },

        // Handle user registration
        async register(email: string, password: string) {
            debugLog('Attempting user registration', { email });
            
            try {
                await account.create(ID.unique(), email, password);
                debugLog('Account created successfully');
                logStore.addLog('Account created successfully');
                
                return await this.login(email, password);
            } catch (error: unknown) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                console.error('Registration error:', error);
                logStore.addLog(`Account creation failed: ${errorMessage}`);
                return false;
            }
        }
    };
}

export const cloudBackup = createCloudBackupStore();