import { checkBuild } from "../api/commands";
import { endBusy, startBusy } from "../ui/busy";
import { appState } from "./state";

type Renderer = () => void;

const CHECK_DELAY_MS = 250;
const planRenderers: Renderer[] = [];

let checkTimer: number | undefined;
let latestCheck = 0;


export function onPlanChanged(render: Renderer): void {
    planRenderers.push(render);
}


export function scheduleBuildCheck(): void {
    window.clearTimeout(checkTimer);

    if (appState.gameLocked) {
        return;
    }

    checkTimer = window.setTimeout(() => void runBuildCheck(), CHECK_DELAY_MS);
}

async function runBuildCheck(): Promise<void> {
    latestCheck += 1;
    const thisCheck = latestCheck;

    startBusy("Checking mods…");

    try {
        const plan = await checkBuild();

        if (thisCheck !== latestCheck || appState.gameLocked) {
            return;
        }

        appState.plan = plan;
        appState.planError = null;
    } catch (error) {
        if (thisCheck !== latestCheck) {
            return;
        }

        appState.plan = null;
        appState.planError = String(error);
    } finally {
        endBusy();
    }

    for (const render of planRenderers) {
        render();
    }
}
