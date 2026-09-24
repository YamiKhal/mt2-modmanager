import type { BuildPlan, LibraryMod, Status } from "../api/types";

interface AppState {
    status: Status | null;
    plan: BuildPlan | null;
    planError: string | null;
    availableSelection: Set<string>;
    appliedSelection: string | null;
    detailsModKey: string | null;
    applying: boolean;
    gameLocked: boolean;
}

export const appState: AppState = {
    status: null,
    plan: null,
    planError: null,
    availableSelection: new Set(),
    appliedSelection: null,
    detailsModKey: null,
    applying: false,
    gameLocked: false,
};


export function currentStatus(): Status {
    if (!appState.status) {
        throw new Error("the mod list hasn't been loaded yet");
    }

    return appState.status;
}


export function modKey(mod: LibraryMod): string {
    return mod.manifest?.id ?? "folder:" + mod.folder;
}

export function findModByKey(key: string): LibraryMod | undefined {
    return appState.status?.mods.find((mod) => modKey(mod) === key);
}

export function modDisplayName(mod: LibraryMod): string {
    return mod.manifest?.name ?? mod.folder;
}

export function modNameById(id: string): string {
    const mod = appState.status?.mods.find((candidate) => candidate.manifest?.id === id);

    return mod?.manifest?.name ?? id;
}


export function appliedMods(): LibraryMod[] {
    return currentStatus().mods.filter((mod) => mod.applied);
}

export function appliedModIds(): string[] {
    return appliedMods().flatMap((mod) => (mod.manifest ? [mod.manifest.id] : []));
}

export function selectedAvailableMods(): LibraryMod[] {
    return [...appState.availableSelection]
        .map(findModByKey)
        .filter((mod): mod is LibraryMod => mod !== undefined);
}


export function pruneSelections(status: Status): void {
    const keys = new Set(status.mods.map(modKey));

    appState.availableSelection = new Set(
        [...appState.availableSelection].filter((key) => keys.has(key)),
    );

    const appliedStillThere = status.mods.some(
        (mod) => mod.applied && modKey(mod) === appState.appliedSelection,
    );

    if (appState.appliedSelection && !appliedStillThere) {
        appState.appliedSelection = null;
    }

    if (appState.detailsModKey && !keys.has(appState.detailsModKey)) {
        appState.detailsModKey = null;
    }
}
