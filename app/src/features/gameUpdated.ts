import { currentStatus } from "../app/state";
import { byId } from "../util/dom";
import { readSetting, STORAGE_KEYS, writeSetting } from "../util/storage";


export function showGameUpdatedNotice(): void {
    const update = currentStatus().game_update;
    const dialog = byId<HTMLDialogElement>("gameUpdated");

    if (!update || dialog.open) {
        return;
    }

    const updateKey = String(update.data_time);

    if (readSetting(STORAGE_KEYS.seenGameUpdate) === updateKey) {
        return;
    }

    byId("gameUpdatedVersion").textContent = update.version
        ? `Version ${update.version}`
        : "The new version number shows once the game has run.";

    writeSetting(STORAGE_KEYS.seenGameUpdate, updateKey);
    dialog.showModal();
}
