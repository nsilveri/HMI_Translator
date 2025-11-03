use rusqlite::{params_from_iter, params, Connection, Result};
use std::fs;
use std::path::Path;
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
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name != 'table_images' AND name != 'settings' AND name != 'projects' AND name != 'project_languages';").map_err(|e| e.to_string())?;
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

#[tauri::command]
fn get_translation_files_in_directory(directory_path: String) -> Result<Vec<std::collections::HashMap<String, String>>, String> {
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
                        translation_files.push(file_info);
                    }
                }
            }
        }
    }
    
    Ok(translation_files)
}

#[tauri::command]
fn find_keys_in_project(directory_path: String, project_name: String) -> Result<Vec<String>, String> {
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
    
    let mut found_keys = std::collections::HashSet::new();
    
    // Funzione ricorsiva per scansionare le directory
    fn scan_directory(dir: &Path, keys: &mut std::collections::HashSet<String>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
        
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_path = entry.path();
            
            if file_path.is_dir() {
                // Scansiona ricorsivamente le sottodirectory
                scan_directory(&file_path, keys)?;
            } else if file_path.is_file() {
                if let Some(file_name) = file_path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.to_lowercase().ends_with(".hmiscr") {
                            // Prova prima a leggere come UTF-8, poi UTF-16
                            let content = match fs::read_to_string(&file_path) {
                                Ok(content) => content,
                                Err(_) => {
                                    match fs::read(&file_path) {
                                        Ok(bytes) => {
                                            // Prova UTF-16 LE con BOM
                                            if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
                                                let utf16_data: Vec<u16> = bytes[2..].chunks_exact(2)
                                                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                                                    .collect();
                                                match String::from_utf16(&utf16_data) {
                                                    Ok(content) => content,
                                                    Err(_) => String::from_utf8_lossy(&bytes).into_owned()
                                                }
                                            }
                                            // Prova UTF-16 LE senza BOM
                                            else if bytes.len() % 2 == 0 {
                                                let utf16_data: Vec<u16> = bytes.chunks_exact(2)
                                                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                                                    .collect();
                                                match String::from_utf16(&utf16_data) {
                                                    Ok(content) => {
                                                        if content.contains("<?xml") {
                                                            content
                                                        } else {
                                                            String::from_utf8_lossy(&bytes).into_owned()
                                                        }
                                                    },
                                                    Err(_) => String::from_utf8_lossy(&bytes).into_owned()
                                                }
                                            } else {
                                                String::from_utf8_lossy(&bytes).into_owned()
                                            }
                                        }
                                        Err(_) => continue,
                                    }
                                }
                            };
                            
                            println!("Debug: verifica presenza di pattern nel contenuto");
                            
                            // Test base: cerca stringhe semplici
                            let has_text_tag = content.contains("<text");
                            let has_text_close = content.contains("</text>");
                            let has_cmd_manuali = content.contains("CMD MANUALI");
                            let has_string = content.contains("STRING_");
                            
                            println!("Contiene '<text': {}", has_text_tag);
                            println!("Contiene '</text>': {}", has_text_close);
                            println!("Contiene 'CMD MANUALI': {}", has_cmd_manuali);
                            println!("Contiene 'STRING_': {}", has_string);
                            
                            // Mostra un campione del contenuto per debug
                            let sample_size = 2000.min(content.len());
                            println!("CAMPIONE INIZIO FILE (primi {} caratteri):", sample_size);
                            
                            // Mostra carattere per carattere i primi 200 per vedere caratteri speciali
                            let debug_sample = &content[0..200.min(content.len())];
                            let mut debug_chars = String::new();
                            for (i, ch) in debug_sample.chars().enumerate() {
                                if ch.is_control() {
                                    debug_chars.push_str(&format!("[{}]", ch as u32));
                                } else {
                                    debug_chars.push(ch);
                                }
                                if i > 200 { break; }
                            }
                            println!("DEBUG CARATTERI: {}", debug_chars);
                            
                            // Se contiene i tag, prova a trovarli
                            if has_text_tag && has_text_close {
                                println!("I tag ci sono, cerchiamo manualmente...");
                                
                                let mut pos = 0;
                                let mut found_count = 0;
                                
                                while let Some(text_start) = content[pos..].find("<text") {
                                    let absolute_text_start = pos + text_start;
                                    found_count += 1;
                                    
                                    println!("Trovato tag <text numero {} alla posizione {}", found_count, absolute_text_start);
                                    
                                    // Trova la fine del tag di apertura >
                                    if let Some(tag_end) = content[absolute_text_start..].find('>') {
                                        let content_start = absolute_text_start + tag_end + 1;
                                        
                                        // Trova il tag di chiusura </text>
                                        if let Some(close_tag) = content[content_start..].find("</text>") {
                                            let content_end = content_start + close_tag;
                                            
                                            // Estrai il contenuto tra i tag
                                            let text_content = &content[content_start..content_end];
                                            let key_str = text_content.trim();
                                            
                                            println!("Contenuto tag {}: '{}'", found_count, key_str);
                                            
                                            // Filtra solo contenuti validi (non XML, non troppo lunghi, non numeri puri)
                                            if !key_str.is_empty() 
                                                && key_str.len() < 100  // Non troppo lungo
                                                && !key_str.contains('<')  // Non contiene altri tag XML
                                                && !key_str.contains('>')  // Non contiene altri tag XML
                                                && !key_str.chars().all(|c| c.is_numeric() || c == '0')  // Non è solo numeri o solo zeri
                                                && key_str.len() > 1  // Almeno 2 caratteri
                                            {
                                                println!("Chiave valida aggiunta: '{}'", key_str);
                                                keys.insert(key_str.to_string());
                                            } else {
                                                println!("Contenuto scartato (troppo lungo, contiene XML o solo numeri): '{}'", 
                                                    if key_str.len() > 50 { &key_str[0..50] } else { key_str });
                                            }
                                            
                                            // Continua la ricerca dopo questo tag
                                            pos = content_end + 7;
                                        } else {
                                            pos = absolute_text_start + 5;
                                        }
                                    } else {
                                        pos = absolute_text_start + 5;
                                    }
                                    
                                    if found_count >= 5 { 
                                        println!("Fermi ai primi 5 per debug...");
                                        break; 
                                    }
                                }
                            }
                            
                            println!("File {}: estratte {} chiavi uniche", 
                                file_path.display(), keys.len());

                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    // Scansiona tutte le cartelle che corrispondono
    for folder_path in matching_folders {
        scan_directory(&folder_path, &mut found_keys)?;
    }
    
    // Converti HashSet in Vec e ordina
    let mut keys_vec: Vec<String> = found_keys.into_iter().collect();
    keys_vec.sort();
    
    Ok(keys_vec)
}

#[tauri::command]
fn import_project_keys(project_name: String, keys: Vec<String>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Prima assicuriamoci che la colonna keys_project esista
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
    
    let mut imported_count = 0;
    let mut updated_count = 0;
    
    for key in keys {
        // Verifica se la chiave esiste già
        let check_sql = format!("SELECT id FROM `{}` WHERE key = ?", project_name);
        let key_exists = conn.query_row(&check_sql, [&key], |_| Ok(())).is_ok();
        
        if key_exists {
            // Aggiorna il campo keys_project per questa chiave
            let update_sql = format!(
                "UPDATE `{}` SET keys_project = ? WHERE key = ?",
                project_name
            );
            conn.execute(&update_sql, [&key, &key]).map_err(|e| e.to_string())?;
            updated_count += 1;
        } else {
            // Inserisci nuova riga con la chiave SOLO nel campo keys_project (key rimane NULL)
            let insert_sql = format!(
                "INSERT INTO `{}` (keys_project) VALUES (?)",
                project_name
            );
            conn.execute(&insert_sql, [&key]).map_err(|e| e.to_string())?;
            imported_count += 1;
        }
    }
    
    Ok(format!("Importate {} nuove chiavi, aggiornate {} chiavi esistenti nella colonna keys_project", imported_count, updated_count))
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

    let mut stmt = conn.prepare("SELECT language_code, language_name FROM project_languages WHERE project_name = ? ORDER BY language_code").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([table_name], |row| {
        let mut map = std::collections::HashMap::new();
        map.insert("code".to_string(), row.get::<_, String>(0)?);
        map.insert("name".to_string(), row.get::<_, String>(1)?);
        Ok(map)
    }).map_err(|e| e.to_string())?;

    let mut languages = Vec::new();
    for row in rows {
        languages.push(row.map_err(|e| e.to_string())?);
    }
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

    // Rimuovi la lingua dalla tabella project_languages
    conn.execute(
        "DELETE FROM project_languages WHERE project_name = ? AND language_code = ?",
        params![table_name, language_code]
    ).map_err(|e| e.to_string())?;

    // Nota: SQLite non supporta DROP COLUMN in modo semplice, quindi lasciamo la colonna
    // ma la ignoriamo nell'interfaccia utente

    Ok(format!("Lingua '{}' rimossa dal progetto '{}'", language_code, project_name))
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![import_cht, import_project_directory, get_tables, delete_table, get_records, set_table_image, delete_table_image, update_record, delete_record, insert_record, get_table_columns, get_table_info, fetch_and_set_logo, get_setting, set_setting, open_url, export_cht_to_path, update_record_order, add_language_to_project, get_project_languages, remove_language_from_project, import_translation_file, get_translation_files_in_directory, find_keys_in_project, import_project_keys])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
