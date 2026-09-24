import type { Level, ReportEvent } from "../api/types";
import { modNameById } from "../app/state";
import { escapeHtml } from "../util/text";

const LEVEL_ORDER: Record<Level, number> = {
    error: 0,
    conflict: 1,
    warning: 2,
    info: 3,
};


export function sortBySeverity(events: ReportEvent[]): ReportEvent[] {
    return [...events].sort((a, b) => LEVEL_ORDER[a.level] - LEVEL_ORDER[b.level]);
}

export function eventHtml(event: ReportEvent): string {
    const where = [event.file, event.path].filter(Boolean).join(" · ");
    const who = event.mod_id ? `<span class="who">${escapeHtml(modNameById(event.mod_id))}</span> ` : "";
    const whereHtml = where ? `<span class="where mono">${escapeHtml(where)}</span>` : "";

    return `<li class="issue ${event.level}"><span class="lvl ${event.level}">${event.level}</span>
        ${who}${escapeHtml(event.message)}
        ${whereHtml}</li>`;
}
