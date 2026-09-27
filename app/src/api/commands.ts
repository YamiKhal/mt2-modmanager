import { invoke } from "@tauri-apps/api/core";
import type {
    BuildPlan,
    GameConfig,
    GamePaths,
    Imported,
    LaunchResult,
    NativeRequest,
    SettingValue,
    Status,
} from "./types";

export type FolderTarget = "mod" | "library" | "profile" | "install" | `mod:${string}`;


export function getStatus(): Promise<Status> {
    return invoke<Status>("status");
}

export function setModEnabled(id: string, enabled: boolean): Promise<void> {
    return invoke("set_enabled", { id, enabled });
}

export function setLoadOrder(order: string[]): Promise<void> {
    return invoke("set_order", { order });
}

export function applyModsToProfile(ids: string[]): Promise<void> {
    return invoke("apply_mods", { ids });
}

export function removeModsFromProfile(ids: string[]): Promise<void> {
    return invoke("unapply_mods", { ids });
}

export function setModSettings(
    id: string,
    changes: Record<string, SettingValue>,
    reset: string[],
): Promise<void> {
    return invoke("set_mod_settings", { id, changes, reset });
}


export function addMods(paths: string[]): Promise<Imported[]> {
    return invoke<Imported[]>("add_mods", { paths });
}

export function adoptMod(path: string): Promise<Imported> {
    return invoke<Imported>("adopt_mod", { path });
}

export function addDevMod(path: string): Promise<Imported> {
    return invoke<Imported>("add_dev_mod", { path });
}

export function refreshDevMods(ids: string[]): Promise<Imported[]> {
    return invoke<Imported[]>("refresh_dev_mods", { ids });
}

export function stopDevMod(id: string): Promise<void> {
    return invoke("stop_dev_mod", { id });
}

export function removeModFromLibrary(id: string): Promise<void> {
    return invoke("remove_mod", { id });
}

// Tauri's postMessage fallback (always used on macOS) sends raw bytes as a number array.
export async function getModIcon(id: string): Promise<Uint8Array<ArrayBuffer>> {
    const bytes = await invoke<ArrayBuffer | number[]>("mod_icon", { id });

    return new Uint8Array(bytes);
}


export function createProfile(name: string, copyActive: boolean): Promise<string> {
    return invoke<string>("create_profile", { name, copyActive });
}

export function switchProfile(name: string): Promise<void> {
    return invoke("switch_profile", { name });
}

export function renameProfile(oldName: string, newName: string): Promise<void> {
    return invoke("rename_profile", { old: oldName, new: newName });
}

export function deleteProfile(name: string): Promise<void> {
    return invoke("delete_profile", { name });
}


export function checkBuild(): Promise<BuildPlan> {
    return invoke<BuildPlan>("check");
}

export function deployToGame(approvedNative: string[]): Promise<BuildPlan> {
    return invoke<BuildPlan>("deploy", { approvedNative });
}

export function launchGame(approvedNative: string[]): Promise<LaunchResult> {
    return invoke<LaunchResult>("launch", { approvedNative });
}

export function cleanGameModFolder(): Promise<number> {
    return invoke<number>("clean");
}


export function getGameConfig(): Promise<GameConfig> {
    return invoke<GameConfig>("get_config");
}

export function saveGameConfig(config: GameConfig): Promise<void> {
    return invoke("save_config", { config });
}

export function detectGamePaths(installDir: string | null): Promise<GamePaths> {
    return invoke<GamePaths>("detect_paths", { installDir });
}

export function isGameRunning(): Promise<boolean> {
    return invoke<boolean>("game_running");
}

export function openFolder(which: FolderTarget): Promise<void> {
    return invoke("open_folder", { which });
}

export function openUrl(url: string): Promise<void> {
    return invoke("open_url", { url });
}


export function installLoader(): Promise<string[]> {
    return invoke<string[]>("install_loader");
}

export function removeLoader(): Promise<string[]> {
    return invoke<string[]>("remove_loader");
}

export function openLoaderLog(): Promise<void> {
    return invoke("open_loader_log");
}

export function getNativeRequest(): Promise<NativeRequest> {
    return invoke<NativeRequest>("native_request");
}

export function rememberNativeApproval(ids: string[]): Promise<void> {
    return invoke("remember_native_approval", { ids });
}
