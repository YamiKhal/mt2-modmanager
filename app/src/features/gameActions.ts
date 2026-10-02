import * as commands from "../api/commands";
import type { BuildPlan, Level } from "../api/types";
import { refresh } from "../app/refresh";
import { appState } from "../app/state";
import { confirmDialog } from "../ui/askDialog";
import { runAction } from "../ui/busy";
import { log } from "../ui/log";
import { toast } from "../ui/toast";
import { updateGameButtons } from "./gameButtons";


export async function launch(): Promise<void> {
    await whileApplying(() =>
        runAction(
            "Launch",
            async () => {
                const result = await commands.launchGame();

                if (result.applied) {
                    logApplied(result.plan);
                    toast("Mods applied. Starting MMORPG Tycoon 2 through Steam…");
                } else {
                    log("Mods already applied and no errors; starting the game", "ok");
                    toast("Starting MMORPG Tycoon 2 through Steam…");
                }
            },
            "Checking mods…",
        ),
    );
}

export async function applyToGame(): Promise<void> {
    await whileApplying(() =>
        runAction(
            "Apply",
            async () => {
                const plan = await commands.deployToGame();

                logApplied(plan);
                toast("Mods applied to the game. Restart the game if it's running.");
            },
            "Applying mods…",
        ),
    );
}

export async function removeModsFromGame(): Promise<void> {
    const confirmed = await confirmDialog({
        title: "Remove mods from game",
        text:
            "Remove every file the manager put into the game's mod folder?\n\n" +
            "Your mod library, profiles, saves and hand-installed mods are not touched.",
        okLabel: "Remove",
    });

    if (!confirmed) {
        return;
    }

    await runAction("Remove", async () => {
        const folderCount = await commands.cleanGameModFolder();

        log(`Removed ${folderCount} deployed folder(s) from the game's mod folder`);
        toast("Mods removed from the game.");
        await refresh();
    });
}


async function whileApplying(work: () => Promise<unknown>): Promise<void> {
    appState.applying = true;
    updateGameButtons();

    try {
        await work();
    } finally {
        appState.applying = false;
        await refresh();
    }
}

function logApplied(plan: BuildPlan): void {
    const countLevel = (level: Level): number =>
        plan.report.events.filter((event) => event.level === level).length;

    log(
        `Applied ${plan.mods.length} mod(s), ${plan.files.length} file(s): ` +
            `${countLevel("conflict")} conflict(s), ${countLevel("warning")} warning(s)`,
        "ok",
    );
}
