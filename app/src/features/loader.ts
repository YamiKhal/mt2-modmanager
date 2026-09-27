import * as commands from "../api/commands";
import type { LoaderOverview } from "../api/types";
import { refresh } from "../app/refresh";
import { appState, currentStatus } from "../app/state";
import { confirmDialog } from "../ui/askDialog";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";
import { toast } from "../ui/toast";
import { byId } from "../util/dom";

type LoaderButtonKind = "install" | "update" | "repair" | null;


function buttonKind(loader: LoaderOverview): LoaderButtonKind {
    if (!loader.available) {
        return null;
    }

    if (loader.state === "not_installed" || loader.state === "undone_by_steam") {
        return "install";
    }

    if (loader.state === "broken") {
        return "repair";
    }

    return loader.update_available ? "update" : null;
}

function buttonTitle(kind: LoaderButtonKind, loader: LoaderOverview): string {
    if (kind === "update") {
        return `Update the MT2 Loader from ${loader.version} to ${loader.available}`;
    }

    if (kind === "repair") {
        return loader.problems.join("\n");
    }

    return "Install the MT2 Loader into the game folder. It runs the native plugins of mods that have them";
}

const BUTTON_LABELS: Record<Exclude<LoaderButtonKind, null>, string> = {
    install: "Install loader",
    update: "Update loader",
    repair: "Repair loader",
};


export function renderLoaderControls(): void {
    const loader = appState.status?.loader ?? null;
    const button = byId<HTMLButtonElement>("btnLoader");
    const kind = loader ? buttonKind(loader) : null;
    const installed = loader !== null && ["enabled", "disabled", "broken"].includes(loader.state);

    button.hidden = kind === null;

    if (loader && kind) {
        button.textContent = BUTTON_LABELS[kind];
        button.title = buttonTitle(kind, loader);
    }

    byId("miRemoveLoader").hidden = !installed && loader?.state !== "undone_by_steam";
    byId("miLoaderLog").hidden = !loader?.log;
}


function logActions(actions: string[]): void {
    for (const action of actions) {
        log(action, "ok");
    }
}

export async function installOrUpdateLoader(): Promise<void> {
    const loader = currentStatus().loader;

    if (!loader) {
        toast("The game folder wasn't found. Set it in Tools ▸ Settings.", true);

        return;
    }

    const isFirstInstall = loader.state === "not_installed" || loader.state === "undone_by_steam";

    if (isFirstInstall) {
        const confirmed = await confirmDialog({
            title: "Install MT2 Loader",
            text: "This replaces zlib1.dll in the game folder with the loader's, and keeps a copy of the original.\n"
                + "Undo it any time with Tools ▸ Remove MT2 Loader, or Steam's \"Verify integrity of game files\".",
            okLabel: "Install",
        });

        if (!confirmed) {
            return;
        }
    }

    await runAction(isFirstInstall ? "Install loader" : "Update loader", async () => {
        logActions(await commands.installLoader());
        toast(`MT2 Loader ${loader.available} installed.`);
        await refresh();
    });
}

export async function removeLoader(): Promise<void> {
    const confirmed = await confirmDialog({
        title: "Remove MT2 Loader",
        text: "The game's own zlib1.dll is put back.\nMods with plugins still work otherwise, but their plugins won't run.",
        okLabel: "Remove",
    });

    if (!confirmed) {
        return;
    }

    await runAction("Remove loader", async () => {
        logActions(await commands.removeLoader());
        toast("MT2 Loader removed.");
        await refresh();
    });
}

export function openLoaderLog(): Promise<unknown> {
    return runAction("Open", () => commands.openLoaderLog());
}
