import * as commands from "../api/commands";
import { refresh } from "../app/refresh";
import { appState, currentStatus } from "../app/state";
import { confirmDialog, promptDialog } from "../ui/askDialog";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";
import { toast } from "../ui/toast";


export async function newProfile(copyActive: boolean): Promise<void> {
    const active = currentStatus().active_profile;

    const name = await promptDialog({
        title: copyActive ? "Duplicate profile" : "New profile",
        text: copyActive ? `Name for the copy of “${active}”:` : "Name for the new, empty profile:",
        initialValue: copyActive ? active + " copy" : "",
        okLabel: "Create",
    });

    if (!name) {
        return;
    }

    await runAction("New profile", async () => {
        await commands.createProfile(name, copyActive);
        log(`Created profile “${name}”`);
        await refresh();
    });
}

export async function renameActiveProfile(): Promise<void> {
    const active = currentStatus().active_profile;

    const name = await promptDialog({
        title: "Rename profile",
        text: "New name:",
        initialValue: active,
        okLabel: "Rename",
    });

    if (!name || name === active) {
        return;
    }

    await runAction("Rename profile", async () => {
        await commands.renameProfile(active, name);
        log(`Renamed profile to “${name}”`);
        await refresh();
    });
}

export async function deleteActiveProfile(): Promise<void> {
    const status = currentStatus();
    const active = status.active_profile;

    if (status.profiles.length <= 1) {
        toast("The last profile can't be deleted.", true);

        return;
    }

    const confirmed = await confirmDialog({
        title: "Delete profile",
        text: `Delete the profile “${active}”?\nIts mods stay in Available Mods.`,
        okLabel: "Delete",
    });

    if (!confirmed) {
        return;
    }

    await runAction("Delete profile", async () => {
        await commands.deleteProfile(active);
        log(`Deleted profile “${active}”`);
        await refresh();
    });
}

export async function switchToProfile(name: string): Promise<void> {
    await runAction("Switch profile", async () => {
        await commands.switchProfile(name);
        log(`Switched to profile “${name}”`);
        appState.appliedSelection = null;
        await refresh();
    });
}
