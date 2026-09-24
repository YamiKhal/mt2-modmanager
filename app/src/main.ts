import "./styles/index.css";

import { onPlanChanged } from "./app/buildCheck";
import { onStatusChanged, refresh } from "./app/refresh";
import { wireActions } from "./features/actions";
import { wireFileDrop } from "./features/addMods";
import { renderAppliedMods } from "./features/appliedMods";
import { renderAvailableMods, wireAvailableMods } from "./features/availableMods";
import { renderConflicts } from "./features/conflicts";
import { updateGameButtons } from "./features/gameButtons";
import { wireGameLock } from "./features/gameLock";
import { showGameUpdatedNotice } from "./features/gameUpdated";
import { wireHandInstalled } from "./features/handInstalled";
import { renderHeader, wireHeader } from "./features/header";
import { renderModDetails } from "./features/modDetails";
import { wireSettingsDialog } from "./features/settingsDialog";
import { reportError } from "./ui/busy";
import { wireDialogCloseButtons } from "./ui/dialogs";
import { log, setLogVisible } from "./ui/log";
import { wireMenus } from "./ui/menus";
import { wireTabs } from "./ui/tabs";
import { readSetting, STORAGE_KEYS } from "./util/storage";

onStatusChanged(renderHeader);
onStatusChanged(updateGameButtons);
onStatusChanged(renderAvailableMods);
onStatusChanged(renderAppliedMods);
onStatusChanged(renderModDetails);
onStatusChanged(showGameUpdatedNotice);

onPlanChanged(updateGameButtons);
onPlanChanged(renderConflicts);

wireMenus();
wireTabs();
wireDialogCloseButtons();
wireActions();
wireHeader();
wireAvailableMods();
wireHandInstalled();
wireSettingsDialog();
wireFileDrop();
wireGameLock();

setLogVisible(readSetting(STORAGE_KEYS.logVisible) === "1");
log("MT2 Mod Manager started");

refresh().catch((error: unknown) => reportError(String(error)));
