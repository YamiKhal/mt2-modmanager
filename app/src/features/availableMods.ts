import type { LibraryMod, Manifest } from "../api/types";
import { gameVersionFit, gameVersionTooltip } from "../app/gameVersion";
import { appState, currentStatus, modDisplayName, modKey, selectedAvailableMods } from "../app/state";
import { iconHtml } from "../ui/modIcons";
import { showTab } from "../ui/tabs";
import { byId } from "../util/dom";
import { escapeHtml } from "../util/text";
import { applyMods } from "./modActions";
import { renderModDetails } from "./modDetails";

const WARNING_ICON = `<path fill-rule="evenodd" d="M8 1.2 15.4 14.4H.6zM7.2 5.6v4.6h1.6V5.6zm0 5.8v1.6h1.6v-1.6z"/>`;


export function renderAvailableMods(): void {
    const list = byId<HTMLUListElement>("availList");
    const allMods = currentStatus().mods;
    const shownMods = sortByName(allMods).filter(matchesFilter);

    list.innerHTML = "";
    byId("availEmpty").hidden = allMods.length > 0;

    for (const mod of shownMods) {
        list.append(createRow(mod, shownMods));
    }

    updateButtons();
}

function sortByName(mods: LibraryMod[]): LibraryMod[] {
    return [...mods].sort((a, b) =>
        modDisplayName(a).localeCompare(modDisplayName(b), undefined, { sensitivity: "base" }),
    );
}

function matchesFilter(mod: LibraryMod): boolean {
    const filter = byId<HTMLInputElement>("filter").value.trim().toLowerCase();

    if (!filter) {
        return true;
    }

    const manifest = mod.manifest;
    const searchable = [
        manifest?.name,
        manifest?.author,
        manifest?.id,
        mod.folder,
        ...(manifest?.tags ?? []),
    ];

    return searchable.some((text) => (text ?? "").toLowerCase().includes(filter));
}

function createRow(mod: LibraryMod, shownMods: LibraryMod[]): HTMLLIElement {
    const key = modKey(mod);
    const selected = appState.availableSelection.has(key);
    const manifest = mod.manifest;
    const row = document.createElement("li");

    row.className = "row";
    row.classList.toggle("applied", mod.applied);
    row.classList.toggle("broken", !manifest);
    row.classList.toggle("sel", selected);
    row.setAttribute("role", "option");
    row.setAttribute("aria-selected", String(selected));

    if (manifest) {
        const byLine =  "by " + (manifest.author ? escapeHtml(manifest.author) : escapeHtml(manifest.id));

        row.title = `${manifest.name} (${manifest.id})`;
        row.innerHTML = `
            ${iconHtml(mod)}
            <div class="rtitle"><b><em>${escapeHtml(manifest.name)}</em>${versionMarkHtml(manifest)}</b><span>${byLine}</span></div>
            <span class="rver">${escapeHtml(manifest.version)}</span>`;
    } else {
        row.title = mod.error ?? "";
        row.innerHTML = `
            ${iconHtml(mod)}
            <div class="rtitle">
                <b>${escapeHtml(mod.folder)}<span class="tag bad">broken</span></b><span>${escapeHtml(mod.error)}</span>
            </div>
            <span class="rver"></span>`;
    }

    row.addEventListener("click", (event) => selectRow(key, event, shownMods));
    row.addEventListener("dblclick", () => {
        if (manifest && !mod.applied) {
            void applyMods([manifest.id]);
        } else {
            showTab("desc");
        }
    });

    return row;
}

function versionMarkHtml(manifest: Manifest): string {
    const gameVersion = currentStatus().game_version;
    const fit = gameVersionFit(manifest, gameVersion);

    if (fit === "fits") {
        return "";
    }

    const tooltip = escapeHtml(gameVersionTooltip(manifest, gameVersion));

    return `<svg class="vmark ${fit}" viewBox="0 0 16 16" role="img" aria-label="${tooltip}"><title>${tooltip}</title>${WARNING_ICON}</svg>`;
}


function selectRow(key: string, event: MouseEvent, shownMods: LibraryMod[]): void {
    const selection = appState.availableSelection;

    if (event.ctrlKey || event.metaKey) {
        if (selection.has(key)) {
            selection.delete(key);
        } else {
            selection.add(key);
        }
    } else if (event.shiftKey && rangeAnchorIndex(shownMods) >= 0) {
        const keys = shownMods.map(modKey);
        const [start, end] = [rangeAnchorIndex(shownMods), keys.indexOf(key)].sort((a, b) => a - b);

        for (const rangeKey of keys.slice(start, end + 1)) {
            selection.add(rangeKey);
        }
    } else {
        appState.availableSelection = new Set([key]);
    }

    appState.detailsModKey = key;
    renderAvailableMods();
    renderModDetails();
}

function rangeAnchorIndex(shownMods: LibraryMod[]): number {
    const lastSelected = [...appState.availableSelection].at(-1);

    if (lastSelected === undefined) {
        return -1;
    }

    return shownMods.findIndex((mod) => modKey(mod) === lastSelected);
}


function updateButtons(): void {
    const selected = selectedAvailableMods();

    byId<HTMLButtonElement>("btnRemove").disabled = selected.length === 0;
    byId<HTMLButtonElement>("btnApply").disabled = !selected.some(
        (mod) => mod.manifest && !mod.applied,
    );
}


export function wireAvailableMods(): void {
    byId("filter").addEventListener("input", renderAvailableMods);
}
