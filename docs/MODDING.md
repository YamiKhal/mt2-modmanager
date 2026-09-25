# Making mods for MT2 Mod Manager

This guide is for anyone making mods for **MMORPG Tycoon 2**.

With the manager, your mod only contains **what it adds or changes**. You never copy a whole vanilla file. The manager merges every enabled mod into the game's files, in the load order the player picked, and warns about any conflicts.

## Folder layout

A mod is a folder (or a `.zip` of one) laid out like the game's data (`Data/MMORPG.zip`), plus a `manifest.json`:

```text
my_mod/
  manifest.json
  icon.png                   <- optional picture shown in the manager
  Powers.txt                 <- only your new or changed entries
  EditInventory.cfg          <- only your new items
  actionbars.win             <- only your new tab or buttons
  config.json                <- optional player settings (see Player settings)
  techs/my_techs.vrt         <- new techs (any file name)
  i18n/english/my_mod.vrt    <- your strings (any file name)
  costumes/…, weapons/…      <- images, models and other files: copied as-is
```

- **icon.png:** Square works best, 128×128 is a good size. Only shown in the manager, never copied into the game.
- **Not copied into the game:** `icon.png`, `config.json`, and any `README`, `LICENSE`, `CHANGELOG` or `CREDITS` file at the top level.

## manifest.json

```json
{
  "id": "mod-id",
  "name": "Mod Name",
  "version": "1.0.0",
  "author": "you",
  "description": "One or two sentences shown in the manager.",
  "contributors": ["a friend"],
  "mod_page": "https://…",
  "support": "https://…",
  "donate": "https://…",
  "source": "https://…",
  "license": "MIT",
  "tags": ["techs", "balance"],
  "dependencies": ["toolbox"],
  "incompatible": [],
  "replace": [],
  "game_versions": ["0.30"]
}
```

Only `id` and `name` are required. The rest is up to you.

- **id (required):** Your mod's unique id, and it's **permanent**. 2–24 characters of `a-z 0-9 _`, starting with a letter. Every new id your mod defines gets this as a prefix (see [Id prefixes](#id-prefixes-why-your-ids-change)). Changing it later breaks existing saves, so pick it once.
- **name (required):** The name shown in the manager.
- **version, author, description:** Shown in the manager.
- **contributors:** A list of other people to credit.
- **mod_page:** Where the mod is published. The manager shows a **Mod page** button for it.
- **support:** Where players can get help or report bugs. A link (issue tracker, Discord invite) or plain text.
- **donate, source:** A donation link and your source repository.
- **license:** e.g. `MIT` or `All rights reserved`.
- **tags:** Keywords shown in the manager. The filter box searches them.
- **dependencies:** Ids of mods that must be enabled and load **before** yours. If they aren't, the build stops.
- **incompatible:** Ids of mods that can't be enabled at the same time as yours.
- **replace:** Text files to replace entirely instead of merging, e.g. `["tweak/tweak.vrt"]`. You'll rarely need this.
- **game_versions:** Game versions you tested with. `"0.30"` matches `0.30.7-gdae20ea`. A mismatch is only a warning, not an error.

Link fields (`mod_page`, `support`, `donate`, `source`) open in the player's browser if they start with `https://`, `http://` or `mailto:`. Anything else is shown as text.

When a player adds a mod whose `id` is already in their library, the new copy replaces the old one (that's an update). It keeps its place in the player's profiles.

## Player settings (config.json)

Your mod can let players change some of its values in the manager: a price, a multiplier, whether something shows up at all.

Wherever a value should go, write `<key>` instead of the value, in any of your mod's text files:

```text
mmoTech
{
	id "megainns";
	cost <megainns_price>;
	…
}
```

Then declare the key in a `config.json` next to your `manifest.json`:

```json
[
  {
    "key": "megainns_price",
    "label": "Mega Inns research price",
    "type": "int",
    "default": 40000,
    "min": 10000,
    "max": 100000,
    "step": 5000,
    "unit": "coins",
    "input": "slider"
  },
  {
    "key": "megainns_boost",
    "label": "Inn capacity",
    "type": "choice",
    "default": "2",
    "options": [
      { "value": "1.5", "label": "+50%" },
      { "value": "2", "label": "Double" }
    ]
  }
]
```

The settings show up on the mod's **Details** tab. When the mod is applied, the manager puts each value where its `<key>` is: either the default, or whatever the player chose.

- **key (required):** The name you write between `<` and `>`. 1–64 characters of `A-Z a-z 0-9 _ - .`, starting with a letter.
- **type (required):** `int` (whole number), `float` (number), `bool` (`true`/`false`), `string` (text) or `choice` (one of `options`).
- **label:** The name the player sees. Uses the key if you leave it out.
- **default:** The value used until the player changes it. If you leave it out: `min` or 0, `false`, empty text, or the first option.
- **description:** Help text, shown when the player hovers over the label.
- **group:** A heading. Settings with the same group are shown together under it, in file order.
- **input:** `field` (default) or `slider`. A slider needs `int` or `float` and both `min` and `max`.
- **min, max, step:** Limits for numbers.
- **unit:** Shown after a number, e.g. `%` or `coins`.
- **options (required for choice):** The choices, either `"value"` or `{ "value": "…", "label": "…" }`. The value goes into your file, the label is what the player sees.

### What gets written

- `int` and `float` are written as numbers (`40000`, `1.25`).
- `bool` is written as `true` or `false`.
- `string` is written as typed, except `"` becomes `'`, `\` becomes `/`, and line breaks are removed. That way `name "<my_name>";` stays valid.
- `choice` is written exactly as the option's `value`.

Placeholders are filled in all merged text files (see [How text files are merged](#how-text-files-are-merged)), translations included. Ids that end up in a placeholder's value get prefixed like any other id. Images, models, shaders and other copied files are left alone.

### Checks

These show up on the **Conflicts** tab:

- **Broken config.json:** Bad JSON, an unknown field, a default outside its limits and so on. That's an error, and the build stops.
- **Unused key:** A declared key that no file uses. Warning.
- **Undeclared placeholder:** A `<word>` that looks like a placeholder but isn't declared. Warning. Only checked if the mod has a `config.json`, and it skips comment lines and tokens glued to other text like `tech_<id>_name`.

### Updating your mod

The player's choices are stored by the manager, not in your mod, so they survive updates. They're stored per profile, so a player can use different values in different profiles.

Only values that differ from the default are stored. If you change a default, players who never touched that setting get your new one.

When an update changes a setting, the stored value is handled like this:

- A value that no longer fits your new limits gets clamped into them.
- A choice that no longer exists goes back to the default.
- A type change goes back to the default if the old value doesn't fit the new type.
- A value for a removed key is dropped.

The player sees a note about each of these on the **Details** tab. If you want to keep a player's value, keep the key's name and type the same.

## How text files are merged

These files get merged: `.txt .cfg .win .def .vrt .costume .variant .conf .defaults .mat .block .prototype .adv .sequence .vsprite .tcolor .tshape .tstyle`, plus the extension-less step files in `scenarios/`. The manager reads them with a port of the game's own parser.

Everything else (`.png .vmb .glsl .van .bank` …) is copied. If two mods ship the same file that edit the same vanilla value (non-additive), the mod lower in the load order wins, and it's listed as a conflict.

### Matching your entries to existing ones

The manager figures out which existing entry each of your entries is meant for:

- **A block with an `id`, `name`, `boneName` or `slot` line,** e.g. `mmoTech { id "gamelog" … }`: matches the block with the same label and key value, and is merged into it line by line.
- **A block without those keys whose label is unique there,** e.g. `Inventory { … }` or `gameValueMod { … }`: matches the block with that label, and is merged into it.
- **A single line inside a block whose label is unique there,** e.g. `cost 5000`: matches the line with that label and **replaces** its value.
- **A line whose label repeats there,** e.g. `Item "@A"` among other `Item` lines, or any top-level line: matches the identical line. Added if it's missing, otherwise nothing happens.
- **Anything that matches nothing:** **appended** at the end of its parent.

Translation files work a bit differently. In `i18n/<language>/*.vrt` every top-level line is matched by its key, so you can override vanilla text. All of a language's files end up merged into that language's `00-base.vrt`.

### Examples

Add a toolbar item (`Powers.txt`):

```text
mmoItemTypeCommand
{
	name "@AddCash"
	key "item_addcash"
	iconName "icon-finance_report"
	command "add_cash 100000"
}
```

Make a vanilla tech cheaper (`techs/base.vrt`). Only the changed line is needed:

```text
mmoTech
{
	id "parties";
	cost 5000;
}
```

Put an item in the inventory (`EditInventory.cfg`):

```text
Inventory
{
	Item "@AddCash";
}
```

Add a button to an existing tab (`actionbars.win`). Buttons are matched by `slot`:

```text
mmoActionBar
{
	id "Edit";
	actionBarContent
	{
		mmoActionBarContent
		{
			id "Paths";
			configuration
			{
				mmoActionBarButtonConfiguration
				{
					slot 9;
					itemName "@AddCash";
				}
			}
		}
	}
}
```

### Removing and replacing

- **`__remove <label> <value>`** deletes a sibling entry:
  - `__remove Item "@Finance"` inside `Inventory { }` removes that item.
  - `__remove mmoTech "gamelog"` at the top of a tech file removes the block whose key value is `gamelog`.
- **`__replace`** on its own line inside a block makes your block replace the matched one completely instead of merging into it.

Only the manager reads these. They never reach the game.

## Id prefixes (why your ids change)

The game stores tech ids and item names inside save files. If two mods both added a tech called `crunchtime`, or a mod's id changed whenever another mod got installed, saves would break.

So the manager **always** prefixes every id your mod *defines* in your `manifest.json` with your mod's `id`, and rewrites your own references to match. With the mod id `crunch`:

- Tech `id "crunchtime"` in `techs/*.vrt` -> `crunch_crunchtime`
- `prerequisite "crunchtime"` -> `prerequisite "crunch_crunchtime"`
- `tech_crunchtime_displayname` in i18n -> `tech_crunch_crunchtime_displayname`
- Item `name "@AddCash"` in `Powers.txt` or `CursorBehaviours.txt` -> `@crunch_AddCash` (also in `EditInventory.cfg`, `itemName` and `itemTypeName`)
- Item `key "item_addcash"` -> `crunch_item_addcash` (and its `…_displayname` / `…_description` keys)
- Action bar tab `name` / content `id` -> `crunch_<name>`, tab `key` -> `crunch_<key>`
- Scenario `id` in `scenarios/library.txt` -> `crunch_<id>`

A few rules:

- **Vanilla ids are never renamed.** Using one means you're changing the vanilla entry.
- **Ids already starting with `<your id>_` are left alone,** so prefixing them yourself is fine.
- **Display text is never renamed.** That covers i18n values and the values of `text`, `title`, `tooltip`, `description`, `displayName`, `label` and `service`.
- **Referencing another mod's id:** write `othermod:id`. For example `prerequisite "crunch:crunchtime"` or `itemName "@toolbox:AddCash"` become `crunch_crunchtime` / `@toolbox_AddCash`. Writing `crunch_crunchtime` directly works too. Either way, add that mod to your `dependencies`.
- **Seeing every rename:** **Tools -> Build details -> Renamed ids** in the manager, or `mt2mm check` on the command line.

## Commands the manager blocks or flags

Toolbar items, `.win` buttons and scenario steps can run game commands, and a few of those can harm the player. The manager checks every mod's text files. What vanilla already does in the same file isn't reported, so shipping a full copy of `gamemenu.win` is fine.

| Found in your mod | Level | Why |
|---|---|---|
| `MessageTo Mode ConfirmDelete …` | error | Deletes a save file without asking |
| `MessageTo Game OpenURL <x>` where `x` isn't an `http://` or `https://` link | error | The game passes `x` to the Windows shell, which can start programs |
| `mmoPredicate_Event` | error | The game can't configure it from data, so it shows a failed assertion and closes |
| `MessageTo Mode Save`, `ConfirmSave`, `Autosave`, `ImmediateAutosave`, and the `mmoStepSave` step | warning | Writes a save and overwrites one with the same name |
| `MessageTo Mode Delete` | warning | Opens the delete-save dialog |
| `MessageTo Game GrantAchievement` / `RevokeAchievement` | warning | Changes Steam achievements |
| `crash`, `backgroundcrash`, `assert`, `MessageTo Core …` | warning | Closes the game or breaks the session |
| `prerequisites` on a scenario | warning | The game never checks it |
| `MessageTo Game OpenURL "https://…"` | info | Opens the link in the browser |
| `errorJump` | info | Not a field the game knows |

Errors stop Launch and Apply until the mod is disabled or fixed.

## Testing your mod

1. **Add mod** and pick your `.zip` or your folder's `manifest.json`, or drop the folder onto the window.
2. Check the **Conflicts** tab. It updates by itself after every change. Errors stop Launch, conflicts and warnings don't.
3. **Tools -> Build details** lists every file the game will get and every renamed id.
4. **Launch** (or **Apply**, which writes the mods without starting the game), start a **new** game, and check the game's `log.txt`:
   - Mod files are listed under `> Mods:`.
   - Toolbar buttons show up as `Adding edit inventory item '@x'`.
   - `Unable to find requested power "@x"` means an item is listed but not defined. Vanilla prints five of these on every load, so don't worry about those.
5. Anything showing up as `<<some_key>>` in game is a missing translation key.

Making a profile just for testing (**Profile -> New profile…**) keeps your test setup away from the mods you actually play with.

Researched techs and scenario progress are saved by id. Test on a throwaway game, and don't rename ids after you release.
