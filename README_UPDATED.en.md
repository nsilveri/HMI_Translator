# SilCheat - Cheat Code Manager

![alt text](image/home.png)

SilCheat is a desktop application to manage and organize cheat codes for your favorite games. It lets you import, edit and reorder cheats in a simple and intuitive way.

## Main Features

- **CHT File Import**: Easily load `.cht` files that contain cheat codes to populate your tables.
![alt text](image/add_new_cht_1.png)
![alt text](image/add_new_cht_2.png)

- **Table Management**: Create and organize separate tables for each game to keep everything tidy.
![alt text](image/cht_view.png)

- **Move Cheats**: Select and change the position of a cheat.
![alt text](image/select_and_move.png)
![alt text](image/moved_cheat.png)

- **View and Edit**: See details for each cheat, edit them or add new ones.
![alt text](image/cheat_view.png)

- **Single and Bulk Add**: Add a single cheat or use "Magic add" to paste multiple cheats as free text.
![alt text](image/create_one_new_cheat.png)
![alt text](image/magic_add_cheat.png)
![alt text](image/cheat_added_with_magic_cheat.png)

- **Export**: Save your tables as `.cht` files to share or use elsewhere.
![alt text](image/export_cht.png)

- **Logo Download**: Download logo/boxart from external services (RAWG / TheGamesDB) and set them as the table image.
![alt text](image/add_image.png)

- **Localization (i18n)**: UI texts available in Italian and English (`src/lib/i18n/{it.json,en.json}`); the backend returns i18n keys that are translated by the UI.
![alt text](image/setting_page.png)

---

## Recent changes (changelog)

Below is an extended summary of recent modifications (frontend and backend), test instructions and developer notes.

### Key changes

- Single `Add` button with dropdown: contains `Add one` (single form) and `Magic add` (bulk modal).
- "Bulk add" modal with smart parsing (`src/routes/home/proj_view/+page.svelte`): supports several pasted formats (labelled pairs `desc:`/`code:`, blank-line separated blocks, single-line `desc code`, or alternating lines). It's designed to simplify fast imports of many cheats.
- Migration to stable identifiers: lists now use `id` as the Svelte key to avoid duplicate/duplicate DOM elements.
- Robust drag & drop: integration with SortableJS (handle, fallbackOnBody, ghost/chosen classes) and order saving via `update_record_order` that sends an array of `id`s to the backend.
- Row selection + header Up/Down controls: select a single row and move it with header arrows; a toast appears if nothing is selected.
- Per-row Up/Down buttons and actions: each row has actions (View, Edit, Move up/down, Delete).
- Edit page uses `id`: the edit link and page now look up the record by `id` (no longer by `desc`).
- Home refresh after import: after importing a `.cht` the table list is reloaded automatically.
- Logo functionality: frontend `fetchAndSetLogo()` calls the backend which tries to fetch images from RAWG/TheGamesDB and save them for the table.
- Backend localization → i18n keys: the backend now returns error keys (e.g. `settings.api_key_missing`, `home.no_game_found_rawg`, `table.duplicate_desc`) instead of hard-coded Italian strings. These keys are translated in `src/lib/i18n/it.json` and `src/lib/i18n/en.json`.

## Contributing

If you'd like to contribute: open an issue or a PR with a clear description of the feature or bug and, if possible, include reproduction steps. For changes to the bulk parser, include real input examples you want supported.

---

For questions or support, contact the maintainer.
