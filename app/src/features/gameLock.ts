import { isGameRunning } from "../api/commands";
import { refresh } from "../app/refresh";
import { appState } from "../app/state";
import { reportError } from "../ui/busy";
import { log } from "../ui/log";
import { clearIcons } from "../ui/modIcons";
import { byId, queryAll } from "../util/dom";

const POLL_INTERVAL_MS = 3000;

let ignoredThisRun = false;


export function wireGameLock(): void {
    const dialog = byId<HTMLDialogElement>("gameRunning");

    // Escape would close the dialog without unlocking anything.
    dialog.addEventListener("cancel", (event) => event.preventDefault());

    byId("btnIgnoreGame").addEventListener("click", () => {
        ignoredThisRun = true;
        log("Game is running; lock ignored");
        unlock();
    });

    void checkGame();
    window.setInterval(() => void checkGame(), POLL_INTERVAL_MS);
}


async function checkGame(): Promise<void> {
    let running: boolean;

    try {
        running = await isGameRunning();
    } catch {
        // Can't tell: leave the page as it is.
        return;
    }

    if (!running) {
        ignoredThisRun = false;

        if (appState.gameLocked) {
            log("Game closed; manager unlocked");
            unlock();
        }

        return;
    }

    if (!appState.gameLocked && !ignoredThisRun) {
        lock();
    }
}


function lock(): void {
    appState.gameLocked = true;
    releaseMemory();
    log("Game is running; manager locked");
    byId<HTMLDialogElement>("gameRunning").showModal();
}

function unlock(): void {
    appState.gameLocked = false;
    byId<HTMLDialogElement>("gameRunning").close();
    refresh({ reloadIcons: true }).catch((error: unknown) => reportError(String(error)));
}

function releaseMemory(): void {
    appState.plan = null;
    clearIcons();

    const buildDetails = byId<HTMLDialogElement>("details");

    buildDetails.close();

    for (const tab of queryAll('[id^="dtab-"]', buildDetails)) {
        tab.innerHTML = "";
    }
}
