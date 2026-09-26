import * as commands from "../api/commands";
import { pickFolder } from "../api/filePickers";
import type { Imported } from "../api/types";
import { refresh } from "../app/refresh";
import { runAction } from "../ui/busy";
import { log, setLogVisible } from "../ui/log";
import { toast } from "../ui/toast";


export async function pickAndAddDevMod(): Promise<void> {
    const folder = await pickFolder("Add dev mod: pick the folder of the mod you're making");

    if (!folder) {
        return;
    }

    await runAction(
        "Add dev mod",
        async () => {
            const result = await commands.addDevMod(folder);

            log(`Added dev mod ${result.id} from ${folder}`, "ok");
            toast(`${result.id} added as a dev mod`);
            await refresh({ reloadIcons: true });
        },
        "Adding dev mod…",
    );
}

export async function refreshDevMods(ids: string[] = []): Promise<void> {
    await runAction(
        "Refresh dev mods",
        async () => {
            const results = await commands.refreshDevMods(ids);
            const failed = results.filter((result) => result.error);

            logResults(results);
            toast(summary(results.length, failed.length), failed.length > 0);

            if (failed.length > 0) {
                setLogVisible(true);
            }

            await refresh({ reloadIcons: true });
        },
        "Refreshing dev mods…",
    );
}

function logResults(results: Imported[]): void {
    for (const result of results) {
        if (result.error) {
            log(`Could not refresh ${result.id}: ${result.error}`, "error");
        } else {
            log(`Refreshed ${result.id} from ${result.source}`, "ok");
        }
    }
}

function summary(total: number, failed: number): string {
    if (total === 0) {
        return "No dev mods to refresh";
    }

    if (failed > 0) {
        return `${failed} of ${total} dev mods failed to refresh (see Log)`;
    }

    return total === 1 ? "Dev mod refreshed" : `${total} dev mods refreshed`;
}

export async function stopDevMod(id: string): Promise<void> {
    await runAction("Stop dev mod", async () => {
        await commands.stopDevMod(id);
        await refresh({ reloadIcons: false });
    });
}
