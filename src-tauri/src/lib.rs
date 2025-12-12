use rusqlite::{params_from_iter, params, Connection, Result};
use std::fs;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
extern crate regex;
use encoding_rs::{WINDOWS_1252, UTF_8};
use tauri_plugin_shell;

// Module declarations
mod alarms;
mod translations;
mod variables;

// Re-export the tauri commands from modules
pub use alarms::{get_alarm_files_in_directory, import_alarm_file_from_path, get_alarm_imported_files, delete_alarm, delete_alarms_database, add_alarm, update_alarm, get_alarm, get_translation_keys, export_alarms, export_alarms_to_machine};
pub use translations::{
    get_translation_files_in_directory, import_translation_file_from_path,
    import_translation_file_with_merge, add_language_to_project, get_project_languages,
    import_translation_file, remove_language_from_project, export_translations_per_language,
    export_translations_to_machine
};
pub use variables::{
    get_variable_files_in_directory, import_variable_file, get_variable_imported_files,
    get_variables, get_structures, get_structure_members, delete_variable, update_variable
};

// Robust file reader: tries UTF-8, UTF-8 BOM, UTF-16 LE BOM, then falls back to Windows-1252
pub fn read_text_file_best_effort(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;

    // Try plain UTF-8
    if let Ok(s) = String::from_utf8(bytes.clone()) {
        return Ok(s);
    }

    // UTF-8 with BOM (EF BB BF)
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        if let Ok(s) = String::from_utf8(bytes[3..].to_vec()) {
            return Ok(s);
        }
    }

    // UTF-16 LE with BOM (FF FE)
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        if let Ok(s) = String::from_utf16(&utf16) {
            return Ok(s);
        }
    }

    // Fallback: decode as Windows-1252 (common for Movicon / PremiumHMI)
    let (cow, _, _) = WINDOWS_1252.decode(&bytes);
    Ok(cow.into_owned())
}

#[tauri::command]
fn get_tables() -> Result<Vec<std::collections::HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Create table_images if not exists
    conn.execute("CREATE TABLE IF NOT EXISTS table_images (table_name TEXT PRIMARY KEY, image TEXT);", []).map_err(|e| e.to_string())?;
    // Create settings table if not exists
    conn.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);", []).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'table_images' AND name != 'settings' AND name != 'projects' AND name != 'project_languages' AND name NOT LIKE '%_imports';").map_err(|e| e.to_string())?;
    let table_names: Vec<String> = stmt.query_map([], |row| row.get(0)).map_err(|e| e.to_string())?.map(|r| r.unwrap()).collect();
    let mut tables = Vec::new();
    for name in table_names {
        let mut map = std::collections::HashMap::new();
        map.insert("name".to_string(), name.clone());
        // Get image
        let image: Option<String> = conn.query_row("SELECT image FROM table_images WHERE table_name = ?", [name], |row| row.get(0)).unwrap_or(None);
        map.insert("image".to_string(), image.unwrap_or_default());
        tables.push(map);
    }
    Ok(tables)
}

// Helper function for internal use (without #[tauri::command])
pub fn get_records_internal(tableName: String) -> std::result::Result<Vec<std::collections::HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Get column names
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", tableName)).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.query_map([], |row| Ok(row.get::<_, String>(1)?)).map_err(|e| e.to_string())?.collect::<std::result::Result<_, _>>().map_err(|e| e.to_string())?;
    
    // Check if order_index column exists
    let has_order_index = columns.contains(&"order_index".to_string());
    let order_clause = if has_order_index { "ORDER BY order_index ASC, id ASC" } else { "ORDER BY id ASC" };
    
    let select_sql = format!("SELECT {} FROM `{}` {};", columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", "), tableName, order_clause);
    let mut stmt = conn.prepare(&select_sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let mut map = std::collections::HashMap::new();
        for (i, col) in columns.iter().enumerate() {
            // Ensure id is read as integer and converted to string so frontend gets a valid id
            let val: String = if col == "id" {
                // attempt to read as i64, fall back to string
                match row.get::<_, i64>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => row.get::<_, String>(i).unwrap_or_else(|_| "".to_string()),
                }
            } else {
                row.get::<_, String>(i).unwrap_or_else(|_| "".to_string())
            };
            if !map.contains_key(col) {
                map.insert(col.clone(), val);
            }
        }
        Ok(map)
    }).map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|e| e.to_string())?);
    }
    Ok(records)
}

#[tauri::command]
fn get_records(tableName: String) -> std::result::Result<Vec<std::collections::HashMap<String, String>>, String> {
    get_records_internal(tableName)
}

#[tauri::command]
fn set_table_image(tableName: String, imagePath: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Create table if not exists
    conn.execute("CREATE TABLE IF NOT EXISTS table_images (table_name TEXT PRIMARY KEY, image TEXT);", []).map_err(|e| e.to_string())?;
    // Read image file
    let image_data = fs::read(&imagePath).map_err(|e| e.to_string())?;
    let base64 = general_purpose::STANDARD.encode(&image_data);
    // Insert or update
    conn.execute("INSERT OR REPLACE INTO table_images (table_name, image) VALUES (?, ?)", params![tableName, base64]).map_err(|e| e.to_string())?;
    Ok("Immagine impostata.".to_string())
}

#[tauri::command]
fn delete_table_image(tableName: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM table_images WHERE table_name = ?", [tableName]).map_err(|e| e.to_string())?;
    Ok("Immagine eliminata.".to_string())
}

#[tauri::command]
fn update_record(tableName: String, id: String, updates: std::collections::HashMap<String, String>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // parse id
    let id_num: i64 = id.parse::<i64>().map_err(|e: std::num::ParseIntError| e.to_string())?;

    // If desc is being updated, ensure uniqueness (ignore current record)
    if let Some(new_desc) = updates.get("desc") {
        let count: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM `{}` WHERE `desc` = ? AND id != ?", tableName), params![new_desc, id_num], |row| row.get(0)).map_err(|e| e.to_string())?;
        if count > 0 {
            return Err("table.duplicate_desc".to_string());
        }
    }

    let set_clause = updates.keys().map(|k| format!("`{}` = ?", k)).collect::<Vec<_>>().join(", ");
    let sql = format!("UPDATE `{}` SET {} WHERE id = ?", tableName, set_clause);
    let params: Vec<String> = updates.values().cloned().collect();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
    param_refs.push(&id_num);
    stmt.execute(param_refs.as_slice()).map_err(|e| e.to_string())?;
    Ok("Record aggiornato.".to_string())
}

#[tauri::command]
fn delete_record(tableName: String, id: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // parse id to integer
    let id_num: i64 = id.parse::<i64>().map_err(|e: std::num::ParseIntError| e.to_string())?;
    conn.execute(&format!("DELETE FROM `{}` WHERE id = ?", tableName), params![id_num]).map_err(|e| e.to_string())?;
    Ok("Record eliminato.".to_string())
}

#[tauri::command]
fn insert_record(tableName: String, record: std::collections::HashMap<String, String>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Check if desc already exists
    if let Some(desc) = record.get("desc") {
        let count: i64 = conn.query_row(&format!("SELECT COUNT(*) FROM `{}` WHERE `desc` = ?", tableName), [desc], |row| row.get(0)).map_err(|e| e.to_string())?;
        if count > 0 {
            return Err("table.duplicate_desc".to_string());
        }
    }
    let columns: Vec<String> = record.keys().cloned().collect();
    let placeholders = vec!["?".to_string(); columns.len()].join(", ");
    let quoted_columns = columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
    let sql = format!("INSERT INTO `{}` ({}) VALUES ({})", tableName, quoted_columns, placeholders);
    let values: Vec<String> = record.values().cloned().collect();
    let params = params_from_iter(values.iter());
    conn.execute(&sql, params).map_err(|e| e.to_string())?;
    Ok("Record inserito.".to_string())
}

// Helper function for internal use
pub fn get_table_info_internal(table_name: String) -> std::result::Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut result = std::collections::HashMap::new();
    
    // Ottieni le colonne della tabella
    let columns = get_table_columns_internal(table_name.clone())?;
    result.insert("columns".to_string(), serde_json::json!(columns));
    
    // Ottieni le lingue del progetto se esistono
    match translations::get_project_languages(table_name.clone()) {
        Ok(languages) => {
            result.insert("languages".to_string(), serde_json::json!(languages));
        }
        Err(_) => {
            result.insert("languages".to_string(), serde_json::json!([]));
        }
    }
    
    // Ottieni il percorso del progetto dalla tabella projects
    let mut stmt = conn.prepare("SELECT path FROM projects WHERE name = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query_map([&table_name], |row| {
        Ok(row.get::<_, String>(0)?)
    }).map_err(|e| e.to_string())?;
    
    if let Some(path_result) = rows.next() {
        let path = path_result.map_err(|e| e.to_string())?;
        result.insert("path".to_string(), serde_json::json!(path));
    }
    
    Ok(result)
}

#[tauri::command]
fn get_table_info(table_name: String) -> std::result::Result<std::collections::HashMap<String, serde_json::Value>, String> {
    get_table_info_internal(table_name)
}

// Helper function for internal use
pub fn get_table_columns_internal(tableName: String) -> std::result::Result<Vec<String>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", tableName)).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.query_map([], |row| Ok(row.get::<_, String>(1)?)).map_err(|e| e.to_string())?.collect::<std::result::Result<_, _>>().map_err(|e| e.to_string())?;
    Ok(columns)
}

#[tauri::command]
fn get_table_columns(tableName: String) -> std::result::Result<Vec<String>, String> {
    get_table_columns_internal(tableName)
}

#[tauri::command]
async fn fetch_and_set_logo(tableName: String, gameName: String) -> Result<String, String> {
    let service = get_setting("api_service".to_string()).unwrap_or("thegamesdb".to_string());
    let api_key = get_setting("thegamesdb_api_key".to_string())?;
    if api_key.is_empty() {
        return Err("settings.api_key_missing".to_string());
    }

    let client = Client::new();

    if service == "rawg" {
        // RAWG API
        let search_url = format!("https://api.rawg.io/api/games?key={}&search={}&page_size=1", api_key, urlencoding::encode(&gameName));
        println!("fetch_and_set_logo: searching RAWG for '{}'", gameName);
        let response: RawgResponse = client.get(&search_url).send().await.map_err(|e| format!("Errore nella ricerca RAWG: {}", e))?.json().await.map_err(|e| format!("Errore nel parsing RAWG: {}", e))?;

        if response.results.is_empty() {
            return Err("home.no_game_found_rawg".to_string());
        }

        let game = &response.results[0];
    let image_url = game.background_image.as_ref().ok_or("home.no_image_available_rawg".to_string())?;
        println!("fetch_and_set_logo: downloading from RAWG {}", image_url);

        let response = client.get(image_url).send().await.map_err(|e| format!("Errore nel download RAWG: {}", e))?;
        let bytes = response.bytes().await.map_err(|e| format!("Errore nella lettura bytes RAWG: {}", e))?;
        println!("fetch_and_set_logo: downloaded {} bytes from RAWG", bytes.len());

        let temp_path = format!("../data/temp_logo_{}.png", tableName);
        fs::write(&temp_path, &bytes).map_err(|e| format!("Errore scrittura temp RAWG: {}", e))?;
        println!("fetch_and_set_logo: wrote temp file {}", temp_path);

        set_table_image(tableName, temp_path)
    } else {
        // TheGamesDB
        let search_url = format!("https://api.thegamesdb.net/v1/Games/ByGameName?apikey={}&name={}&fields=artworks", api_key, urlencoding::encode(&gameName));
        println!("fetch_and_set_logo: searching TheGamesDB for '{}'", gameName);
        let search_response: GamesResponse = client.get(&search_url).send().await.map_err(|e| format!("Errore nella ricerca TheGamesDB: {}", e))?.json().await.map_err(|e| format!("Errore nel parsing TheGamesDB: {}", e))?;

        if search_response.data.games.is_empty() {
            return Err("home.no_game_found_thegamesdb".to_string());
        }

        let game = &search_response.data.games[0];
        let game_id = game.id;
        println!("fetch_and_set_logo: found game '{}' with id {}", game.name, game_id);

        let images_url = format!("https://api.thegamesdb.net/v1/Games/Images?apikey={}&games_id={}", api_key, game_id);
        let images_response: ImagesResponse = client.get(&images_url).send().await.map_err(|e| format!("Errore nel recupero immagini TheGamesDB: {}", e))?.json().await.map_err(|e| format!("Errore nel parsing immagini TheGamesDB: {}", e))?;

    let game_images = images_response.data.images.get(&game_id.to_string()).ok_or("home.no_image_available_thegamesdb".to_string())?;

    let logo_image = game_images.iter().find(|img| img.image_type == "logo").or_else(|| game_images.iter().find(|img| img.image_type == "boxart")).ok_or("home.no_logo_or_boxart_thegamesdb".to_string())?;

        let image_url = format!("{}/{}", images_response.data.base_url.original.trim_end_matches('/'), logo_image.filename);
        println!("fetch_and_set_logo: downloading from TheGamesDB {}", image_url);

        let response = client.get(&image_url).send().await.map_err(|e| format!("Errore nel download TheGamesDB: {}", e))?;
        let bytes = response.bytes().await.map_err(|e| format!("Errore nella lettura bytes TheGamesDB: {}", e))?;
        println!("fetch_and_set_logo: downloaded {} bytes from TheGamesDB", bytes.len());

        let temp_path = format!("../data/temp_logo_{}.png", tableName);
        fs::write(&temp_path, &bytes).map_err(|e| format!("Errore scrittura temp TheGamesDB: {}", e))?;
        println!("fetch_and_set_logo: wrote temp file {}", temp_path);

        set_table_image(tableName, temp_path)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KeyWithFile {
    key: String,
    file: String,
    full_line: String,
    all_files: Vec<String>, // JSON array di tutti i file dove appare questa chiave
}

#[tauri::command]
fn find_keys_in_project(directory_path: String, project_name: String) -> Result<Vec<KeyWithFile>, String> {
    let base_path = Path::new(&directory_path);
    
    if !base_path.is_dir() {
        return Err("Il percorso specificato non è una directory".to_string());
    }
    
    // Percorso verso la directory RESOURCES
    let resources_path = base_path.join("RESOURCES");
    
    if !resources_path.exists() || !resources_path.is_dir() {
        return Err("Directory RESOURCES non trovata nel progetto".to_string());
    }
    
    // Trova tutte le cartelle in RESOURCES che sono contenute nel nome del progetto
    let mut matching_folders = Vec::new();
    println!("Cerco cartelle in: {}", resources_path.display());
    println!("Nome progetto: {}", project_name);
    
    let entries = fs::read_dir(&resources_path).map_err(|e| e.to_string())?;
    
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let folder_path = entry.path();
        
        if folder_path.is_dir() {
            if let Some(folder_name) = folder_path.file_name() {
                if let Some(folder_name_str) = folder_name.to_str() {
                    println!("Trovata cartella: {}", folder_name_str);
                    // Controlla se il nome della cartella è contenuto nel nome del progetto
                    if project_name.contains(folder_name_str) {
                        println!("Cartella {} corrisponde al progetto {}", folder_name_str, project_name);
                        matching_folders.push(folder_path);
                    }
                }
            }
        }
    }
    
    println!("Cartelle corrispondenti trovate: {}", matching_folders.len());
    
    if matching_folders.is_empty() {
        return Err(format!("Nessuna cartella in RESOURCES corrisponde al progetto '{}'. Cartelle disponibili controllate sopra.", project_name));
    }
    
    let mut found_keys = Vec::new();
    
    // Funzione ricorsiva per scansionare le directory
    fn scan_directory(dir: &Path, keys: &mut Vec<KeyWithFile>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_path = entry.path();

            if file_path.is_dir() {
                scan_directory(&file_path, keys)?;
            } else if let Some(name) = file_path.file_name().and_then(|n| n.to_str()) {
                let name_lower = name.to_lowercase();
                if name_lower.ends_with(".hmiscr") || name_lower.ends_with(".movscr") {
                    println!("Scansiono file: {}", file_path.display());

                    // --- Lettura file robusta (UTF-8 o UTF-16) ---
                    let content = read_text_file_best_effort(&file_path)?;

                    // --- Dividi in righe ---
                    let mut local_count = 0;
                    for line in content.lines() {
                        if line.contains("</text>") {
                            // Trova la parte tra '>' e '</text>'
                            if let Some(start) = line.find('>') {
                                if let Some(end) = line.find("</text>") {
                                    if end > start + 1 {
                                        let key_str = &line[start + 1..end];
                                        let key_str = key_str.trim();

                                        // Filtra contenuti non validi
                                        if key_str.is_empty()
                                            || key_str.len() > 100
                                            || key_str.contains('<')
                                            || key_str.contains('>')
                                            || key_str.chars().all(|c| c.is_numeric() || c == '0')
                                        {
                                            continue;
                                        }

                                        keys.push(KeyWithFile {
                                            key: key_str.to_string(),
                                            file: name.to_string(),
                                            full_line: line.trim().to_string(),
                                            all_files: vec![name.to_string()], // Inizialmente solo questo file
                                        });
                                        local_count += 1;
                                    }
                                }
                            }
                        }
                    }

                    println!("  → Estratte {} chiavi da {}", local_count, name);
                }
            }
        }

        Ok(())
    }
    
    // Scansiona tutte le cartelle che corrispondono
    for folder_path in matching_folders {
        scan_directory(&folder_path, &mut found_keys)?;
    }
    
    // Raggruppa le chiavi duplicate raccogliendo tutti i file dove appaiono
    use std::collections::HashMap;
    let mut key_map: HashMap<String, (String, String, Vec<String>)> = HashMap::new();
    
    for key_entry in found_keys {
        let entry = key_map.entry(key_entry.key.clone()).or_insert((
            key_entry.file.clone(),
            key_entry.full_line.clone(),
            Vec::new()
        ));
        
        // Aggiungi il file alla lista se non è già presente
        if !entry.2.contains(&key_entry.file) {
            entry.2.push(key_entry.file);
        }
    }
    
    // Converti la HashMap in Vec<KeyWithFile>
    let mut result: Vec<KeyWithFile> = key_map.into_iter().map(|(key, (file, full_line, all_files))| {
        KeyWithFile {
            key,
            file, // File primario (primo trovato) 
            full_line,
            all_files,
        }
    }).collect();
    
    // Ordina per chiave
    result.sort_by(|a, b| a.key.cmp(&b.key));
    
    Ok(result)
}

#[tauri::command]
fn import_project_keys(project_name: String, keys: Vec<KeyWithFile>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Prima assicuriamoci che la colonna keys_project esista e che key possa essere NULL
    let check_column_sql = format!(
        "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name='keys_project'",
        project_name
    );
    
    let column_exists: i32 = conn.query_row(&check_column_sql, [], |row| {
        Ok(row.get::<_, i32>("count")?)
    }).unwrap_or(0);
    
    if column_exists == 0 {
        // Aggiungi la colonna keys_project se non esiste
        let add_column_sql = format!(
            "ALTER TABLE `{}` ADD COLUMN keys_project TEXT",
            project_name
        );
        conn.execute(&add_column_sql, []).map_err(|e| e.to_string())?;
        println!("Aggiunta colonna keys_project alla tabella {}", project_name);
    }
    
    // Verifica se esiste la colonna key_files per memorizzare i file JSON
    let check_files_column_sql = format!(
        "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name='key_files'",
        project_name
    );
    
    let files_column_exists: i32 = conn.query_row(&check_files_column_sql, [], |row| {
        Ok(row.get::<_, i32>("count")?)
    }).unwrap_or(0);
    
    if files_column_exists == 0 {
        // Aggiungi la colonna key_files se non esiste
        let add_files_column_sql = format!(
            "ALTER TABLE `{}` ADD COLUMN key_files TEXT",
            project_name
        );
        conn.execute(&add_files_column_sql, []).map_err(|e| e.to_string())?;
        println!("Aggiunta colonna key_files alla tabella {}", project_name);
    }
    
    // Verifica se la colonna key ha constraint NOT NULL e lo rimuove se necessario
    // Purtroppo SQLite non supporta ALTER COLUMN, quindi creiamo una nuova tabella
    let table_info_sql = format!("PRAGMA table_info('{}')", project_name);
    let mut stmt = conn.prepare(&table_info_sql).map_err(|e| e.to_string())?;
    let mut key_is_not_null = false;
    
    let rows = stmt.query_map([], |row| {
        let column_name: String = row.get("name")?;
        let not_null: i32 = row.get("notnull")?;
        if column_name == "key" && not_null == 1 {
            key_is_not_null = true;
        }
        Ok(())
    }).map_err(|e| e.to_string())?;
    
    for _ in rows {
        // Itera per eseguire la query
    }
    
    if key_is_not_null {
        println!("Aggiornamento schema tabella {} per permettere key NULL", project_name);
        
        // Prima ottieni TUTTE le colonne esistenti
        let mut columns = Vec::new();
        let mut column_definitions = Vec::new();
        
        let table_info_sql = format!("PRAGMA table_info('{}')", project_name);
        let mut stmt = conn.prepare(&table_info_sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get("name")?;
            let column_type: String = row.get("type")?;
            let not_null: i32 = row.get("notnull")?;
            let pk: i32 = row.get("pk")?;
            let default_value: Option<String> = row.get("dflt_value")?;
            
            columns.push(name.clone());
            
            // Costruisci la definizione della colonna
            let mut def = format!("{} {}", name, column_type);
            if pk == 1 {
                def.push_str(" PRIMARY KEY AUTOINCREMENT");
            } else if not_null == 1 && name != "key" {
                def.push_str(" NOT NULL");
            }
            if let Some(default) = default_value {
                def.push_str(&format!(" DEFAULT {}", default));
            } else if name == "created_at" {
                def.push_str(" DEFAULT CURRENT_TIMESTAMP");
            }
            
            column_definitions.push(def);
            Ok(())
        }).map_err(|e| e.to_string())?;
        
        for _ in rows {
            // Esegui la query
        }
        
        // Aggiungi keys_project se non esiste già
        if !columns.contains(&"keys_project".to_string()) {
            columns.push("keys_project".to_string());
            column_definitions.push("keys_project TEXT".to_string());
        }
        
        // Crea tabella temporanea con TUTTE le colonne esistenti
        let temp_table = format!("{}_temp", project_name);
        let create_temp_sql = format!(
            "CREATE TABLE `{}` ({})",
            temp_table,
            column_definitions.join(", ")
        );
        conn.execute(&create_temp_sql, []).map_err(|e| e.to_string())?;
        
        // Copia TUTTI i dati dalla tabella originale
        let columns_list = columns.join(", ");
        let copy_sql = format!(
            "INSERT INTO `{}` ({}) SELECT {} FROM `{}`",
            temp_table, columns_list, columns_list, project_name
        );
        conn.execute(&copy_sql, []).map_err(|e| e.to_string())?;
        
        // Elimina la tabella originale
        let drop_sql = format!("DROP TABLE `{}`", project_name);
        conn.execute(&drop_sql, []).map_err(|e| e.to_string())?;
        
        // Rinomina la tabella temporanea
        let rename_sql = format!("ALTER TABLE `{}` RENAME TO `{}`", temp_table, project_name);
        conn.execute(&rename_sql, []).map_err(|e| e.to_string())?;
        
        println!("Schema tabella {} aggiornato con successo", project_name);
    }
    
    let mut imported_count = 0;
    let mut updated_count = 0;
    let mut skipped_count = 0;
    
    for key_info in keys {
        // Verifica se la chiave esiste già in qualsiasi campo (key o keys_project)
        let check_key_sql = format!("SELECT id FROM `{}` WHERE key = ? OR keys_project = ?", project_name);
        let key_exists = conn.query_row(&check_key_sql, [&key_info.key, &key_info.key], |_| Ok(())).is_ok();
        
        if key_exists {
            // Verifica se esiste già nella colonna keys_project
            let check_keys_project_sql = format!("SELECT id FROM `{}` WHERE keys_project = ?", project_name);
            let exists_in_keys_project = conn.query_row(&check_keys_project_sql, [&key_info.key], |_| Ok(())).is_ok();
            
            if !exists_in_keys_project {
                // Esiste nella colonna key ma non in keys_project, aggiorna con i file
                let files_json = serde_json::to_string(&key_info.all_files).unwrap_or_else(|_| "[]".to_string());
                let update_sql = format!(
                    "UPDATE `{}` SET keys_project = ?, key_files = ? WHERE key = ?",
                    project_name
                );
                conn.execute(&update_sql, [&key_info.key, &files_json, &key_info.key]).map_err(|e| e.to_string())?;
                updated_count += 1;
            } else {
                // Esiste già in keys_project, aggiorna solo i file se necessario
                let files_json = serde_json::to_string(&key_info.all_files).unwrap_or_else(|_| "[]".to_string());
                let update_files_sql = format!(
                    "UPDATE `{}` SET key_files = ? WHERE keys_project = ?",
                    project_name
                );
                conn.execute(&update_files_sql, [&files_json, &key_info.key]).map_err(|e| e.to_string())?;
                skipped_count += 1;
            }
        } else {
            // La chiave non esiste da nessuna parte, inserisci nuova riga con i file JSON
            let files_json = serde_json::to_string(&key_info.all_files).unwrap_or_else(|_| "[]".to_string());
            let insert_sql = format!(
                "INSERT INTO `{}` (keys_project, key_files) VALUES (?, ?)",
                project_name
            );
            conn.execute(&insert_sql, [&key_info.key, &files_json]).map_err(|e| e.to_string())?;
            imported_count += 1;
        }
    }
    
    let total_processed = imported_count + updated_count + skipped_count;
    Ok(format!(
        "Processate {} chiavi: {} nuove importate, {} aggiornate, {} saltate (già esistenti)", 
        total_processed, imported_count, updated_count, skipped_count
    ))
}

#[tauri::command]
fn get_project_keys(project_name: String) -> Result<Vec<String>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Controlla se la colonna keys_project esiste
    let check_column_sql = format!(
        "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name='keys_project'",
        project_name
    );
    
    let column_exists: i32 = conn.query_row(&check_column_sql, [], |row| {
        Ok(row.get::<_, i32>("count")?)
    }).unwrap_or(0);
    
    if column_exists == 0 {
        return Ok(vec![]); // Se la colonna non esiste, nessuna chiave
    }
    
    // Recupera tutte le chiavi keys_project non NULL
    let query_sql = format!(
        "SELECT keys_project FROM `{}` WHERE keys_project IS NOT NULL ORDER BY keys_project",
        project_name
    );
    
    let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
    let keys_iter = stmt.query_map([], |row| {
        row.get::<_, String>("keys_project")
    }).map_err(|e| e.to_string())?;
    
    let mut keys = Vec::new();
    for key_result in keys_iter {
        keys.push(key_result.map_err(|e| e.to_string())?);
    }
    
    Ok(keys)
}

#[derive(serde::Serialize)]
struct ImportedFile {
    id: i32,
    file_path: String,
    file_name: String,
    language_code: String,
    import_date: String,
    translations_count: i32,
}

// Helper function for internal use
pub fn get_imported_files_internal(project_name: String) -> std::result::Result<Vec<ImportedFile>, String> {
    // Protezione contro nomi di tabelle malformati con _imports ripetuti
    let original_name = project_name.clone(); // Clone per il debug
    let clean_project_name = if project_name.contains("_imports") {
        // Trova la prima occorrenza di _imports e taglia tutto dopo
        project_name.split("_imports").next().unwrap_or(&project_name).to_string()
    } else {
        project_name
    };
    
    println!("get_imported_files - Original: '{}', Clean: '{}'", original_name, clean_project_name);
    
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Crea la tabella imports se non esiste (usando il nome pulito)
    let create_imports_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}_imports` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            language_code TEXT NOT NULL,
            import_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            translations_count INTEGER NOT NULL,
            UNIQUE(file_path, language_code)
        )",
        clean_project_name
    );
    conn.execute(&create_imports_table_sql, []).map_err(|e| e.to_string())?;
    
    let query_sql = format!(
        "SELECT id, file_path, file_name, language_code, import_date, translations_count 
         FROM `{}_imports` 
         ORDER BY import_date DESC",
        clean_project_name
    );
    
    let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
    let file_iter = stmt.query_map([], |row| {
        Ok(ImportedFile {
            id: row.get("id")?,
            file_path: row.get("file_path")?,
            file_name: row.get("file_name")?,
            language_code: row.get("language_code")?,
            import_date: row.get("import_date")?,
            translations_count: row.get("translations_count")?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut files = Vec::new();
    for file in file_iter {
        files.push(file.map_err(|e| e.to_string())?);
    }
    
    Ok(files)
}

#[tauri::command]
fn get_imported_files(project_name: String) -> std::result::Result<Vec<ImportedFile>, String> {
    get_imported_files_internal(project_name)
}

#[derive(serde::Serialize)]
struct ProjectKeyDetail {
    key: String,
    exists_in_translations: bool,
}

#[tauri::command]
fn get_project_keys_with_status(project_name: String) -> Result<Vec<ProjectKeyDetail>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Controlla se la colonna keys_project esiste
    let check_column_sql = format!(
        "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name='keys_project'",
        project_name
    );
    
    let column_exists: i32 = conn.query_row(&check_column_sql, [], |row| {
        Ok(row.get::<_, i32>("count")?)
    }).unwrap_or(0);
    
    if column_exists == 0 {
        return Ok(vec![]); // Se la colonna non esiste, nessuna chiave
    }
    
    // Recupera tutte le chiavi keys_project con verifica se esistono in key
    let query_sql = format!(
        "SELECT 
            keys_project,
            CASE 
                WHEN EXISTS (SELECT 1 FROM `{}` t2 WHERE t2.key = t1.keys_project AND t2.key IS NOT NULL) 
                THEN 1 
                ELSE 0 
            END as exists_in_translations
         FROM `{}` t1 
         WHERE keys_project IS NOT NULL 
         ORDER BY keys_project",
        project_name, project_name
    );
    
    let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
    let keys_iter = stmt.query_map([], |row| {
        Ok(ProjectKeyDetail {
            key: row.get::<_, String>("keys_project")?,
            exists_in_translations: row.get::<_, i32>("exists_in_translations")? == 1,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut keys = Vec::new();
    for key_result in keys_iter {
        keys.push(key_result.map_err(|e| e.to_string())?);
    }
    
    Ok(keys)
}

#[tauri::command]
fn import_project_directory(directory_path: String) -> Result<String, String> {
    let path = Path::new(&directory_path);
    
    // Controlla che sia una directory
    if !path.is_dir() {
        return Err("Il percorso specificato non è una directory".to_string());
    }
    
    // Ottieni il nome della directory (sarà il nome del progetto/tabella)
    let project_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Nome directory non valido".to_string())?;
    
    // Sanitize project name: replace non-alphanumeric (except _) with _
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Crea tabella progetti se non esiste
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            path TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );", 
        []
    ).map_err(|e| e.to_string())?;

    // Inserisci il progetto nella tabella progetti
    conn.execute(
        "INSERT OR REPLACE INTO projects (name, path) VALUES (?, ?)",
        params![table_name, directory_path]
    ).map_err(|e| e.to_string())?;

    // Crea una tabella per questo progetto con struttura per traduzioni
    let create_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT UNIQUE NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
        table_name
    );
    conn.execute(&create_table_sql, []).map_err(|e| e.to_string())?;

    // Crea tabella per gestire le lingue del progetto
    conn.execute(
        "CREATE TABLE IF NOT EXISTS project_languages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_name TEXT NOT NULL,
            language_code TEXT NOT NULL,
            language_name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(project_name, language_code)
        );", 
        []
    ).map_err(|e| e.to_string())?;

    Ok(format!(
        "Progetto '{}' aggiunto con successo. Tabella '{}' creata.",
        project_name,
        table_name
    ))
}

#[tauri::command]
fn import_cht(filePath: String) -> Result<String, String> {
    // Estrarre nome tabella dal nome file (senza estensione)
    let path = Path::new(&filePath);
    let table_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("home.invalid_file_name".to_string())?;
    // Sanitize table name: replace non-alphanumeric (except _) with _
    let table_name: String = table_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Leggi il file .cht
    let content = read_text_file_best_effort(Path::new(&filePath)).map_err(|e| e.to_string())?;
    let mut lines = content.lines();
    // Skippa la prima riga (cheats = n)
    lines.next();

    // Parsing: raccogli tutti i cheat e i loro campi
    use std::collections::{BTreeSet, HashMap};
    let mut cheats: Vec<HashMap<String, String>> = Vec::new();
    let mut fields: BTreeSet<String> = BTreeSet::new();
    let mut current: HashMap<String, String> = HashMap::new();
    let mut last_idx = None;
    for line in lines {
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"');
            // Esempio: cheat0_desc
            if let Some(rest) = k.strip_prefix("cheat") {
                if let Some((idx, field)) = rest.split_once('_') {
                    if last_idx.is_some() && last_idx != Some(idx) {
                        // Nuovo cheat, pusha il precedente
                        cheats.push(current.clone());
                        current.clear();
                    }
                    last_idx = Some(idx);
                    current.insert(field.to_string(), v.to_string());
                    fields.insert(field.to_string());
                }
            }
        }
    }
    if !current.is_empty() {
        cheats.push(current);
    }

    // Aggiungi campo immagine
    // fields.insert("image".to_string()); // Non più necessario, immagini in table_images

    // Crea la tabella se non esiste
    let mut sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (id INTEGER PRIMARY KEY AUTOINCREMENT",
        table_name
    );
    for field in &fields {
        sql.push_str(&format!(", `{}` TEXT", field));
    }
    sql.push_str(");");
    conn.execute(&sql, []).map_err(|e| e.to_string())?;

    // Inserisci i record
    for cheat in &cheats {
        let mut columns = Vec::new();
        let mut values: Vec<String> = Vec::new();
        for field in &fields {
            columns.push(field.as_str());
            values.push(cheat.get(field).cloned().unwrap_or_default());
        }
        let placeholders = vec!["?".to_string(); columns.len()].join(", ");
        let quoted_columns = columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
        let insert_sql = format!(
            "INSERT INTO `{}` ({}) VALUES ({});",
            table_name,
            quoted_columns,
            placeholders
        );
        let params = params_from_iter(values.iter());
        conn.execute(&insert_sql, params)
            .map_err(|e| e.to_string())?;
    }

    Ok(format!(
        "Tabella '{}' creata/importata con {} record.",
        table_name,
        cheats.len()
    ))
}

#[tauri::command]
fn delete_table(table_name: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(&format!("DROP TABLE `{}`;", table_name), []).map_err(|e| e.to_string())?;
    
    // Elimina anche dalle tabelle correlate
    conn.execute("DELETE FROM table_images WHERE table_name = ?", [table_name.clone()]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM projects WHERE name = ?", [table_name.clone()]).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM project_languages WHERE project_name = ?", [table_name.clone()]).map_err(|e| e.to_string())?;
    
    Ok(format!("Tabella '{}' e tutti i record correlati eliminati.", table_name))
}

#[tauri::command]
fn open_url(url: String) -> Result<String, String> {
    std::process::Command::new("cmd")
        .args(&["/C", "start", &url])
        .spawn()
        .map_err(|e| format!("Errore nell'apertura dell'URL: {}", e))?;
    Ok("URL aperto.".to_string())
}

#[tauri::command]
fn remove_unused_keys(project_name: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Conta i record che hanno keys_project vuoto o NULL (chiavi inutilizzate)
    let count_unused_query = format!(
        "SELECT COUNT(*) FROM {} WHERE (keys_project IS NULL OR keys_project = '') AND (key IS NOT NULL AND key != '')", 
        table_name
    );
    
    let unused_count: i32 = conn.query_row(&count_unused_query, [], |row| {
        Ok(row.get::<_, i32>(0)?)
    }).map_err(|e| e.to_string())?;

    if unused_count == 0 {
        return Ok("Nessuna chiave inutilizzata trovata. Tutti i record hanno una corrispondenza nel progetto.".to_string());
    }

    // Elimina fisicamente i record che hanno keys_project vuoto o NULL
    // Questi sono record di traduzione che non hanno più una corrispondenza nei file del progetto
    let delete_query = format!(
        "DELETE FROM {} WHERE (keys_project IS NULL OR keys_project = '') AND (key IS NOT NULL AND key != '')", 
        table_name
    );
    
    let deleted_rows = conn.execute(&delete_query, []).map_err(|e| e.to_string())?;

    Ok(format!("Eliminate {} chiavi inutilizzate (record senza corrispondenza nel progetto).", deleted_rows))
}

// Helper function for internal use
pub fn get_setting_internal(key: String) -> std::result::Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);", []).map_err(|e| e.to_string())?;
    let value: Option<String> = conn.query_row("SELECT value FROM settings WHERE key = ?", [key], |row| row.get(0)).unwrap_or(None);
    Ok(value.unwrap_or_default())
}

#[tauri::command]
fn get_setting(key: String) -> std::result::Result<String, String> {
    get_setting_internal(key)
}

#[tauri::command]
fn set_setting(key: String, value: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);", []).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)", params![key, value]).map_err(|e| e.to_string())?;
    Ok("Impostazione salvata.".to_string())
}

#[tauri::command]
fn update_record_order(tableName: String, recordOrder: Vec<String>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Add order_index column if it doesn't exist and initialize existing records
    let alter_sql = format!("ALTER TABLE `{}` ADD COLUMN order_index INTEGER;", tableName);
    match conn.execute(&alter_sql, []) {
        Ok(_) => {
            // Column was added, initialize existing records with their current order (by id)
            conn.execute(&format!("UPDATE `{}` SET order_index = id;", tableName), []).map_err(|e| e.to_string())?;
        }
        Err(_) => {
            // Column already exists, continue
        }
    }

    // Update order_index for each record by id (recordOrder contains ids as strings)
    for (index, id_str) in recordOrder.iter().enumerate() {
        let id_num: i64 = id_str.parse::<i64>().map_err(|e: std::num::ParseIntError| e.to_string())?;
        conn.execute(
            &format!("UPDATE `{}` SET order_index = ? WHERE id = ?", tableName),
            params![index as i32, id_num]
        ).map_err(|e| e.to_string())?;
    }

    Ok("Ordine aggiornato.".to_string())
}

// Helper function for internal use
pub fn detect_file_encoding_internal(file_path: String) -> std::result::Result<String, String> {
    // Read the file as raw bytes
    let bytes = fs::read(&file_path).map_err(|e| format!("Errore lettura file: {}", e))?;
    
    // Try to decode as UTF-8 first
    let utf8_result = UTF_8.decode_without_bom_handling(&bytes);
    if utf8_result.1 {
        // Successfully decoded as UTF-8
        return Ok("UTF-8".to_string());
    }
    
    // Try to decode as Windows-1252
    let windows1252_result = WINDOWS_1252.decode_without_bom_handling(&bytes);
    if windows1252_result.1 {
        // Successfully decoded as Windows-1252
        return Ok("Windows-1252".to_string());
    }
    
    // If neither works perfectly, check which one produces fewer replacement characters
    let utf8_text = utf8_result.0;
    let windows1252_text = windows1252_result.0;
    
    // Count replacement characters (�) in each
    let utf8_replacements = utf8_text.chars().filter(|&c| c == '�').count();
    let windows1252_replacements = windows1252_text.chars().filter(|&c| c == '�').count();
    
    if utf8_replacements <= windows1252_replacements {
        Ok("UTF-8".to_string())
    } else {
        Ok("Windows-1252".to_string())
    }
}

#[tauri::command]
fn detect_file_encoding(file_path: String) -> std::result::Result<String, String> {
    detect_file_encoding_internal(file_path)
}

#[tauri::command]
fn export_cht_to_path(table_name: String, file_path: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let _conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Get all records from the table
    let records = get_records(table_name.clone())?;

    if records.is_empty() {
        return Err("home.table_empty".to_string());
    }

    // Get column names (excluding id and image)
    let columns = get_table_columns(table_name.clone())?;
    let export_columns: Vec<String> = columns.into_iter()
        .filter(|col| col != "id" && col != "image")
        .collect();

    // Create CHT content
    let mut content = format!("cheats = {}\n\n", records.len());

    for (i, record) in records.iter().enumerate() {
        for col in &export_columns {
            if let Some(value) = record.get(col) {
                // Escape quotes in value
                let escaped_value = value.replace("\"", "\\\"");
                content.push_str(&format!("cheat{}_{} = \"{}\"\n", i, col, escaped_value));
            }
        }
        // Add empty line between cheats (except for the last one)
        if i < records.len() - 1 {
            content.push_str("\n");
        }
    }

    // Write to file
    fs::write(&file_path, content).map_err(|e| format!("Errore nella scrittura del file: {}", e))?;

    // Try to open the directory containing the saved file
    if let Some(parent_dir) = Path::new(&file_path).parent() {
        if let Err(e) = std::process::Command::new("explorer").arg(parent_dir).spawn() {
            println!("Could not open explorer: {}", e);
        }
    }

    Ok(format!("File esportato con successo: {}", file_path))
}

#[tauri::command]
fn get_export_preview(table_name: String) -> Result<serde_json::Value, String> {
    // Retrieve project info (path)
    let info = get_table_info(table_name.clone())?;
    let project_path = info.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if project_path.is_empty() {
        return Err("Percorso progetto non disponibile".to_string());
    }

    // Find .hmiprj or .movprj file in project path to get the project name
    let mut project_name = table_name.clone(); // fallback to table name
    for entry in fs::read_dir(&project_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let file_name_lower = file_name.to_lowercase();
                if file_name_lower.ends_with(".hmiprj") || file_name_lower.ends_with(".movprj") {
                    // Extract project name without extension
                    if let Some(name_without_ext) = file_name.strip_suffix(".hmiprj")
                        .or_else(|| file_name.strip_suffix(".HMIPRJ"))
                        .or_else(|| file_name.strip_suffix(".movprj"))
                        .or_else(|| file_name.strip_suffix(".MOVPRJ")) {
                        project_name = name_without_ext.to_string();
                        break;
                    }
                }
            }
        }
    }

    // Get language columns
    let columns = get_table_columns(table_name.clone())?;
    let export_columns: Vec<String> = columns.into_iter()
        .filter(|col| col != "id" && col != "image" && col != "key" && col != "keys_project" && col != "key_files" && col != "order_index")
        .collect();

    // Generate export file names (deduplicated by extension)
    let mut extensions = std::collections::HashSet::new();
    let mut export_files = Vec::new();
    
    for lang in &export_columns {
        let ext = match lang.as_str() {
            "en" | "eng" => "eng",
            "it" | "ita" => "ita",
            "fr" | "fra" | "fre" => "fra",
            "de" | "deu" | "ger" => "deu",
            "es" | "esp" | "spa" => "esp",
            _ => "eng",
        };
        
        // Only add if we haven't seen this extension before
        if extensions.insert(ext.to_string()) {
            let file_name = format!("{}string.{}", project_name, ext);
            export_files.push(file_name);
        }
    }

    // Find files that will be backed up
    let mut backup_files = Vec::new();
    let mut file_encodings = std::collections::HashMap::new();
    
    for entry in fs::read_dir(&project_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let file_name_lower = file_name.to_lowercase();
                
                // Check if it's a translation file that will be backed up
                let is_translation_file = 
                    file_name_lower.ends_with(".eng") ||
                    file_name_lower.ends_with(".ita") ||
                    file_name_lower.ends_with(".fra") ||
                    file_name_lower.ends_with(".fre") ||
                    file_name_lower.ends_with(".deu") ||
                    file_name_lower.ends_with(".ger") ||
                    file_name_lower.ends_with(".esp") ||
                    file_name_lower.ends_with(".spa") ||
                    (file_name_lower.contains("string") && 
                     (file_name_lower.ends_with(".xml") || file_name_lower.ends_with(".txt")));
                
                if is_translation_file {
                    backup_files.push(file_name.to_string());
                    
                    // Detect encoding for this file
                    let encoding = detect_file_encoding(path.to_string_lossy().to_string())
                        .unwrap_or("UTF-8".to_string());
                    file_encodings.insert(file_name.to_string(), encoding);
                }
            }
        }
    }

    Ok(serde_json::json!({
        "projectName": project_name,
        "projectPath": project_path,
        "exportFiles": export_files,
        "backupFiles": backup_files,
        "fileEncodings": file_encodings,
        "languageCount": extensions.len(),
        // Debug: include detected columns so we can diagnose missing languages
        "columns": export_columns
    }))
}

#[derive(Deserialize)]
struct GamesResponse {
    data: GamesData,
}

#[derive(Deserialize)]
struct GamesData {
    games: Vec<Game>,
}

#[derive(Deserialize)]
struct Game {
    id: u32,
    name: String,
}

#[derive(Deserialize)]
struct ImagesResponse {
    data: ImagesData,
}

#[derive(Deserialize)]
struct ImagesData {
    base_url: BaseUrl,
    images: HashMap<String, Vec<Image>>,
}

#[derive(Deserialize)]
struct BaseUrl {
    original: String,
}

#[derive(Deserialize)]
struct RawgResponse {
    results: Vec<RawgGame>,
}

#[derive(Deserialize)]
struct RawgGame {
    id: u32,
    name: String,
    background_image: Option<String>,
}

#[derive(Deserialize)]
struct Image {
    id: u32,
    #[serde(rename = "type")]
    image_type: String,
    filename: String,
}

#[tauri::command]
async fn translate_text(text: String, source_lang: String, target_lang: String) -> Result<String, String> {
    let client = Client::new();
    
    // Carica le impostazioni
    let service = get_setting("translation_service".to_string()).unwrap_or("deepl".to_string());
    let api_key = match service.as_str() {
        "deepl" => get_setting("deepl_api_key".to_string()),
        "google" => get_setting("google_api_key".to_string()),
        "microsoft" => get_setting("microsoft_api_key".to_string()),
        _ => return Err("Servizio di traduzione non supportato".to_string()),
    };
    
    let api_key = api_key.map_err(|_| "Chiave API non configurata".to_string())?;
    let region = if service == "microsoft" {
        Some(get_setting("microsoft_region".to_string()).unwrap_or("westeurope".to_string()))
    } else {
        None
    };
    
    match service.as_str() {
        "deepl" => {
            let source_language = source_lang.to_uppercase();
            let target_language = if target_lang.to_uppercase() == "EN" { "EN-US".to_string() } else { target_lang.to_uppercase() };
            
            let params = vec![
                ("text", text.as_str()),
                ("source_lang", &source_language),
                ("target_lang", &target_language),
            ];
            
            let response = client
                .post("https://api-free.deepl.com/v2/translate")
                .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
                .header("Content-Type", "application/x-www-form-urlencoded")
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Errore connessione DeepL: {}", e))?;
            
            if !response.status().is_success() {
                let status_code = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(format!("DeepL API Error {}: {}", status_code, error_text));
            }
            
            let json: serde_json::Value = response.json().await
                .map_err(|e| format!("Errore parsing risposta DeepL: {}", e))?;
            
            let translated_text = json["translations"][0]["text"]
                .as_str()
                .ok_or("Risposta DeepL non valida")?
                .to_string();
                
            Ok(translated_text)
        },
        "google" => {
            let request_body = serde_json::json!({
                "q": text,
                "source": source_lang,
                "target": target_lang,
                "format": "text"
            });
            
            let response = client
                .post(&format!("https://translation.googleapis.com/language/translate/v2?key={}", api_key))
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| format!("Errore connessione Google: {}", e))?;
            
            if !response.status().is_success() {
                let status_code = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(format!("Google Translate API Error {}: {}", status_code, error_text));
            }
            
            let json: serde_json::Value = response.json().await
                .map_err(|e| format!("Errore parsing risposta Google: {}", e))?;
            
            let translated_text = json["data"]["translations"][0]["translatedText"]
                .as_str()
                .ok_or("Risposta Google non valida")?
                .to_string();
                
            Ok(translated_text)
        },
        "microsoft" => {
            let region = region.unwrap_or_else(|| "westeurope".to_string());
            let request_body = serde_json::json!([{
                "text": text
            }]);
            
            let response = client
                .post(&format!("https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&from={}&to={}", source_lang, target_lang))
                .header("Ocp-Apim-Subscription-Key", api_key)
                .header("Ocp-Apim-Subscription-Region", region)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
                .map_err(|e| format!("Errore connessione Microsoft: {}", e))?;
            
            if !response.status().is_success() {
                let status_code = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(format!("Microsoft Translator API Error {}: {}", status_code, error_text));
            }
            
            let json: serde_json::Value = response.json().await
                .map_err(|e| format!("Errore parsing risposta Microsoft: {}", e))?;
            
            let translated_text = json[0]["translations"][0]["text"]
                .as_str()
                .ok_or("Risposta Microsoft non valida")?
                .to_string();
                
            Ok(translated_text)
        },
        _ => Err(format!("Servizio di traduzione non supportato: {}", service))
    }
}

#[tauri::command]
fn export_database(file_path: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    
    // Verifica che il database esista
    if !Path::new(db_path).exists() {
        return Err("Database non trovato".to_string());
    }
    
    // Copia il file del database nella posizione specificata
    fs::copy(db_path, &file_path).map_err(|e| format!("Errore durante l'esportazione: {}", e))?;
    
    Ok(format!("Database esportato con successo in: {}", file_path))
}

#[tauri::command]
fn import_database(file_path: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    
    // Verifica che il file di importazione esista
    if !Path::new(&file_path).exists() {
        return Err("File di importazione non trovato".to_string());
    }
    
    // Crea backup del database attuale
    let backup_path = format!("{}.backup", db_path);
    if Path::new(db_path).exists() {
        fs::copy(db_path, &backup_path).map_err(|e| format!("Errore durante la creazione del backup: {}", e))?;
    }
    
    // Copia il file importato come nuovo database
    fs::copy(&file_path, db_path).map_err(|e| format!("Errore durante l'importazione: {}", e))?;
    
    Ok(format!("Database importato con successo. Backup creato in: {}", backup_path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![import_cht, import_project_directory, get_tables, delete_table, get_records, set_table_image, delete_table_image, update_record, delete_record, insert_record, get_table_columns, get_table_info, fetch_and_set_logo, get_setting, set_setting, open_url, export_cht_to_path, get_export_preview, export_translations_per_language, export_translations_to_machine, update_record_order, add_language_to_project, get_project_languages, remove_language_from_project, import_translation_file, get_translation_files_in_directory, get_alarm_files_in_directory, import_alarm_file_from_path, get_alarm_imported_files, delete_alarm, delete_alarms_database, add_alarm, update_alarm, get_alarm, get_translation_keys, export_alarms, export_alarms_to_machine, find_keys_in_project, import_project_keys, get_project_keys, get_project_keys_with_status, import_translation_file_from_path, get_imported_files, translate_text, remove_unused_keys, detect_file_encoding, get_variable_files_in_directory, import_variable_file, get_variable_imported_files, get_variables, get_structures, get_structure_members, delete_variable, update_variable, export_database, import_database])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
