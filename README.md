# HMITranslator - Translation Manager for HMI Projects

![alt text](image/home.png)

HMITranslator is a desktop application designed to manage and translate text strings in HMI (Human Machine Interface) projects. It supports both Premium HMI and Movicon project formats, providing an intuitive interface for managing multilingual translations.

## Main Features

- **Project Import**: Load Premium HMI (`.hmiprj`) and Movicon (`.movprj`) project files to automatically detect translatable strings.
![alt text](image/add_new_cht_1.png)
![alt text](image/add_new_cht_2.png)

- **Translation Management**: Organize translations in dedicated tables for each project, supporting multiple languages.
![alt text](image/cht_view.png)

- **Record Organization**: Sort and reorder translation records to maintain project structure.
![alt text](image/select_and_move_cht.png)
![alt text](image/moved_cheat.png)

- **Translation Editor**: View and edit translation records with dedicated forms for each language.
![alt text](image/cheat_view.png)

- **Batch Operations**: Add single translations or use bulk import features for efficient workflow.
![alt text](image/create_one_new_cheat.png)
![alt text](image/magic_add_cheat.png)
![alt text](image/cheat_added_with_magic_cheat.png)

- **Export Functionality**: Export completed translations back to project files or generate language-specific files.
![alt text](image/export_cht.png)

- **AI Translation Integration**: Connect with external translation services (DeepL, Google Translate, Microsoft Translator) for automated translations.
![alt text](image/add_image.png)

- **Multilingual Interface**: Full internationalization support with Italian and English UI (`src/lib/i18n/{it.json,en.json}`).
![alt text](image/setting_page.png)

---

## System Requirements

- **Supported HMI Systems**: Premium HMI and Movicon (automatic detection)
- **Project Files**: `.hmiprj` / `.movprj` (project files), `.hmiscr` / `.movscr` (script files)
- **Translation Services**: DeepL, Google Translate, Microsoft Translator API integration
- **Platforms**: Windows, macOS, Linux (Tauri-based cross-platform application)

## Recent changes (changelog)

Below is an extended summary of recent modifications (frontend and backend), focusing on translation management and HMI compatibility.

### Key changes

- **Cross-platform HMI Support**: Full compatibility with both Premium HMI and Movicon project formats, with automatic detection and file extension handling.
- **Translation Record Management**: Advanced record management with `Add one` (single form) and bulk import capabilities for efficient translation workflow.
- **Smart Deletion Logic**: Intelligent confirmation system that only prompts for deletion when records contain actual translations or keys, preventing accidental data loss.
- **Stable Record Identifiers**: Migration to `id`-based record management to ensure consistent data handling and avoid DOM conflicts.
- **Advanced Record Organization**: Integration with SortableJS for drag & drop functionality, with order saving via `update_record_order` backend calls.
- **Row-level Actions**: Each translation record includes dedicated actions (View, Edit, Move up/down, Delete) for granular control.
- **ID-based Navigation**: Edit pages and record lookups now use stable `id` references instead of description-based routing.
- **Auto-refresh on Import**: Automatic table refresh after HMI project import to reflect newly detected strings.
- **AI Translation Integration**: Backend API connections to external translation services with configurable API keys and service selection.
- **Complete Internationalization**: Backend returns i18n keys (e.g. `settings.api_key_missing`, `home.project_imported`, `table.translation_saved`) instead of hardcoded strings, with full Italian and English translations in `src/lib/i18n/{it.json,en.json}`.

## Installation & Usage

1. **Download**: Get the latest release for your platform from the releases page
2. **Import Project**: Use "Import HMI Project" to load your `.hmiprj` or `.movprj` file
3. **Configure Translation Services**: Set up API keys for DeepL, Google Translate, or Microsoft Translator in Settings
4. **Manage Translations**: Add, edit, and organize your translation records
5. **Export**: Generate translated project files or export to various formats

## Technical Architecture

- **Frontend**: SvelteKit with Tailwind CSS and Bootstrap components
- **Backend**: Rust with Tauri for cross-platform desktop functionality  
- **Database**: SQLite for local translation storage and project management
- **APIs**: Integration with major translation service providers
- **File Processing**: Native handling of HMI project file formats

## Contributing

If you'd like to contribute: open an issue or a PR with a clear description of the feature or bug. For translation service integrations, include API documentation references and test cases.

## Support

This tool is designed for HMI developers working with Premium HMI and Movicon systems. For questions about specific HMI project formats or translation workflows, please include sample project files when reporting issues.

---

*HMITranslator - Simplifying multilingual HMI development*