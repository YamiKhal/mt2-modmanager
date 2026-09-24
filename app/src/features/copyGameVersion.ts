import { currentStatus } from "../app/state";
import { log } from "../ui/log";
import { toast } from "../ui/toast";


export async function copyGameVersion(): Promise<void> {
    const version = currentStatus().game_version;

    if (!version) {
        toast("Game version not known yet. It shows once the game has run.", true);

        return;
    }

    try {
        await navigator.clipboard.writeText(version);
    } catch {
        toast("Couldn't copy to the clipboard.", true);

        return;
    }

    toast(`Copied game version ${version}`);
    log(`Copied game version: ${version}`);
}
