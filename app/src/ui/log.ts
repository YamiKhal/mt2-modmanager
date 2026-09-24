import { byId } from "../util/dom";
import { STORAGE_KEYS, writeSetting } from "../util/storage";
import { escapeHtml } from "../util/text";

export type LogKind = "" | "ok" | "warn" | "error";

const MAX_LINES = 500;


export function log(message: string, kind: LogKind = ""): void {
    const list = byId<HTMLOListElement>("log");
    const line = document.createElement("li");
    const time = new Date().toLocaleTimeString();

    line.innerHTML = `<span class="t">${time}</span><span class="${kind}">${escapeHtml(message)}</span>`;
    list.append(line);

    while (list.children.length > MAX_LINES) {
        list.firstElementChild?.remove();
    }

    list.scrollTop = list.scrollHeight;
}

export function clearLog(): void {
    byId("log").innerHTML = "";
}


export function isLogVisible(): boolean {
    return !byId("logPane").hidden;
}

export function setLogVisible(visible: boolean): void {
    byId("logPane").hidden = !visible;
    byId("miLog").classList.toggle("checked", visible);
    writeSetting(STORAGE_KEYS.logVisible, visible ? "1" : "0");
}
