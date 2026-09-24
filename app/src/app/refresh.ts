import { getStatus } from "../api/commands";
import { clearIcons, loadIcons } from "../ui/modIcons";
import { scheduleBuildCheck } from "./buildCheck";
import { appState, pruneSelections } from "./state";

type Renderer = () => void;

interface RefreshOptions {
    reloadIcons?: boolean;
}

const statusRenderers: Renderer[] = [];


export function onStatusChanged(render: Renderer): void {
    statusRenderers.push(render);
}


export async function refresh(options: RefreshOptions = {}): Promise<void> {
    if (options.reloadIcons) {
        clearIcons();
    }

    const status = await getStatus();

    appState.status = status;
    pruneSelections(status);

    for (const render of statusRenderers) {
        render();
    }

    if (!appState.gameLocked) {
        void loadIcons(status.mods);
        scheduleBuildCheck();
    }
}
