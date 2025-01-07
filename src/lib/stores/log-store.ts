import { writable } from 'svelte/store';

export interface LogEntry {
    timestamp: string;
    message: string;
    type: 'error' | 'success' | 'info' | 'start';
}

function createLogStore() {
    const { subscribe, update, set } = writable<LogEntry[]>([]);

    function formatLogEntry(message: string): LogEntry {
        const timestamp = new Date().toISOString();
        const type = 
            message.toLowerCase().includes('error') || message.toLowerCase().includes('failed') ? 'error' :
            message.toLowerCase().includes('success') || message.toLowerCase().includes('completed') ? 'success' :
            message.toLowerCase().includes('starting') || message.toLowerCase().includes('attempting') ? 'start' :
            'info';

        return { timestamp, message, type };
    }

    return {
        subscribe,
        addLog: (entry: string | LogEntry) => {
            if (typeof entry === 'string') {
                update(logs => [...logs, formatLogEntry(entry)]);
            } else {
                update(logs => [...logs, {
                    ...entry,
                    timestamp: entry.timestamp || new Date().toISOString()
                }]);
            }
        },
        clear: () => set([]),
        formatLogEntry
    };
}

export const logStore = createLogStore(); 