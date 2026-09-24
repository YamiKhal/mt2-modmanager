import type { BuildPlan, OutputFile, ReportEvent, Rename, Status } from "../api/types";
import { appState, currentStatus, modNameById } from "../app/state";
import { byId } from "../util/dom";
import { baseName, escapeHtml } from "../util/text";
import { sortBySeverity } from "./reportEvents";


export function openBuildDetails(): void {
    const plan = appState.plan;

    byId("dtab-report").innerHTML = reportHtml(plan);
    byId("dtab-files").innerHTML = plan?.files.length ? filesHtml(plan.files) : emptyHtml("No files.");
    byId("dtab-renames").innerHTML = plan?.renames.length
        ? renamesHtml(plan.renames)
        : emptyHtml("No ids needed renaming.");
    byId("dtab-paths").innerHTML = foldersHtml(currentStatus());

    byId<HTMLDialogElement>("details").showModal();
}

function emptyHtml(text: string): string {
    return `<p class="empty">${text}</p>`;
}


function reportHtml(plan: BuildPlan | null): string {
    if (!plan) {
        return emptyHtml(appState.planError ? escapeHtml(appState.planError) : "No build yet.");
    }

    if (plan.report.events.length === 0) {
        return emptyHtml("Nothing to report.");
    }

    const rows = sortBySeverity(plan.report.events).map(reportRowHtml).join("");

    return `
        <table class="report">
            <thead><tr><th>Level</th><th>Mod</th><th>Message</th><th>Where</th></tr></thead>
            <tbody>${rows}</tbody>
        </table>`;
}

function reportRowHtml(event: ReportEvent): string {
    const mod = event.mod_id ? modNameById(event.mod_id) : "";
    const quotedPath = event.path ? `"${event.path}"` : "";
    const where = [event.file, quotedPath].filter(Boolean).join(" ");

    return `
        <tr>
            <td><span class="lvl ${event.level}">${event.level}</span></td>
            <td>${escapeHtml(mod)}</td>
            <td>${escapeHtml(event.message)}</td>
            <td class="mono where">${escapeHtml(where)}</td>
        </tr>`;
}


function filesHtml(files: OutputFile[]): string {
    const rows = files
        .map((file) => `
            <tr>
                <td class="mono">${escapeHtml(file.rel)}</td>
                <td>${escapeHtml(file.how)}</td>
                <td>${escapeHtml(file.sources.join(", "))}</td>
            </tr>`)
        .join("");

    return `
        <table>
            <thead><tr><th>File written to the game</th><th>How</th><th>From</th></tr></thead>
            <tbody>${rows}</tbody>
        </table>`;
}


function renamesHtml(renames: Rename[]): string {
    const rows = renames
        .map((rename) => `
            <tr>
                <td>${escapeHtml(rename.mod_id)}</td>
                <td>${escapeHtml(rename.kind)}</td>
                <td class="mono">${escapeHtml(rename.from)}</td>
                <td class="mono">${escapeHtml(rename.to)}</td>
            </tr>`)
        .join("");

    return `
        <p class="muted note">New ids get the mod's id as a prefix so they never clash with other mods and stay the same in your saves.</p>
        <table>
            <thead><tr><th>Mod</th><th>Kind</th><th>Written as</th><th>In game</th></tr></thead>
            <tbody>${rows}</tbody>
        </table>`;
}


function foldersHtml(status: Status): string {
    const paths = status.paths;
    const deployed = status.deployed.length ? status.deployed.map(baseName).join(", ") : "nothing";
    const handInstalled = status.unmanaged.length ? status.unmanaged.map(baseName).join(", ") : "none";

    return `
        <table>
            ${folderRow("Game version", status.game_version)}
            ${folderRow("Game install", paths.install_dir)}
            ${folderRow("Game data", paths.data_zip)}
            ${folderRow("Game profile", paths.profile_dir)}
            ${folderRow("Mod folder", status.mod_dir)}
            ${folderRow("Manager folder", status.manager_dir)}
            ${folderRow("Deployed", deployed)}
            ${folderRow("Hand-installed", handInstalled)}
        </table>`;
}

function folderRow(label: string, value: string | null): string {
    return `<tr><th>${label}</th><td class="mono">${escapeHtml(value ?? "not found")}</td></tr>`;
}
