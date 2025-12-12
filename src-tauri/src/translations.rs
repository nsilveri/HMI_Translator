use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use encoding_rs::WINDOWS_1252;

use crate::{get_table_columns_internal, read_text_file_best_effort, get_records_internal, get_table_info_internal, detect_file_encoding_internal};

// Helper function to parse translation file content
pub fn parse_translation_file_content(file_path: &Path) -> Result<HashMap<String, String>, String> {
    let content = read_text_file_best_effort(file_path).map_err(|e| e.to_string())?;
    let mut translations = HashMap::new();
    
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

pub fn check_if_content_already_imported(file_path: &Path, table_name: &str, language_column: &str) -> Result<bool, String> {
    let file_translations = parse_translation_file_content(file_path)?;
    
    if file_translations.is_empty() {
        return Ok(false);
    }
    
    // Get database content for this language
    let db_path = "../data/database.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(&format!("SELECT key, {} FROM `{}`", language_column, table_name)).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let key: String = row.get("key")?;
        let value: Option<String> = row.get(language_column).unwrap_or(None);
        Ok((key, value.unwrap_or_default()))
    }).map_err(|e| e.to_string())?;
    
    let mut db_translations = HashMap::new();
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
pub fn get_translation_files_in_directory(directory_path: String, table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
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
                        let mut file_info = HashMap::new();
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
                        if let Ok(columns) = get_table_columns_internal(table_name.clone()) {
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

#[tauri::command]
pub fn import_translation_file_from_path(table_name: String, language_code: String, file_path: String) -> Result<String, String> {
    // Leggi il file dal filesystem
    let content = read_text_file_best_effort(Path::new(&file_path)).map_err(|e| format!("Errore lettura file {}: {}", file_path, e))?;
    
    // Usa la funzione esistente per importare il contenuto
    import_translation_file_with_merge(table_name, language_code, content, file_path)
}

#[tauri::command]
pub fn import_translation_file_with_merge(table_name: String, language_code: String, xml_content: String, file_path: String) -> Result<String, String> {
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
    
    // Parsa l'XML e estrai le traduzioni
    let mut translations = HashMap::new();
    
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
                            // Unescape XML entities
                            let unescaped_key = key
                                .replace("&apos;", "'")
                                .replace("&quot;", "\"")
                                .replace("&lt;", "<")
                                .replace("&gt;", ">")
                                .replace("&amp;", "&");
                            let unescaped_value = value
                                .replace("&apos;", "'")
                                .replace("&quot;", "\"")
                                .replace("&lt;", "<")
                                .replace("&gt;", ">")
                                .replace("&amp;", "&");
                            translations.insert(unescaped_key, unescaped_value);
                        }
                    }
                }
            }
        }
        // Formato 3: chiave tra > e </text>
        else if trimmed.contains(">") && trimmed.contains("</text>") {
            if let Some(content_start) = trimmed.find(">") {
                if let Some(content_end) = trimmed.rfind("</text>") {
                    let content = &trimmed[content_start + 1..content_end];
                    // Usa il contenuto stesso come chiave se non abbiamo un ID
                    if !content.is_empty() {
                        translations.insert(content.to_string(), content.to_string());
                    }
                }
            }
        }
    }
    
    if translations.is_empty() {
        return Err("Nessuna traduzione trovata nel file".to_string());
    }
    
    // Inserisci/aggiorna le traduzioni
    let mut inserted = 0;
    let mut updated = 0;
    
    for (key, value) in &translations {
        // Controlla se la chiave esiste già
        let check_sql = format!("SELECT COUNT(*) FROM `{}` WHERE key = ?", table_name);
        let exists: i64 = conn.query_row(&check_sql, [key], |row| row.get(0)).unwrap_or(0);
        
        if exists > 0 {
            // Aggiorna il record esistente
            let update_sql = format!("UPDATE `{}` SET `{}` = ? WHERE key = ?", table_name, column_name);
            conn.execute(&update_sql, params![value, key]).map_err(|e| e.to_string())?;
            updated += 1;
        } else {
            // Inserisci un nuovo record
            let insert_sql = format!("INSERT INTO `{}` (key, `{}`) VALUES (?, ?)", table_name, column_name);
            conn.execute(&insert_sql, params![key, value]).map_err(|e| e.to_string())?;
            inserted += 1;
        }
    }
    
    // Estrai il nome del file dal percorso
    let file_name = Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    // Registra l'importazione
    let insert_import_sql = format!(
        "INSERT OR REPLACE INTO `{}_imports` (file_path, file_name, language_code, translations_count) VALUES (?, ?, ?, ?)",
        table_name
    );
    
    let total_translations = translations.len() as i32;
    conn.execute(&insert_import_sql, params![file_path, file_name, language_code, total_translations]).map_err(|e| e.to_string())?;
    
    Ok(format!(
        "Importazione completata: {} nuove traduzioni, {} aggiornate per lingua '{}'",
        inserted, updated, language_code
    ))
}

#[tauri::command]
pub fn add_language_to_project(project_name: String, language_code: String, language_name: String) -> Result<String, String> {
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
pub fn get_project_languages(project_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Get all language columns from the project table (excluding system columns)
    let columns = get_table_columns_internal(table_name.clone())?;
    let language_columns: Vec<String> = columns.into_iter()
        .filter(|col| col != "id" && col != "image" && col != "key" && col != "keys_project" && col != "key_files" && col != "order_index" && col != "created_at")
        .collect();

    let mut languages = Vec::new();
    
    // For each language column, try to get the name from project_languages table, or use the column name
    for lang_code in language_columns {
        // Try to get the proper name from project_languages table
        let language_name = if let Ok(name) = conn.query_row(
            "SELECT language_name FROM project_languages WHERE project_name = ? AND language_code = ?",
            [&table_name, &lang_code],
            |row| row.get::<_, String>(0)
        ) {
            name
        } else {
            // Generate a friendly name based on the column name
            match lang_code.as_str() {
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
            }
        };
        
        let mut map = HashMap::new();
        map.insert("code".to_string(), lang_code);
        map.insert("name".to_string(), language_name);
        languages.push(map);
    }
    
    // Sort by code
    languages.sort_by(|a, b| a.get("code").unwrap().cmp(b.get("code").unwrap()));
    
    Ok(languages)
}

#[tauri::command]
pub fn import_translation_file(project_name: String, language_code: String, file_path: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Sanitize project name
    let table_name: String = project_name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();

    // Leggi e parsa il file XML
    let content = read_text_file_best_effort(Path::new(&file_path)).map_err(|e| format!("Errore lettura file: {}", e))?;
    
    // Parsing XML semplice per estrarre le coppie key-value
    let mut translations = HashMap::new();
    
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
pub fn remove_language_from_project(project_name: String, language_code: String) -> Result<String, String> {
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
        return Err(format!("Errore rinomina tabella: {}", e));
    }

    // Commit della transazione
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    Ok(format!("Lingua '{}' rimossa con successo dal progetto '{}'", language_code, project_name))
}

#[tauri::command]
pub fn export_translations_per_language(table_name: String) -> Result<String, String> {
    // Retrieve project info (path)
    let info = get_table_info_internal(table_name.clone())?;
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
                
                // Backup only translation files with specific patterns
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
    let records = get_records_internal(table_name.clone())?;
    if records.is_empty() {
        return Err("home.table_empty".to_string());
    }

    let columns = get_table_columns_internal(table_name.clone())?;
    // Filter out non-language columns more thoroughly
    let export_columns: Vec<String> = columns.into_iter()
        .filter(|col| {
            let col_lower = col.to_lowercase();
            // Exclude technical/system columns
            col_lower != "id" && 
            col_lower != "image" && 
            col_lower != "key" && 
            col_lower != "keys_project" && 
            col_lower != "key_files" && 
            col_lower != "order_index" &&
            col_lower != "order" &&
            col_lower != "project_id" &&
            col_lower != "file_path" &&
            col_lower != "source_file" &&
            col_lower != "created_at" &&
            col_lower != "updated_at" &&
            col_lower != "timestamp" &&
            col_lower != "created" &&
            col_lower != "updated" &&
            !col_lower.contains("_id") &&
            !col_lower.contains("path")
        })
        .collect();

    // Group languages by extension - only known language codes
    let mut language_groups: HashMap<String, Vec<String>> = HashMap::new();
    
    for lang in &export_columns {
        let lang_lower = lang.to_lowercase();
        let ext = match lang_lower.as_str() {
            "en" | "eng" | "english" => Some("eng"),
            "it" | "ita" | "italian" | "italiano" => Some("ita"), 
            "fr" | "fra" | "fre" | "french" | "francese" => Some("fra"),
            "de" | "deu" | "ger" | "german" | "tedesco" => Some("deu"),
            "es" | "esp" | "spa" | "spanish" | "spagnolo" => Some("esp"),
            "pt" | "por" | "portuguese" | "portoghese" => Some("por"),
            "nl" | "nld" | "dut" | "dutch" | "olandese" => Some("nld"),
            "ru" | "rus" | "russian" | "russo" => Some("rus"),
            "zh" | "chi" | "zho" | "chinese" | "cinese" => Some("zho"),
            "ja" | "jpn" | "japanese" | "giapponese" => Some("jpn"),
            "ko" | "kor" | "korean" | "coreano" => Some("kor"),
            "ar" | "ara" | "arabic" | "arabo" => Some("ara"),
            "pl" | "pol" | "polish" | "polacco" => Some("pol"),
            "tr" | "tur" | "turkish" | "turco" => Some("tur"),
            "sv" | "swe" | "swedish" | "svedese" => Some("swe"),
            "da" | "dan" | "danish" | "danese" => Some("dan"),
            "fi" | "fin" | "finnish" | "finlandese" => Some("fin"),
            "no" | "nor" | "norwegian" | "norvegese" => Some("nor"),
            "cs" | "ces" | "cze" | "czech" | "ceco" => Some("ces"),
            "hu" | "hun" | "hungarian" | "ungherese" => Some("hun"),
            "el" | "ell" | "gre" | "greek" | "greco" => Some("ell"),
            "he" | "heb" | "hebrew" | "ebraico" => Some("heb"),
            "th" | "tha" | "thai" | "tailandese" => Some("tha"),
            "vi" | "vie" | "vietnamese" | "vietnamita" => Some("vie"),
            "id" | "ind" | "indonesian" | "indonesiano" => Some("ind"),
            "ms" | "msa" | "malay" | "malese" => Some("msa"),
            "ro" | "ron" | "rum" | "romanian" | "rumeno" => Some("ron"),
            "uk" | "ukr" | "ukrainian" | "ucraino" => Some("ukr"),
            "bg" | "bul" | "bulgarian" | "bulgaro" => Some("bul"),
            "hr" | "hrv" | "croatian" | "croato" => Some("hrv"),
            "sk" | "slk" | "slo" | "slovak" | "slovacco" => Some("slk"),
            "sl" | "slv" | "slovenian" | "sloveno" => Some("slv"),
            "sr" | "srp" | "serbian" | "serbo" => Some("srp"),
            "lt" | "lit" | "lithuanian" | "lituano" => Some("lit"),
            "lv" | "lav" | "latvian" | "lettone" => Some("lav"),
            "et" | "est" | "estonian" | "estone" => Some("est"),
            _ => None,  // Skip unknown columns - DON'T default to eng!
        };
        
        if let Some(extension) = ext {
            language_groups.entry(extension.to_string()).or_insert_with(Vec::new).push(lang.clone());
        }
    }

    // For each extension group, create a file with XML format
    for (ext, langs) in &language_groups {
        // Try to detect encoding from existing file
        let file_name = format!("{}string.{}", project_name, ext);
        let file_path = Path::new(&project_path).join(&file_name);
        let encoding = if file_path.exists() {
            detect_file_encoding_internal(file_path.to_string_lossy().to_string())
                .unwrap_or("Windows-1252".to_string())
        } else {
            "Windows-1252".to_string() // Default for Movicon/Premium HMI compatibility
        };
        
        let encoding_declaration = match encoding.as_str() {
            "UTF-8" => "UTF-8",
            _ => "Windows-1252"
        };
        
        let mut content = String::new();
        content.push_str(&format!("<?xml version=\"1.0\" encoding=\"{}\" ?>\n", encoding_declaration));
        content.push_str("<strings>\n");
        content.push_str("<list>\n");
        
        for record in &records {
            if let Some(key) = record.get("key") {
                // Skip entries with empty keys
                if key.trim().is_empty() {
                    continue;
                }
                // For each record, use the preferred language in this group
                let mut val = String::new();
                for lang_col in langs {
                    if let Some(v) = record.get(lang_col) {
                        if !v.is_empty() {
                            val = v.clone();
                            break;
                        }
                    }
                }
                
                // Escape XML special characters
                let escaped_key = key
                    .replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;")
                    .replace("\"", "&quot;");
                let escaped_val = val
                    .replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;")
                    .replace("\"", "&quot;");
                
                content.push_str(&format!("<item key=\"{}\" value=\"{}\"/>\n", escaped_key, escaped_val));
            }
        }
        
        content.push_str("</list>\n");
        content.push_str("</strings>\n");
        
        // Write file with correct encoding
        let output_path = Path::new(&project_path).join(&file_name);
        
        if encoding_declaration == "Windows-1252" {
            // Encode content to Windows-1252
            let (encoded_bytes, _, had_errors) = WINDOWS_1252.encode(&content);
            if had_errors {
                // If there are characters that cannot be encoded, log a warning but continue
                eprintln!("Warning: Some characters could not be encoded to Windows-1252 in file {}", file_name);
            }
            fs::write(&output_path, &*encoded_bytes).map_err(|e| e.to_string())?;
        } else {
            // Write as UTF-8
            fs::write(&output_path, content).map_err(|e| e.to_string())?;
        }
    }

    Ok(format!("Esportazione completata. File di traduzione creati in: {}", project_path))
}

#[tauri::command]
pub fn export_translations_to_machine(table_name: String, machine_path: String) -> Result<String, String> {
    println!("[export_translations_to_machine] Avvio export per '{}' in '{}'", table_name, machine_path);
    // 1. Esporta i file di traduzione nella cartella progetto
    let export_result = export_translations_per_language(table_name.clone());
    if let Err(e) = &export_result {
        println!("[export_translations_to_machine] Errore export_translations_per_language: {}", e);
    }
    let info = get_table_info_internal(table_name.clone())?;
    let project_path = info.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    println!("[export_translations_to_machine] project_path: '{}'", project_path);
    if project_path.is_empty() {
        return Err("Percorso progetto non disponibile".to_string());
    }
    if machine_path.is_empty() {
        return Err("Percorso macchina non configurato. Impostalo nelle impostazioni.".to_string());
    }
    let machine_dir = std::path::Path::new(&machine_path);
    if !machine_dir.exists() {
        return Err(format!("Il percorso macchina non esiste: {}", machine_path));
    }

    // 2. Trova i file di traduzione appena creati nella cartella progetto
    let mut copied_files = Vec::new();
    let mut errors = Vec::new();
    let mut found_any = false;
    if let Ok(entries) = std::fs::read_dir(&project_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    let file_name_lower = file_name.to_lowercase();
                    // Solo file di traduzione
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
                        found_any = true;
                        let dest_path = machine_dir.join(file_name);
                        match std::fs::copy(&path, &dest_path) {
                            Ok(_) => copied_files.push(file_name.to_string()),
                            Err(e) => errors.push(format!("{}: {}", file_name, e)),
                        }
                    }
                }
            }
        }
    }

    if !found_any {
        return Err(format!("Nessun file di traduzione trovato in '{}'. Controlla che l'export sia andato a buon fine e che i file siano stati generati.", project_path));
    }
    if !errors.is_empty() {
        return Err(format!("Alcuni file non sono stati copiati: {}", errors.join(", ")));
    }

    let msg = format!("File di traduzione esportati: {}", copied_files.join(", "));
    if let Err(e) = export_result {
        return Err(format!("Esportazione locale completata, ma errore: {}. {}", e, msg));
    }
    Ok(msg)
}