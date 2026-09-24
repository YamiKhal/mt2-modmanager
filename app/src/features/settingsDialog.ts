import * as commands from "../api/commands";
import { pickFolder, pickGameDataZip } from "../api/filePickers";
import type { GameConfig, GamePaths } from "../api/types";
import { refresh } from "../app/refresh";
import { appState } from "../app/state";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";
import { byId, queryAll } from "../util/dom";

type PathField = "cfgInstall" | "cfgData" | "cfgProfile";


function field(id: PathField): HTMLInputElement {
    return byId<HTMLInputElement>(id);
}

function fieldValue(id: PathField): string | null {
    return field(id).value.trim() || null;
}


export async function openSettingsDialog(): Promise<void> {
    const config = await commands.getGameConfig();
    const inUse = appState.status?.paths;

    field("cfgInstall").value = config.install_dir ?? inUse?.install_dir ?? "";
    field("cfgData").value = config.data ?? inUse?.data_zip ?? "";
    field("cfgProfile").value = config.profile_dir ?? inUse?.profile_dir ?? "";

    byId<HTMLDialogElement>("settings").showModal();
}


async function detectInto(id: PathField): Promise<void> {
    await runAction("Detect", async () => {
        const detected = await commands.detectGamePaths(fieldValue("cfgInstall"));
        const path = detectedFor(id, detected);

        field(id).value = path ?? "";
        log(path ? `Detected ${path}` : "Detect: not found", path ? "ok" : "warn");
    });
}

function detectedFor(id: PathField, detected: GamePaths): string | null {
    switch (id) {
        case "cfgInstall":
            return detected.install_dir;

        case "cfgData":
            return detected.data_zip;

        case "cfgProfile":
            return detected.profile_dir;
    }
}


async function saveSettings(): Promise<void> {
    await runAction("Settings", async () => {
        const detected = await commands.detectGamePaths(fieldValue("cfgInstall"));

        // Only paths that differ from detection are saved; the rest stay "detect".
        const config: GameConfig = {
            install_dir: unlessDetected(fieldValue("cfgInstall"), detected.install_dir),
            data: unlessDetected(fieldValue("cfgData"), detected.data_zip),
            profile_dir: unlessDetected(fieldValue("cfgProfile"), detected.profile_dir),
        };

        await commands.saveGameConfig(config);
        log("Settings saved");
        await refresh({ reloadIcons: true });
    });
}

function unlessDetected(value: string | null, detected: string | null): string | null {
    return value === detected ? null : value;
}


export function wireSettingsDialog(): void {
    const dialog = byId<HTMLDialogElement>("settings");

    dialog.addEventListener("close", () => {
        if (dialog.returnValue === "save") {
            void saveSettings();
        }
    });

    for (const button of queryAll("[data-detect]", dialog)) {
        button.addEventListener("click", () => void detectInto(button.dataset.detect as PathField));
    }

    for (const button of queryAll("[data-browse]", dialog)) {
        button.addEventListener("click", async () => {
            const picked = button.dataset.kind === "zip" ? await pickGameDataZip() : await pickFolder();

            if (picked) {
                field(button.dataset.browse as PathField).value = picked;
            }
        });
    }
}
