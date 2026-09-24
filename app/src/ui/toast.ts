import { byId } from "../util/dom";

const SHOW_MS = 3500;
const SHOW_ERROR_MS = 7000;

let hideTimer: number | undefined;


export function toast(message: string, isError = false): void {
    const element = byId("toast");

    element.textContent = message;
    element.className = "toast" + (isError ? " bad" : "");
    element.hidden = false;

    window.clearTimeout(hideTimer);
    hideTimer = window.setTimeout(() => {
        element.hidden = true;
    }, isError ? SHOW_ERROR_MS : SHOW_MS);
}
