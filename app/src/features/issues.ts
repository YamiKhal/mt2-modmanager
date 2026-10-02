import type { Level, ReportEvent } from "../api/types";
import { appState, modNameById } from "../app/state";
import { byId } from "../util/dom";
import { countOf, escapeHtml } from "../util/text";
import { sortBySeverity } from "./reportEvents";

const LEVEL_NAMES: Record<Level, string> = {
    error: "Error",
    conflict: "Conflict",
    warning: "Warning",
    info: "Info",
};


export function renderIssues(): void {
    const box = byId("tab-issues");
    const count = byId("issueCount");

    if (appState.planError) {
        box.innerHTML = tableHtml([rowHtml("error", "", appState.planError, "")]);
        count.hidden = false;
        count.className = "count err";
        count.textContent = "!";

        return;
    }

    const plan = appState.plan;

    if (!plan) {
        return;
    }

    const events = sortBySeverity(plan.report.events.filter((event) => event.level !== "info"));
    const errorCount = events.filter((event) => event.level === "error").length;

    count.hidden = events.length === 0;
    count.className = "count" + (errorCount > 0 ? " err" : "");
    count.textContent = String(events.length);

    if (events.length === 0) {
        const modCount = plan.mods.length;
        const message = modCount > 0
            ? `No issues in the ${countOf(modCount, "mod")} that are enabled.`
            : "No mods are enabled in this profile.";

        box.innerHTML = `<p class="empty">${message}</p>`;

        return;
    }

    const errorNote = errorCount > 0
        ? `<p class="issuenote">These mods can't be applied until the errors are fixed.</p>`
        : "";

    box.innerHTML = `${errorNote}
        ${tableHtml(events.map(eventRowHtml))}
        <p class="hint issuehint">A conflict means two mods change the same thing; the mod lower in the load order wins.</p>`;
}

function tableHtml(rows: string[]): string {
    return `
        <table class="issuelist">
            <thead><tr><th></th><th>Mod</th><th>Issue</th></tr></thead>
            <tbody>${rows.join("")}</tbody>
        </table>`;
}

function eventRowHtml(event: ReportEvent): string {
    const where = [event.file, event.path].filter(Boolean).join(" · ");
    const mod = event.mod_id ? modNameById(event.mod_id) : "";

    return rowHtml(event.level, mod, event.message, where);
}

function rowHtml(level: Level, mod: string, message: string, where: string): string {
    const whereHtml = where ? `<span class="where mono">${escapeHtml(where)}</span>` : "";

    return `
        <tr>
            <td class="dotcell"><span class="dot ${level}" title="${LEVEL_NAMES[level]}" aria-label="${LEVEL_NAMES[level]}"></span></td>
            <td class="who">${escapeHtml(mod)}</td>
            <td>${escapeHtml(message)}${whereHtml}</td>
        </tr>`;
}
