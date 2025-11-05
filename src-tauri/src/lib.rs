use rusqlite::{params_from_iter, params, Connection, Result};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
extern crate regex;

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

#[tauri::command]
fn get_records(tableName: String) -> Result<Vec<std::collections::HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Get column names
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", tableName)).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.query_map([], |row| Ok(row.get::<_, String>(1)?)).map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    
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

#[tauri::command]
fn get_table_info(table_name: String) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut result = std::collections::HashMap::new();
    
    // Ottieni le colonne della tabella
    let columns = get_table_columns(table_name.clone())?;
    result.insert("columns".to_string(), serde_json::json!(columns));
    
    // Ottieni le lingue del progetto se esistono
    match get_project_languages(table_name.clone()) {
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
fn get_table_columns(tableName: String) -> Result<Vec<String>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", tableName)).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.query_map([], |row| Ok(row.get::<_, String>(1)?)).map_err(|e| e.to_string())?.collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    Ok(columns)
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

fn parse_translation_file_content(file_path: &Path) -> Result<std::collections::HashMap<String, String>, String> {
    let content = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    let mut translations = std::collections::HashMap::new();
    
    // Try to parse as XML first
    if content.contains("<?xml") && content.contains("<strings>") {
        // Parse XML format
        for line in content.lines() {
            if line.contains("<item key=") {
                if let Some(key_start) = line.find("key=\"") {
                    if let Some(key_end) = line[key_start + 5..].find("\"") {
                        let key = &line[key_start + 5..key_start + 5 + key_end];
                        
                        if let Some(value_start) = line.find("value=\"") {
                            if let Some(value_end) = line[value_start + 7..].find("\"") {
                                let value = &line[value_start + 7..value_start + 7 + value_end];
                                // Unescape XML entities
                                let unescaped_key = key.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'");
                                let unescaped_value = value.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'");
                                translations.insert(unescaped_key, unescaped_value);
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Parse key=value format
        for line in content.lines() {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim().to_string();
                translations.insert(key, value);
            }
        }
    }
    
    Ok(translations)
}

fn check_if_content_already_imported(file_path: &Path, table_name: &str, language_column: &str) -> Result<bool, String> {
    let file_translations = parse_translation_file_content(file_path)?;
    
    if file_translations.is_empty() {
        return Ok(false);
    }
    
    // Get database content for this language
    let db_path = "../data/database.db";
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(&format!("SELECT key, {} FROM `{}`", language_column, table_name)).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get("key")?;
        let value: Option<String> = row.get(language_column).unwrap_or(None);
        Ok((key, value.unwrap_or_default()))
    }).map_err(|e| e.to_string())?;
    
    let mut db_translations = std::collections::HashMap::new();
    for row in rows {
        let (key, value) = row.map_err(|e| e.to_string())?;
        if !value.is_empty() {
            db_translations.insert(key, value);
        }
    }
    
    // Check if file content matches database content
    if file_translations.len() != db_translations.len() {
        return Ok(false);
    }
    
    for (key, file_value) in &file_translations {
        if let Some(db_value) = db_translations.get(key) {
            if file_value != db_value {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    
    Ok(true)
}

#[tauri::command]
fn get_translation_files_in_directory(directory_path: String, table_name: String) -> Result<Vec<std::collections::HashMap<String, String>>, String> {
    let path = Path::new(&directory_path);
    
    if !path.is_dir() {
        return Err("Il percorso specificato non è una directory".to_string());
    }
    
    let mut translation_files = Vec::new();
    
    // Scansiona i file nella directory
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();
        
        if file_path.is_file() {
            if let Some(file_name) = file_path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    let file_name_lower = file_name_str.to_lowercase();
                    
                    // Mappa delle estensioni alle lingue
                    let language_name = if file_name_lower.ends_with(".ita") {
                        Some("Italiano".to_string())
                    } else if file_name_lower.ends_with(".eng") {
                        Some("English".to_string())
                    } else if file_name_lower.ends_with(".fra") || file_name_lower.ends_with(".fre") {
                        Some("Français".to_string())
                    } else if file_name_lower.ends_with(".deu") || file_name_lower.ends_with(".ger") {
                        Some("Deutsch".to_string())
                    } else if file_name_lower.ends_with(".esp") || file_name_lower.ends_with(".spa") {
                        Some("Español".to_string())
                    } else {
                        None
                    };
                    
                    if let Some(lang_name) = language_name {
                        let mut file_info = std::collections::HashMap::new();
                        file_info.insert("file_name".to_string(), file_name_str.to_string());
                        file_info.insert("file_path".to_string(), file_path.to_string_lossy().to_string());
                        file_info.insert("language_name".to_string(), lang_name);
                        
                        // Estrai il codice lingua dall'estensione
                        let language_code = if file_name_lower.ends_with(".ita") {
                            "it"
                        } else if file_name_lower.ends_with(".eng") {
                            "en"
                        } else if file_name_lower.ends_with(".fra") || file_name_lower.ends_with(".fre") {
                            "fr"
                        } else if file_name_lower.ends_with(".deu") || file_name_lower.ends_with(".ger") {
                            "de"
                        } else if file_name_lower.ends_with(".esp") || file_name_lower.ends_with(".spa") {
                            "es"
                        } else {
                            "unknown"
                        };
                        
                        file_info.insert("language_code".to_string(), language_code.to_string());
                        
                        // Check if content is already imported
                        let mut already_imported = false;
                        
                        // Find matching column in database
                        if let Ok(columns) = get_table_columns(table_name.clone()) {
                            for column in &columns {
                                if column == language_code || 
                                   (language_code == "en" && (column == "eng" || column == "en")) ||
                                   (language_code == "it" && (column == "ita" || column == "it")) ||
                                   (language_code == "fr" && (column == "fra" || column == "fre" || column == "fr")) ||
                                   (language_code == "de" && (column == "deu" || column == "ger" || column == "de")) ||
                                   (language_code == "es" && (column == "esp" || column == "spa" || column == "es")) {
                                    
                                    if let Ok(is_imported) = check_if_content_already_imported(&file_path, &table_name, column) {
                                        if is_imported {
                                            already_imported = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        
                        file_info.insert("already_imported".to_string(), already_imported.to_string());
                        
                        // Only add to list if not already imported or if we want to show all files
                        if !already_imported {
                            translation_files.push(file_info);
                        }
                    }
                }
            }
        }
    }
    
    Ok(translation_files)
}

#[derive(serde::Serialize, serde::Deserialize)]
struct KeyWithFile {
    key: String,
    file: String,
    full_line: String,
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
                    let content = match fs::read_to_string(&file_path) {
                        Ok(c) => c,
                        Err(_) => {
                            let bytes = fs::read(&file_path).map_err(|e| e.to_string())?;
                            if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
                                let utf16: Vec<u16> = bytes[2..]
                                    .chunks_exact(2)
                                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                                    .collect();
                                String::from_utf16(&utf16).unwrap_or_default()
                            } else {
                                String::from_utf8_lossy(&bytes).into_owned()
                            }
                        }
                    };

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
    
    // Ordina per chiave
    found_keys.sort_by(|a, b| a.key.cmp(&b.key));
    
    Ok(found_keys)
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
                // Esiste nella colonna key ma non in keys_project, aggiorna
                let update_sql = format!(
                    "UPDATE `{}` SET keys_project = ? WHERE key = ?",
                    project_name
                );
                conn.execute(&update_sql, [&key_info.key, &key_info.key]).map_err(|e| e.to_string())?;
                updated_count += 1;
            } else {
                // Esiste già in keys_project, salta l'importazione
                skipped_count += 1;
            }
        } else {
            // La chiave non esiste da nessuna parte, inserisci nuova riga
            let insert_sql = format!(
                "INSERT INTO `{}` (keys_project) VALUES (?)",
                project_name
            );
            conn.execute(&insert_sql, [&key_info.key]).map_err(|e| e.to_string())?;
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

#[tauri::command]
fn import_translation_file_from_path(table_name: String, language_code: String, file_path: String) -> Result<String, String> {
    // Leggi il file dal filesystem
    let content = std::fs::read_to_string(&file_path).map_err(|e| format!("Errore lettura file {}: {}", file_path, e))?;
    
    // Usa la funzione esistente per importare il contenuto
    import_translation_file_with_merge(table_name, language_code, content, file_path)
}

#[tauri::command]
fn import_translation_file_with_merge(table_name: String, language_code: String, xml_content: String, file_path: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Assicurati che la colonna per la lingua esista
    let column_name = language_code.clone();
    let check_column_sql = format!(
        "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name='{}'",
        table_name, column_name
    );
    
    let column_exists: i32 = conn.query_row(&check_column_sql, [], |row| {
        Ok(row.get::<_, i32>("count")?)
    }).unwrap_or(0);
    
    if column_exists == 0 {
        let add_column_sql = format!("ALTER TABLE `{}` ADD COLUMN `{}` TEXT", table_name, column_name);
        conn.execute(&add_column_sql, []).map_err(|e| e.to_string())?;
    }
    
    // Assicurati che la colonna keys_project esista
    let check_keys_project_sql = format!(
        "SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name='keys_project'",
        table_name
    );
    
    let keys_project_exists: i32 = conn.query_row(&check_keys_project_sql, [], |row| {
        Ok(row.get::<_, i32>("count")?)
    }).unwrap_or(0);
    
    if keys_project_exists == 0 {
        let add_keys_project_sql = format!("ALTER TABLE `{}` ADD COLUMN keys_project TEXT", table_name);
        conn.execute(&add_keys_project_sql, []).map_err(|e| e.to_string())?;
    }
    
    // Crea tabella per tracciare i file importati se non esiste
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
        table_name
    );
    conn.execute(&create_imports_table_sql, []).map_err(|e| e.to_string())?;
    
    // Debug: mostra le prime righe del file per capire il formato (commentato per produzione)
    // println!("Contenuto file (prime 10 righe):");
    // for (i, line) in xml_content.lines().take(10).enumerate() {
    //     println!("Riga {}: {}", i + 1, line);
    // }
    
    // Parsa l'XML e estrai le traduzioni
    let mut translations = std::collections::HashMap::new();
    
    // Parsing per diversi formati XML
    let lines: Vec<&str> = xml_content.lines().collect();
    for line in lines {
        let trimmed = line.trim();
        
        // Formato 1: <string id="KEY">VALORE</string>
        if trimmed.starts_with("<string id=\"") && trimmed.contains("</string>") {
            if let Some(id_start) = trimmed.find("id=\"") {
                if let Some(id_end) = trimmed[id_start + 4..].find("\"") {
                    let key = &trimmed[id_start + 4..id_start + 4 + id_end];
                    
                    if let Some(content_start) = trimmed.find(">") {
                        if let Some(content_end) = trimmed.rfind("</string>") {
                            let content = &trimmed[content_start + 1..content_end];
                            translations.insert(key.to_string(), content.to_string());
                            // println!("Trovata traduzione formato 1: {} = {}", key, content);
                        }
                    }
                }
            }
        }
        // Formato 2: <item key="CHIAVE" value="VALORE"/>
        else if trimmed.starts_with("<item key=\"") && trimmed.contains("value=\"") {
            // Estrai la chiave
            if let Some(key_start) = trimmed.find("key=\"") {
                if let Some(key_end) = trimmed[key_start + 5..].find("\"") {
                    let key = &trimmed[key_start + 5..key_start + 5 + key_end];
                    
                    // Estrai il valore
                    if let Some(value_start) = trimmed.find("value=\"") {
                        if let Some(value_end) = trimmed[value_start + 7..].find("\"") {
                            let value = &trimmed[value_start + 7..value_start + 7 + value_end];
                            translations.insert(key.to_string(), value.to_string());
                            // println!("Trovata traduzione formato 2: {} = {}", key, value);
                        }
                    }
                }
            }
        }
        // Formato 3: chiave tra > e </text> (come hai detto prima)
        else if trimmed.contains(">") && trimmed.contains("</text>") {
            if let Some(content_start) = trimmed.find(">") {
                if let Some(content_end) = trimmed.rfind("</text>") {
                    let key = &trimmed[content_start + 1..content_end];
                    // Per ora usiamo la chiave come valore, poi dovremo capire dove trovare la traduzione
                    translations.insert(key.to_string(), key.to_string());
                    // println!("Trovata chiave formato 3: {}", key);
                }
            }
        }
    }
    
    // println!("Totale traduzioni trovate: {}", translations.len());
    
    let mut imported_count = 0;
    let mut updated_count = 0;
    
    for (key, translation) in translations {
        // Prima controlla se esiste un record con questa chiave in keys_project
        let check_keys_project_sql = format!(
            "SELECT id, key FROM `{}` WHERE keys_project = ?",
            table_name
        );
        
        let found_in_keys_project = conn.query_row(&check_keys_project_sql, [&key], |row| {
            Ok((
                row.get::<_, i32>("id")?,
                row.get::<_, Option<String>>("key")?
            ))
        });
        
        match found_in_keys_project {
            Ok((id, existing_key)) => {
                // Trovato in keys_project, aggiorna il record
                // Se key è vuota, aggiungi la chiave del file, altrimenti mantieni quella esistente
                let final_key = match existing_key {
                    Some(ref k) if !k.is_empty() => k.clone(),
                    _ => key.clone()
                };
                
                let update_sql = format!(
                    "UPDATE `{}` SET key = ?, `{}` = ? WHERE id = ?",
                    table_name, column_name
                );
                conn.execute(&update_sql, [&final_key, &translation, &id.to_string()]).map_err(|e| e.to_string())?;
                updated_count += 1;
            }
            Err(_) => {
                // Non trovato in keys_project, controlla se esiste già in key
                let check_key_sql = format!(
                    "SELECT id FROM `{}` WHERE key = ?",
                    table_name
                );
                
                match conn.query_row(&check_key_sql, [&key], |row| {
                    Ok(row.get::<_, i32>("id")?)
                }) {
                    Ok(id) => {
                        // Trovato in key, aggiorna solo la traduzione
                        let update_sql = format!(
                            "UPDATE `{}` SET `{}` = ? WHERE id = ?",
                            table_name, column_name
                        );
                        conn.execute(&update_sql, [&translation, &id.to_string()]).map_err(|e| e.to_string())?;
                        updated_count += 1;
                    }
                    Err(_) => {
                        // Chiave non presente in nessuna colonna, crea un nuovo record
                        // Questa chiave proviene dal file di traduzione e deve essere importata
                        let insert_sql = format!(
                            "INSERT INTO `{}` (key, `{}`) VALUES (?, ?)",
                            table_name, column_name
                        );
                        conn.execute(&insert_sql, [&key, &translation]).map_err(|e| e.to_string())?;
                        imported_count += 1;
                    }
                }
            }
        }
    }
    
    // Registra l'importazione nella tabella di tracking (se abbiamo importato qualcosa)
    if imported_count > 0 || updated_count > 0 {
        let total_translations = imported_count + updated_count;
        let insert_import_sql = format!(
            "INSERT OR REPLACE INTO `{}_imports` (file_path, file_name, language_code, translations_count) VALUES (?, ?, ?, ?)",
            table_name
        );
        
        // Estrai il nome del file dal path
        let file_name = std::path::Path::new(&file_path).file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
            
        conn.execute(&insert_import_sql, [&file_path, &file_name, &language_code, &total_translations.to_string()]).map_err(|e| e.to_string())?;
    }
    
    Ok(format!("Importate {} nuove traduzioni, aggiornate {} traduzioni esistenti per la lingua {}", imported_count, updated_count, language_code))
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

#[tauri::command]
fn get_imported_files(project_name: String) -> Result<Vec<ImportedFile>, String> {
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
fn add_language_to_project(project_name: String, language_code: String, language_name: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    
    // Aggiungi la lingua alla tabella project_languages
    conn.execute(
        "INSERT OR IGNORE INTO project_languages (project_name, language_code, language_name) VALUES (?, ?, ?)",
        params![table_name, language_code, language_name]
    ).map_err(|e| e.to_string())?;

    // Aggiungi la colonna alla tabella del progetto
    let alter_sql = format!("ALTER TABLE `{}` ADD COLUMN `{}` TEXT DEFAULT '';", table_name, language_code);
    match conn.execute(&alter_sql, []) {
        Ok(_) => {
            Ok(format!("Lingua '{}' ({}) aggiunta al progetto '{}'", language_name, language_code, project_name))
        }
        Err(e) => {
            // Se la colonna esiste già, ignora l'errore
            if e.to_string().contains("duplicate column name") {
                Ok(format!("Lingua '{}' ({}) già presente nel progetto '{}'", language_name, language_code, project_name))
            } else {
                Err(e.to_string())
            }
        }
    }
}

#[tauri::command]
fn get_project_languages(project_name: String) -> Result<Vec<std::collections::HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Get all language columns from the project table (excluding system columns)
    let columns = get_table_columns(table_name.clone())?;
    let language_columns: Vec<String> = columns.into_iter()
        .filter(|col| col != "id" && col != "image" && col != "key" && col != "keys_project" && col != "order_index" && col != "created_at")
        .collect();

    let mut languages = Vec::new();
    
    // For each language column, try to get the name from project_languages table, or use the column name
    for lang_code in language_columns {
        let mut language_name = lang_code.clone(); // Default to column name
        
        // Try to get the proper name from project_languages table
        if let Ok(name) = conn.query_row(
            "SELECT language_name FROM project_languages WHERE project_name = ? AND language_code = ?",
            [&table_name, &lang_code],
            |row| row.get::<_, String>(0)
        ) {
            language_name = name;
        } else {
            // Generate a friendly name based on the column name
            language_name = match lang_code.as_str() {
                "en" | "eng" => "English".to_string(),
                "it" | "ita" => "Italiano".to_string(),
                "fr" | "fra" | "fre" => "Français".to_string(),
                "de" | "deu" | "ger" => "Deutsch".to_string(),
                "es" | "esp" | "spa" => "Español".to_string(),
                _ => {
                    // Capitalize first letter
                    let mut chars = lang_code.chars();
                    match chars.next() {
                        None => lang_code.clone(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                }
            };
        }
        
        let mut map = std::collections::HashMap::new();
        map.insert("code".to_string(), lang_code);
        map.insert("name".to_string(), language_name);
        languages.push(map);
    }
    
    // Sort by code
    languages.sort_by(|a, b| a.get("code").unwrap().cmp(b.get("code").unwrap()));
    
    Ok(languages)
}

#[tauri::command]
fn import_translation_file(project_name: String, language_code: String, file_path: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Leggi e parsa il file XML
    let content = fs::read_to_string(&file_path).map_err(|e| format!("Errore lettura file: {}", e))?;
    
    // Parsing XML semplice per estrarre le coppie key-value
    let mut translations = std::collections::HashMap::new();
    
    // Cerca tutti gli elementi <item key="..." value="..."/>
    let re = regex::Regex::new(r#"<item\s+key="([^"]+)"\s+value="([^"]*)"/>"#).map_err(|e| e.to_string())?;
    
    for cap in re.captures_iter(&content) {
        let key = cap.get(1).unwrap().as_str();
        let value = cap.get(2).unwrap().as_str();
        translations.insert(key.to_string(), value.to_string());
    }

    if translations.is_empty() {
        return Err("Nessuna traduzione trovata nel file XML".to_string());
    }

    // Aggiungi la lingua se non esiste
    let lang_name = match language_code.as_str() {
        "eng" | "en" => "English",
        "ita" | "it" => "Italiano", 
        "fra" | "fr" => "Français",
        "deu" | "de" => "Deutsch",
        "esp" | "es" => "Español",
        _ => &language_code
    };

    // Inserisci la lingua nella tabella project_languages
    conn.execute(
        "INSERT OR IGNORE INTO project_languages (project_name, language_code, language_name) VALUES (?, ?, ?)",
        params![table_name, language_code, lang_name]
    ).map_err(|e| e.to_string())?;

    // Aggiungi la colonna alla tabella del progetto se non esiste
    let alter_sql = format!("ALTER TABLE `{}` ADD COLUMN `{}` TEXT DEFAULT '';", table_name, language_code);
    let _ = conn.execute(&alter_sql, []); // Ignora l'errore se la colonna esiste già

    // Inserisci/aggiorna le traduzioni
    let mut inserted = 0;
    let mut updated = 0;

    for (key, value) in translations {
        // Controlla se la chiave esiste già
        let exists: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM `{}` WHERE key = ?", table_name),
            [&key],
            |row| row.get(0)
        ).unwrap_or(0);

        if exists > 0 {
            // Aggiorna il record esistente
            conn.execute(
                &format!("UPDATE `{}` SET `{}` = ? WHERE key = ?", table_name, language_code),
                params![value, key]
            ).map_err(|e| e.to_string())?;
            updated += 1;
        } else {
            // Inserisci nuovo record
            conn.execute(
                &format!("INSERT INTO `{}` (key, `{}`) VALUES (?, ?)", table_name, language_code),
                params![key, value]
            ).map_err(|e| e.to_string())?;
            inserted += 1;
        }
    }

    Ok(format!(
        "Importazione completata: {} nuove chiavi, {} aggiornate per la lingua '{}'",
        inserted, updated, lang_name
    ))
}

#[tauri::command]
fn remove_language_from_project(project_name: String, language_code: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Verifica che la tabella esista
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&table_name],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);

    if !table_exists {
        return Err(format!("Il progetto '{}' non esiste", project_name));
    }

    // Verifica che la colonna lingua esista nella tabella del progetto
    let mut column_exists = false;
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`)", table_name)).map_err(|e| e.to_string())?;
    let column_iter = stmt.query_map([], |row| {
        let column_name: String = row.get(1)?;
        Ok(column_name)
    }).map_err(|e| e.to_string())?;

    for column_result in column_iter {
        if let Ok(column_name) = column_result {
            if column_name == language_code {
                column_exists = true;
                break;
            }
        }
    }

    if !column_exists {
        return Err(format!("La colonna lingua '{}' non esiste nella tabella del progetto '{}'", language_code, project_name));
    }

    // Verifica se la lingua è registrata in project_languages (opzionale - potrebbe non esserci)
    let language_registered: bool = conn.query_row(
        "SELECT COUNT(*) FROM project_languages WHERE project_name = ? AND language_code = ?",
        params![table_name, language_code],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);

    // Rimuovi la lingua dalla tabella project_languages se esiste
    if language_registered {
        conn.execute(
            "DELETE FROM project_languages WHERE project_name = ? AND language_code = ?",
            params![table_name, language_code]
        ).map_err(|e| e.to_string())?;
    }

    // Elimina completamente la colonna dalla tabella
    // Ottieni la struttura della tabella corrente
    let mut columns = Vec::new();
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`)", table_name)).map_err(|e| e.to_string())?;
    let column_iter = stmt.query_map([], |row| {
        let column_name: String = row.get(1)?;
        let column_type: String = row.get(2)?;
        let not_null: bool = row.get(3)?;
        let default_value: Option<String> = row.get(4)?;
        let pk: bool = row.get(5)?;
        
        Ok((column_name, column_type, not_null, default_value, pk))
    }).map_err(|e| e.to_string())?;

    for column_result in column_iter {
        if let Ok((name, col_type, not_null, default_val, is_pk)) = column_result {
            if name != language_code {  // Escludi la colonna da eliminare
                let mut column_def = format!("`{}` {}", name, col_type);
                
                if not_null {
                    column_def.push_str(" NOT NULL");
                }
                
                if let Some(default) = default_val {
                    column_def.push_str(&format!(" DEFAULT {}", default));
                }
                
                if is_pk {
                    column_def.push_str(" PRIMARY KEY");
                }
                
                columns.push(column_def);
            }
        }
    }

    if columns.is_empty() {
        return Err("Errore: non è possibile eliminare tutte le colonne dalla tabella".to_string());
    }

    // Inizia una transazione per sicurezza
    conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

    // Crea una nuova tabella temporanea senza la colonna da eliminare
    let temp_table_name = format!("{}_temp", table_name);
    let create_temp_sql = format!(
        "CREATE TABLE `{}` ({})",
        temp_table_name,
        columns.join(", ")
    );
    
    if let Err(e) = conn.execute(&create_temp_sql, []) {
        conn.execute("ROLLBACK", []).ok();
        return Err(format!("Errore creazione tabella temporanea: {}", e));
    }

    // Copia tutti i dati nella nuova tabella (eccetto la colonna eliminata)
    let column_names: Vec<String> = columns.iter()
        .map(|col| col.split_whitespace().next().unwrap_or("").trim_matches('`').to_string())
        .collect();
    
    let select_columns = column_names.iter()
        .map(|name| format!("`{}`", name))
        .collect::<Vec<_>>()
        .join(", ");
    
    let copy_sql = format!(
        "INSERT INTO `{}` ({}) SELECT {} FROM `{}`",
        temp_table_name, select_columns, select_columns, table_name
    );
    
    if let Err(e) = conn.execute(&copy_sql, []) {
        conn.execute("ROLLBACK", []).ok();
        return Err(format!("Errore copia dati: {}", e));
    }

    // Elimina la tabella originale
    if let Err(e) = conn.execute(&format!("DROP TABLE `{}`", table_name), []) {
        conn.execute("ROLLBACK", []).ok();
        return Err(format!("Errore eliminazione tabella originale: {}", e));
    }

    // Rinomina la tabella temporanea
    if let Err(e) = conn.execute(&format!("ALTER TABLE `{}` RENAME TO `{}`", temp_table_name, table_name), []) {
        conn.execute("ROLLBACK", []).ok();
        return Err(format!("Errore rinominazione tabella: {}", e));
    }

    // Conferma la transazione
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    let message = if language_registered {
        format!("Lingua '{}' completamente eliminata dal progetto '{}' (colonna rimossa)", language_code, project_name)
    } else {
        format!("Lingua '{}' eliminata dal progetto '{}' (colonna rimossa - non era registrata)", language_code, project_name)
    };
    
    Ok(message)
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
    let content = fs::read_to_string(&filePath).map_err(|e| e.to_string())?;
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

#[derive(serde::Serialize)]
struct AccentedCharacter {
    id: i32,
    key: String,
    column: String,
    original_value: String,
    suggested_value: String,
    row_number: i32,
}

#[tauri::command]
fn check_accented_characters(table_name: String) -> Result<Vec<AccentedCharacter>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize table name
    let table_name: String = table_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Get all columns except system columns
    let columns = get_table_columns(table_name.clone())?;
    let text_columns: Vec<String> = columns.into_iter()
        .filter(|col| col != "id" && col != "image" && col != "key" && col != "keys_project" && col != "order_index")
        .collect();

    let mut accented_chars = Vec::new();
    let mut row_counter = 0;

    // Query all records
    let query = format!("SELECT id, key, {} FROM {} ORDER BY id", text_columns.join(", "), table_name);
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let id: i32 = row.get("id")?;
        let key: Option<String> = row.get("key")?;
        
        let mut row_data = std::collections::HashMap::new();
        for (i, col) in text_columns.iter().enumerate() {
            let value: Option<String> = row.get(2 + i)?; // id=0, key=1, then text columns start at 2
            row_data.insert(col.clone(), value.unwrap_or_default());
        }
        
        Ok((id, key.unwrap_or_default(), row_data))
    }).map_err(|e| e.to_string())?;

    for row_result in rows {
        let (id, key, row_data) = row_result.map_err(|e| e.to_string())?;
        row_counter += 1;

        // Check each text column for accented characters
        for (column, value) in row_data {
            if !value.is_empty() && has_accented_characters(&value) {
                let suggested = replace_accented_characters(&value);
                accented_chars.push(AccentedCharacter {
                    id,
                    key: key.clone(),
                    column: column.clone(),
                    original_value: value,
                    suggested_value: suggested,
                    row_number: row_counter,
                });
            }
        }
    }

    Ok(accented_chars)
}

fn has_accented_characters(text: &str) -> bool {
    text.chars().any(|c| match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'æ' |
        'è' | 'é' | 'ê' | 'ë' |
        'ì' | 'í' | 'î' | 'ï' |
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' |
        'ù' | 'ú' | 'û' | 'ü' |
        'ý' | 'ÿ' |
        'ç' | 'ñ' |
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Æ' |
        'È' | 'É' | 'Ê' | 'Ë' |
        'Ì' | 'Í' | 'Î' | 'Ï' |
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' |
        'Ù' | 'Ú' | 'Û' | 'Ü' |
        'Ý' | 'Ÿ' |
        'Ç' | 'Ñ' => true,
        _ => false,
    })
}

fn replace_accented_characters(text: &str) -> String {
    text.chars().map(|c| match c {
        // Lowercase vowels
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => "a&apos;",
        'è' | 'é' | 'ê' | 'ë' => "e&apos;",
        'ì' | 'í' | 'î' | 'ï' => "i&apos;",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => "o&apos;",
        'ù' | 'ú' | 'û' | 'ü' => "u&apos;",
        'ý' | 'ÿ' => "y&apos;",
        
        // Uppercase vowels
        'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "A&apos;",
        'È' | 'É' | 'Ê' | 'Ë' => "E&apos;",
        'Ì' | 'Í' | 'Î' | 'Ï' => "I&apos;",
        'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "O&apos;",
        'Ù' | 'Ú' | 'Û' | 'Ü' => "U&apos;",
        'Ý' | 'Ÿ' => "Y&apos;",
        
        // Special characters
        'ç' => "c&apos;",
        'Ç' => "C&apos;",
        'ñ' => "n&apos;",
        'Ñ' => "N&apos;",
        'æ' => "ae&apos;",
        'Æ' => "AE&apos;",
        
        // Regular characters remain unchanged
        _ => {
            let mut s = String::new();
            s.push(c);
            return s;
        }
    }.to_string()).collect::<Vec<String>>().join("")
}

#[tauri::command]
fn fix_accented_characters(table_name: String, fixes: Vec<serde_json::Value>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize table name
    let table_name: String = table_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    let mut fixed_count = 0;

    for fix in fixes {
        let id = fix.get("id").and_then(|v| v.as_i64()).ok_or("ID mancante")? as i32;
        let column = fix.get("column").and_then(|v| v.as_str()).ok_or("Colonna mancante")?;
        let new_value = fix.get("newValue").and_then(|v| v.as_str()).ok_or("Nuovo valore mancante")?;

        // Sanitize column name
        let column: String = column.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

        let update_query = format!("UPDATE {} SET {} = ? WHERE id = ?", table_name, column);
        conn.execute(&update_query, [new_value, &id.to_string()]).map_err(|e| e.to_string())?;
        fixed_count += 1;
    }

    Ok(format!("Corretti {} caratteri accentati.", fixed_count))
}

#[tauri::command]
fn get_setting(key: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);", []).map_err(|e| e.to_string())?;
    let value: Option<String> = conn.query_row("SELECT value FROM settings WHERE key = ?", [key], |row| row.get(0)).unwrap_or(None);
    Ok(value.unwrap_or_default())
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
        .filter(|col| col != "id" && col != "image" && col != "key" && col != "keys_project" && col != "order_index")
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
                }
            }
        }
    }

    Ok(serde_json::json!({
        "projectName": project_name,
        "projectPath": project_path,
        "exportFiles": export_files,
        "backupFiles": backup_files,
        "languageCount": extensions.len()
    }))
}

#[tauri::command]
fn export_translations_per_language(table_name: String) -> Result<String, String> {
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

    // Create backup folder and move existing translation-like files into it
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Local> = now.into();
    let timestamp = datetime.format("%Y%m%d_%H%M%S").to_string();
    let backup_dir = Path::new(&project_path).join(format!("translation_backups_{}", timestamp));
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    // Backup only files that match translation file patterns
    for entry in fs::read_dir(&project_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let file_name_lower = file_name.to_lowercase();
                
                // Backup only translation files with specific patterns:
                // - Files ending with .eng, .ita, .fra/.fre, .deu/.ger, .esp/.spa
                // - Files containing "string" and ending with .eng/.ita/etc (like rmc25010string.eng)
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
                    let dest = backup_dir.join(file_name);
                    // Try rename (move). If fails, fallback to copy.
                    if let Err(_) = fs::rename(&path, &dest) {
                        fs::copy(&path, &dest).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }

    // Load records and determine language columns
    let records = get_records(table_name.clone())?;
    if records.is_empty() {
        return Err("home.table_empty".to_string());
    }

    let columns = get_table_columns(table_name.clone())?;
    let export_columns: Vec<String> = columns.into_iter()
        .filter(|col| col != "id" && col != "image" && col != "key" && col != "keys_project" && col != "order_index")
        .collect();

    // Group languages by extension to avoid duplicates
    let mut language_groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    
    for lang in &export_columns {
        let ext = match lang.as_str() {
            "en" | "eng" => "eng",
            "it" | "ita" => "ita", 
            "fr" | "fra" | "fre" => "fra",
            "de" | "deu" | "ger" => "deu",
            "es" | "esp" | "spa" => "esp",
            _ => "eng",
        };
        language_groups.entry(ext.to_string()).or_insert_with(Vec::new).push(lang.clone());
    }

    // For each extension group, create a file with XML format
    for (ext, langs) in &language_groups {
        let mut content = String::new();
        content.push_str("<?xml version=\"1.0\" encoding=\"ISO-8859-1\" ?>\n");
        content.push_str("<strings>\n");
        content.push_str("<list>\n");
        
        for record in &records {
            if let Some(key) = record.get("key") {
                // For each record, use the preferred language in this group
                let mut val = String::new();
                
                // Create a priority order for languages in this extension group
                let mut sorted_langs = langs.clone();
                sorted_langs.sort_by(|a, b| {
                    // Give priority to full language codes over short ones
                    let a_priority = match a.as_str() {
                        "eng" => 0, "ita" => 0, "fra" => 0, "deu" => 0, "esp" => 0,
                        "en" => 1, "it" => 1, "fr" => 1, "de" => 1, "es" => 1,
                        _ => 2,
                    };
                    let b_priority = match b.as_str() {
                        "eng" => 0, "ita" => 0, "fra" => 0, "deu" => 0, "esp" => 0,
                        "en" => 1, "it" => 1, "fr" => 1, "de" => 1, "es" => 1,
                        _ => 2,
                    };
                    a_priority.cmp(&b_priority)
                });
                
                for lang in &sorted_langs {
                    if let Some(lang_val) = record.get(lang) {
                        if !lang_val.is_empty() {
                            val = lang_val.clone();
                            break;
                        }
                    }
                }
                
                // Escape XML entities
                let escaped_key = key.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;").replace("'", "&apos;");
                let escaped_val = val.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;").replace("'", "&apos;");
                content.push_str(&format!("<item key=\"{}\" value=\"{}\"/>\n", escaped_key, escaped_val));
            }
        }
        
        content.push_str("</list>\n");
        content.push_str("</strings>\n");

        let file_name = format!("{}string.{}", project_name, ext);
        let file_path = Path::new(&project_path).join(file_name);
        fs::write(&file_path, content).map_err(|e| format!("Errore nella scrittura del file {}: {}", file_path.display(), e))?;
    }

    Ok(format!("Esportati {} file in {} (backup in {})", language_groups.len(), project_path, backup_dir.display()))
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
    .invoke_handler(tauri::generate_handler![import_cht, import_project_directory, get_tables, delete_table, get_records, set_table_image, delete_table_image, update_record, delete_record, insert_record, get_table_columns, get_table_info, fetch_and_set_logo, get_setting, set_setting, open_url, export_cht_to_path, get_export_preview, export_translations_per_language, update_record_order, add_language_to_project, get_project_languages, remove_language_from_project, import_translation_file, get_translation_files_in_directory, find_keys_in_project, import_project_keys, get_project_keys, get_project_keys_with_status, import_translation_file_from_path, get_imported_files, translate_text, remove_unused_keys, check_accented_characters, fix_accented_characters])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
