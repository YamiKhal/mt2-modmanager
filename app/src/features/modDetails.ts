import * as commands from "../api/commands";
import type { LibraryMod, Manifest } from "../api/types";
import { appState, findModByKey } from "../app/state";
import { runAction } from "../ui/busy";
import { iconHtml } from "../ui/modIcons";
import { byId, query, queryAll } from "../util/dom";
import { escapeHtml, isWebLink } from "../util/text";
import { applyMods } from "./modActions";
import { settingsSectionHtml, wireSettings } from "./modSettings";


export function renderModDetails(): void {
    const box = byId("tab-desc");
    const mod = appState.detailsModKey ? findModByKey(appState.detailsModKey) : undefined;

    box.classList.add("desc");

    if (!mod) {
        box.innerHTML = `<p class="empty" style="grid-column:1/-1">Select a mod to see its details.</p>`;

        return;
    }

    if (!mod.manifest) {
        box.innerHTML = brokenModHtml(mod);

        return;
    }

    box.innerHTML = `
        ${iconHtml(mod, true)}
        ${headHtml(mod, mod.manifest)}
        ${mod.manifest.description ? `<p class="text">${escapeHtml(mod.manifest.description)}</p>` : ""}
        ${factsHtml(mod.manifest)}
        ${settingsSectionHtml(mod)}`;

    wireButtons(box, mod.manifest);
    wireSettings(box, mod);
}

function brokenModHtml(mod: LibraryMod): string {
    return `
        ${iconHtml(mod, true)}
        <div><h3>${escapeHtml(mod.folder)}</h3><p class="by">Can't be loaded</p></div>
        <p class="text">${escapeHtml(mod.error)}</p>`;
}


function headHtml(mod: LibraryMod, manifest: Manifest): string {
    const author = manifest.author.trim();

    // Without an author the line stays, empty, so the layout doesn't shift.
    const byLine = author ? `by ${escapeHtml(author)}` : "&nbsp;";

    const addButton = mod.applied
        ? ""
        : `<button class="btn small" data-desc="apply">Add to profile</button>`;

    const modPageButton = isWebLink(manifest.mod_page)
        ? `<button class="btn small" data-url="${escapeHtml(manifest.mod_page.trim())}">Mod page</button>`
        : "";

    return `
        <div class="head">
            <h3>${escapeHtml(manifest.name)} <span class="ver">${escapeHtml(manifest.version)}</span></h3>
            <p class="by">${byLine}</p>
            <p class="tags">${statusTagsHtml(mod, manifest)}</p>
            <div class="btnrow">
                ${addButton}
                ${modPageButton}
                <button class="btn small" data-desc="open">Open folder</button>
            </div>
        </div>`;
}

function statusTagsHtml(mod: LibraryMod, manifest: Manifest): string {
    const tags: string[] = [];
    const inGame = appState.status?.deployed_mods.includes(manifest.id) ?? false;

    if (inGame) {
        tags.push(`<span class="tag state live" title="This version is in the game's mod folder">Applied</span>`);
    }

    if (!mod.applied) {
        tags.push(`<span class="tag state disabled">Not added</span>`);
    } else if (mod.enabled) {
        tags.push(`<span class="tag state">Enabled</span>`);
    } else {
        tags.push(`<span class="tag state disabled">Disabled</span>`);
    }

    for (const tag of manifest.tags ?? []) {
        tags.push(`<span class="tag">${escapeHtml(tag)}</span>`);
    }

    return tags.join("");
}


function factsHtml(manifest: Manifest): string {
    return `
        <dl class="facts">
            <dt>Mod id</dt><dd class="mono">${escapeHtml(manifest.id)}</dd>
            ${textFact("Author", manifest.author)}
            ${listFact("Contributors", manifest.contributors)}
            ${textFact("Mod page", manifest.mod_page)}
            ${textFact("Support", manifest.support)}
            ${textFact("Donate", manifest.donate)}
            ${textFact("Source", manifest.source)}
            ${textFact("License", manifest.license)}
            ${listFact("Requires", manifest.dependencies)}
            ${listFact("Incompatible with", manifest.incompatible)}
            ${listFact("Made for game", manifest.game_versions)}
        </dl>`;
}

function textFact(label: string, value: string | undefined): string {
    if (!value) {
        return "";
    }

    return `<dt>${label}</dt><dd>${linkOrText(value)}</dd>`;
}

function listFact(label: string, values: string[] | undefined): string {
    if (!values?.length) {
        return "";
    }

    return `<dt>${label}</dt><dd>${values.map(escapeHtml).join(", ")}</dd>`;
}

function linkOrText(value: string): string {
    if (!isWebLink(value)) {
        return escapeHtml(value);
    }

    const url = value.trim();
    const shown = url.replace(/^mailto:/i, "");

    return `<a href="#" data-url="${escapeHtml(url)}">${escapeHtml(shown)}</a>`;
}


function wireButtons(box: HTMLElement, manifest: Manifest): void {
    query('[data-desc="apply"]', box)?.addEventListener("click", () => {
        void applyMods([manifest.id]);
    });

    for (const link of queryAll("[data-url]", box)) {
        link.addEventListener("click", (event) => {
            event.preventDefault();
            void runAction("Open link", () => commands.openUrl(link.dataset.url ?? ""));
        });
    }

    query('[data-desc="open"]', box)?.addEventListener("click", () => {
        void runAction("Open", () => commands.openFolder(`mod:${manifest.id}`));
    });
}
