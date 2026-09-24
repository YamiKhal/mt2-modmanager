import { appState } from "../app/state";
import { byId } from "../util/dom";
import { countOf, escapeHtml } from "../util/text";
import { eventHtml, sortBySeverity } from "./reportEvents";


export function renderConflicts(): void {
    const box = byId("tab-conflicts");
    const count = byId("conflictCount");

    if (appState.planError) {
        box.innerHTML = `<ul class="issues"><li class="issue error"><span class="lvl error">error</span>${escapeHtml(appState.planError)}</li></ul>`;
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
            ? `No conflicts between the ${countOf(modCount, "mod")} that are enabled.`
            : "No mods are enabled in this profile.";

        box.innerHTML = `<p class="empty">${message}</p>`;

        return;
    }

    const errorNote = errorCount > 0
        ? `<p class="issue error" style="margin:0 0 6px">These mods can't be applied until the errors below are fixed.</p>`
        : "";

    box.innerHTML = `${errorNote}
        <ul class="issues">${events.map(eventHtml).join("")}</ul>
        <p class="hint" style="margin-top:10px">A conflict means two mods change the same thing; the mod lower in the load order wins.</p>`;
}
