import { appState } from "../app/state";
import { byId } from "../util/dom";


export function updateGameButtons(): void {
    const status = appState.status;

    if (!status) {
        return;
    }

    const enabledCount = status.mods.filter((mod) => mod.enabled).length;
    const deployState = status.deploy_state;
    const inSync = deployState === "current" || (deployState === "none" && enabledCount === 0);
    const hasErrors = buildErrorCount() > 0;
    const errorReason = appState.planError ?? "Fix the errors on the Conflicts tab first";

    const applyButton = byId<HTMLButtonElement>("btnApplyGame");

    applyButton.textContent = appState.applying ? "Applying…" : inSync ? "Applied" : "Apply";
    applyButton.disabled = appState.applying || inSync || hasErrors;
    applyButton.title = applyButtonTitle(inSync, hasErrors, errorReason);

    const launchButton = byId<HTMLButtonElement>("btnLaunch");

    launchButton.disabled = appState.applying || hasErrors;
    launchButton.title = hasErrors
        ? errorReason
        : "Apply the profile's mods to the game, then start it through Steam";
}

function applyButtonTitle(inSync: boolean, hasErrors: boolean, errorReason: string): string {
    if (inSync) {
        return "The game's mod folder already has this profile's mods.";
    }

    if (hasErrors) {
        return errorReason;
    }

    return "Write this profile's mods into the game's mod folder without starting the game.";
}

function buildErrorCount(): number {
    if (appState.planError) {
        return 1;
    }

    if (!appState.plan) {
        return 0;
    }

    return appState.plan.report.events.filter((event) => event.level === "error").length;
}
