import { byId, queryAll } from "../util/dom";

export type MainTab = "applied" | "desc" | "conflicts";

const MAIN_TABS: MainTab[] = ["applied", "desc", "conflicts"];
const BUILD_DETAILS_TABS = ["report", "files", "renames", "paths"] as const;


export function showTab(name: MainTab): void {
    for (const tab of queryAll('.window [role="tab"]')) {
        tab.setAttribute("aria-selected", String(tab.dataset.tab === name));
    }

    for (const tabName of MAIN_TABS) {
        byId("tab-" + tabName).hidden = tabName !== name;
    }
}

function showBuildDetailsTab(name: string): void {
    for (const tab of queryAll("#details [data-dtab]")) {
        tab.setAttribute("aria-selected", String(tab.dataset.dtab === name));
    }

    for (const tabName of BUILD_DETAILS_TABS) {
        byId("dtab-" + tabName).hidden = tabName !== name;
    }
}


export function wireTabs(): void {
    for (const tab of queryAll(".window [data-tab]")) {
        tab.addEventListener("click", () => showTab(tab.dataset.tab as MainTab));
    }

    for (const tab of queryAll("#details [data-dtab]")) {
        tab.addEventListener("click", () => showBuildDetailsTab(tab.dataset.dtab ?? "report"));
    }
}
