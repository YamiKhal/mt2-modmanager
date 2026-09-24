import { getModIcon } from "../api/commands";
import type { LibraryMod } from "../api/types";
import { queryAll } from "../util/dom";
import { escapeHtml, initials } from "../util/text";

const iconUrls = new Map<string, string | null>();


export function iconHtml(mod: LibraryMod, big = false): string {
    const id = mod.manifest?.id;
    const url = id ? iconUrls.get(id) : null;
    const className = "icon" + (big ? " big" : "");

    if (url) {
        return `<span class="${className}"><img src="${url}" alt=""></span>`;
    }

    // Filled in by loadIcons once the icon arrives.
    const iconId = mod.icon && id ? escapeHtml(id) : "";
    const letters = escapeHtml(initials(mod.manifest?.name ?? mod.folder));

    return `<span class="${className}" data-icon="${iconId}">${letters}</span>`;
}


export async function loadIcons(mods: LibraryMod[]): Promise<void> {
    const ids = mods
        .filter((mod) => mod.icon && mod.manifest && !iconUrls.has(mod.manifest.id))
        .map((mod) => mod.manifest!.id);

    for (const id of ids) {
        const url = await fetchIconUrl(id);

        iconUrls.set(id, url);

        if (!url) {
            continue;
        }

        for (const placeholder of queryAll(`[data-icon="${CSS.escape(id)}"]`)) {
            placeholder.innerHTML = `<img src="${url}" alt="">`;
        }
    }
}

async function fetchIconUrl(id: string): Promise<string | null> {
    try {
        const bytes = await getModIcon(id);

        return URL.createObjectURL(new Blob([bytes], { type: "image/png" }));
    } catch {
        return null;
    }
}

export function clearIcons(): void {
    for (const url of iconUrls.values()) {
        if (url) {
            URL.revokeObjectURL(url);
        }
    }

    iconUrls.clear();
}
