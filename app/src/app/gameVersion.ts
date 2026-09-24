import type { Manifest } from "../api/types";

export type VersionFit = "fits" | "outdated" | "unknown";


export function gameVersionFit(manifest: Manifest, gameVersion: string | null): VersionFit {
    const targets = manifest.game_versions;

    if (targets.length === 0) {
        return "unknown";
    }

    if (!gameVersion) {
        return "fits";
    }

    return targets.some((target) => gameVersion.startsWith(target)) ? "fits" : "outdated";
}

export function gameVersionTooltip(manifest: Manifest, gameVersion: string | null): string {
    const installed = gameVersion ? ` You have ${gameVersion}.` : "";

    if (manifest.game_versions.length === 0) {
        return `Doesn't say which game version it was made for.${installed}`;
    }

    return `Made for game ${manifest.game_versions.join(", ")}.${installed}`;
}
