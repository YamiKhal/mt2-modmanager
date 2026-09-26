import * as commands from "../api/commands";
import { refresh } from "../app/refresh";
import { appState } from "../app/state";
import { runAction } from "../ui/busy";
import { clearLog, isLogVisible, setLogVisible } from "../ui/log";
import { closeMenus } from "../ui/menus";
import { openAbout } from "./about";
import { pickAndAddFolder, pickAndAddMods } from "./addMods";
import { openBuildDetails } from "./buildDetails";
import { copyGameVersion } from "./copyGameVersion";
import { pickAndAddDevMod, refreshDevMods } from "./devMods";
import { applyToGame, launch, removeModsFromGame } from "./gameActions";
import {
    applySelectedMods,
    moveSelectedMod,
    removeSelectedMods,
    unapplySelectedMod,
} from "./modActions";
import { deleteActiveProfile, newProfile, renameActiveProfile } from "./profiles";
import { openSettingsDialog } from "./settingsDialog";

const ACTIONS: Record<string, () => unknown> = {
    addMod: pickAndAddMods,
    addFolder: pickAndAddFolder,
    refresh: refreshEverything,
    openMod: () => runAction("Open", () => commands.openFolder("mod")),
    openLibrary: () => runAction("Open", () => commands.openFolder("library")),

    newProfile: () => newProfile(false),
    dupProfile: () => newProfile(true),
    renameProfile: renameActiveProfile,
    deleteProfile: deleteActiveProfile,

    deploy: applyToGame,
    launch,
    clean: removeModsFromGame,
    addDevMod: pickAndAddDevMod,
    refreshDevMods: () => refreshDevMods(),
    details: openBuildDetails,
    copyGameVersion,
    settings: openSettingsDialog,

    toggleLog: () => setLogVisible(!isLogVisible()),
    clearLog,
    about: openAbout,

    removeMods: removeSelectedMods,
    applyMods: applySelectedMods,

    up: () => moveSelectedMod(-1),
    down: () => moveSelectedMod(1),
    unapply: unapplySelectedMod,
};


function refreshEverything(): Promise<unknown> {
    return runAction("Refresh", () => refresh({ reloadIcons: true }));
}


export function wireActions(): void {
    document.addEventListener("click", (event) => {
        const target = event.target as Element;
        const actionElement = target.closest<HTMLButtonElement>("[data-act]");
        const clickedInMenu = target.closest(".menu") !== null;
        const closesMenus = !clickedInMenu || actionElement !== null;

        if (closesMenus) {
            closeMenus();
        }

        if (actionElement && !actionElement.disabled) {
            void ACTIONS[actionElement.dataset.act ?? ""]?.();
        }
    });

    document.addEventListener("keydown", (event) => {
        // The lock dialog blocks clicks but not these shortcuts.
        if (appState.gameLocked) {
            return;
        }

        if (event.key === "F5") {
            event.preventDefault();
            void refreshEverything();
        }

        const isCtrlO = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "o";

        if (isCtrlO) {
            event.preventDefault();
            void pickAndAddMods();
        }
    });
}
