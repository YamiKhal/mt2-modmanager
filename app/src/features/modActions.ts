import * as commands from "../api/commands";
import { refresh } from "../app/refresh";
import {
    appState,
    appliedModIds,
    currentStatus,
    modDisplayName,
    modNameById,
    selectedAvailableMods,
} from "../app/state";
import { confirmDialog } from "../ui/askDialog";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";


export async function applyMods(ids: string[]): Promise<void> {
    await runAction("Apply", async () => {
        await commands.applyModsToProfile(ids);
        log(`Applied to “${currentStatus().active_profile}”: ${ids.join(", ")}`);
        await refresh();
    });
}

export async function applySelectedMods(): Promise<void> {
    const ids = selectedAvailableMods()
        .filter((mod) => !mod.applied)
        .flatMap((mod) => (mod.manifest ? [mod.manifest.id] : []));

    if (ids.length > 0) {
        await applyMods(ids);
    }
}


export async function removeSelectedMods(): Promise<void> {
    const mods = selectedAvailableMods();

    if (mods.length === 0) {
        return;
    }

    const names = mods.map(modDisplayName);
    const what = names.length === 1 ? `“${names[0]}”` : `${names.length} mods`;

    const confirmed = await confirmDialog({
        title: "Remove mods",
        text:
            `Delete ${what} from the library and from every profile?\n\n` +
            "This deletes the manager's copy. The game only changes after the next Launch or Apply.",
        okLabel: "Remove",
    });

    if (!confirmed) {
        return;
    }

    await runAction("Remove", async () => {
        for (const mod of mods) {
            await commands.removeModFromLibrary(mod.manifest?.id ?? mod.folder);
            log(`Removed ${modDisplayName(mod)} from the library`);
        }

        appState.availableSelection.clear();
        await refresh();
    });
}


export async function setModEnabled(id: string, name: string, enabled: boolean): Promise<void> {
    await runAction("Switch", async () => {
        await commands.setModEnabled(id, enabled);
        log(`${enabled ? "Enabled" : "Disabled"}: ${name}`);
        await refresh();
    });
}


export async function unapplySelectedMod(): Promise<void> {
    const id = appState.appliedSelection;

    if (!id) {
        return;
    }

    await runAction("Unapply", async () => {
        await commands.removeModsFromProfile([id]);
        log(`Took ${modNameById(id)} out of “${currentStatus().active_profile}”`);
        await refresh();
    });
}


export async function saveLoadOrder(order: string[]): Promise<void> {
    await runAction("Reorder", async () => {
        await commands.setLoadOrder(order);
        log(`Load order: ${order.join(" → ")}`);
        await refresh();
    });
}

export function moveSelectedMod(offset: -1 | 1): void {
    const order = appliedModIds();
    const from = order.indexOf(appState.appliedSelection ?? "");
    const to = from + offset;

    if (from < 0 || to < 0 || to >= order.length) {
        return;
    }

    [order[from], order[to]] = [order[to], order[from]];
    void saveLoadOrder(order);
}
