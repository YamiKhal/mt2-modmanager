import { getCurrentWebview } from "@tauri-apps/api/webview";
import * as commands from "../api/commands";
import { pickFolder, pickModFiles } from "../api/filePickers";
import type { Imported } from "../api/types";
import { refresh } from "../app/refresh";
import { appState } from "../app/state";
import { runAction } from "../ui/busy";
import { log, setLogVisible } from "../ui/log";
import { toast } from "../ui/toast";
import { byId } from "../util/dom";
import { baseName } from "../util/text";

interface AddCounts {
    added: number;
    updated: number;
    failed: number;
}


export async function addModsFromPaths(paths: string[]): Promise<void> {
    if (paths.length === 0) {
        return;
    }

    await runAction(
        "Add mod",
        async () => {
            const results = await commands.addMods(paths);
            const counts = logResults(results);

            toast(summary(counts), counts.failed > 0 && counts.added === 0 && counts.updated === 0);

            if (counts.failed > 0) {
                setLogVisible(true);
            }

            await refresh({ reloadIcons: counts.updated > 0 });
        },
        "Adding mods…",
    );
}

function logResults(results: Imported[]): AddCounts {
    const counts: AddCounts = { added: 0, updated: 0, failed: 0 };

    for (const result of results) {
        const source = baseName(result.source);

        if (result.error) {
            counts.failed += 1;
            log(`Could not add ${source}: ${result.error}`, "error");
        } else if (result.updated) {
            counts.updated += 1;
            log(`Updated ${result.id} from ${source}`, "ok");
        } else {
            counts.added += 1;
            log(`Added ${result.id} from ${source}`, "ok");
        }
    }

    return counts;
}

function summary(counts: AddCounts): string {
    const parts: string[] = [];

    if (counts.added > 0) {
        parts.push(`${counts.added} added`);
    }

    if (counts.updated > 0) {
        parts.push(`${counts.updated} updated`);
    }

    if (counts.failed > 0) {
        parts.push(`${counts.failed} failed (see Log)`);
    }

    return parts.join(", ") || "Nothing added";
}


export async function pickAndAddMods(): Promise<void> {
    const paths = await pickModFiles();

    if (paths) {
        await addModsFromPaths(paths);
    }
}

export async function pickAndAddFolder(): Promise<void> {
    const folder = await pickFolder("Add every mod in a folder");

    if (folder) {
        await addModsFromPaths([folder]);
    }
}


export function wireFileDrop(): void {
    const dropZone = byId("dropzone");

    void getCurrentWebview().onDragDropEvent((event) => {
        const drag = event.payload;

        if (drag.type === "enter") {
            dropZone.hidden = appState.gameLocked;
        } else if (drag.type === "leave") {
            dropZone.hidden = true;
        } else if (drag.type === "drop") {
            dropZone.hidden = true;

            if (!appState.gameLocked) {
                void addModsFromPaths(drag.paths);
            }
        }
    });
}
