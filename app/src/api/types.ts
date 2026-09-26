export interface Manifest {
    id: string;
    name: string;
    version: string;
    author: string;
    description: string;
    contributors?: string[];
    mod_page?: string;
    support?: string;
    donate?: string;
    source?: string;
    license?: string;
    tags?: string[];
    dependencies: string[];
    incompatible: string[];
    replace: string[];
    game_versions: string[];
}

export type SettingType = "int" | "float" | "bool" | "string" | "choice";
export type SettingInput = "field" | "slider";
export type SettingValue = number | boolean | string;

export interface SettingChoice {
    value: string;
    label: string;
}

export interface ConfigOption {
    key: string;
    label: string;
    type: SettingType;
    default: SettingValue;
    description?: string;
    group?: string;
    input: SettingInput;
    min?: number;
    max?: number;
    step?: number;
    unit?: string;
    options?: SettingChoice[];
}

export interface ResolvedSettings {
    values: Record<string, SettingValue>;
    changed: string[];
    notes: string[];
    saved_for?: string;
}

export interface LibraryMod {
    folder: string;
    dir: string;
    manifest: Manifest | null;
    error: string | null;
    applied: boolean;
    enabled: boolean;
    icon: boolean;
    config: ConfigOption[];
    config_error: string | null;
    settings: ResolvedSettings;
    dev_source: string | null;
}

export interface GamePaths {
    install_dir: string | null;
    data_zip: string | null;
    profile_dir: string | null;
}

export type DeployState = "none" | "current" | "outdated";

export interface Status {
    paths: GamePaths;
    mod_dir: string | null;
    manager_dir: string | null;
    game_version: string | null;
    mods: LibraryMod[];
    unmanaged: string[];
    deployed: string[];
    deploy_state: DeployState;
    profiles: string[];
    active_profile: string;
    deployed_mods: string[];
    problems: string[];
    game_update: GameUpdate | null;
}

export interface GameUpdate {
    data_time: number;
    version: string | null;
}

export type Level = "info" | "warning" | "conflict" | "error";

export interface ReportEvent {
    level: Level;
    mod_id: string;
    file: string;
    path: string;
    message: string;
}

export interface OutputFile {
    rel: string;
    how: "merged" | "record" | "copied" | "replaced";
    sources: string[];
}

export interface Rename {
    mod_id: string;
    kind: string;
    from: string;
    to: string;
}

export interface BuildPlan {
    mods: string[];
    files: OutputFile[];
    renames: Rename[];
    report: {
        events: ReportEvent[];
    };
    game_version: string | null;
}

export interface Imported {
    source: string;
    id: string | null;
    updated: boolean;
    error: string | null;
}

export interface GameConfig {
    install_dir: string | null;
    data: string | null;
    profile_dir: string | null;
}

export interface LaunchResult {
    plan: BuildPlan;
    applied: boolean;
}
