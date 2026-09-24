import { byId } from "../util/dom";
import { log } from "./log";
import { toast } from "./toast";

let runningCount = 0;


export function startBusy(text: string): void {
    runningCount += 1;
    byId("busyText").textContent = text;
    updateBusyIndicator();
}

export function endBusy(): void {
    runningCount = Math.max(0, runningCount - 1);
    updateBusyIndicator();
}

function updateBusyIndicator(): void {
    const busy = runningCount > 0;

    byId("busyLine").hidden = !busy;
    document.body.style.cursor = busy ? "progress" : "";
}


export function reportError(message: string): void {
    toast(message, true);
    log(message, "error");
}

export async function runAction<T>(
    label: string,
    action: () => Promise<T>,
    busyText = label + "…",
): Promise<T | undefined> {
    startBusy(busyText);

    try {
        return await action();
    } catch (error) {
        reportError(`${label}: ${String(error)}`);

        return undefined;
    } finally {
        endBusy();
    }
}
