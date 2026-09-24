import { currentStatus } from "../app/state";
import { byId } from "../util/dom";
import { countOf, escapeHtml } from "../util/text";
import { switchToProfile } from "./profiles";


export function renderHeader(): void {
    renderProfilePicker();
    renderProblems();
    renderStatusLine();
    renderHandInstalledNotice();
}

function renderProfilePicker(): void {
    const status = currentStatus();

    byId("profileSel").innerHTML = status.profiles
        .map((name) => {
            const selected = name === status.active_profile ? "selected" : "";

            return `<option ${selected}>${escapeHtml(name)}</option>`;
        })
        .join("");
}

function renderProblems(): void {
    const problems = currentStatus().problems;
    const banner = byId("problems");

    banner.hidden = problems.length === 0;
    banner.innerHTML = `
        <span>${problems.map(escapeHtml).join("<br>")}</span>
        <span class="grow"></span>
        <button class="btn small" data-act="settings">Open settings</button>`;
}

function renderStatusLine(): void {
    const status = currentStatus();
    const enabledCount = status.mods.filter((mod) => mod.enabled).length;

    const game = status.game_version
        ? `MMORPG Tycoon 2 ${status.game_version}`
        : "Game version unknown (start the game once)";

    byId("gameLine").textContent =
        `${game} · ${countOf(enabledCount, "mod")} enabled in “${status.active_profile}”`;
}

function renderHandInstalledNotice(): void {
    const count = currentStatus().unmanaged.length;
    const notice = byId("handInstalled");

    notice.hidden = count === 0;
    notice.textContent =
        `${count} folder${count === 1 ? " was" : "s were"} installed into the game's mod folder by hand. Review…`;
}


export function wireHeader(): void {
    const picker = byId<HTMLSelectElement>("profileSel");

    picker.addEventListener("change", () => void switchToProfile(picker.value));
}
