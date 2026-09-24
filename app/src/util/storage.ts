type StorageKey = (typeof STORAGE_KEYS)[keyof typeof STORAGE_KEYS];

export const STORAGE_KEYS = {
    logVisible: "mt2mm.log",
    seenGameUpdate: "mt2mm.seenGameUpdate",
} as const;


export function readSetting(key: StorageKey): string | null {
    try {
        return localStorage.getItem(key);
    } catch {
        return null;
    }
}

export function writeSetting(key: StorageKey, value: string): void {
    try {
        localStorage.setItem(key, value);
    } catch {
        // Storage unavailable: the preference just isn't remembered.
    }
}
