import type { Level, ReportEvent } from "../api/types";

const LEVEL_ORDER: Record<Level, number> = {
    error: 0,
    conflict: 1,
    warning: 2,
    info: 3,
};


export function sortBySeverity(events: ReportEvent[]): ReportEvent[] {
    return [...events].sort((a, b) => LEVEL_ORDER[a.level] - LEVEL_ORDER[b.level]);
}
