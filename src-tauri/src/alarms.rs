use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::collections::HashMap;

use crate::read_text_file_best_effort;

// Funzione helper per scansionare ricorsivamente le directory
fn scan_directory_recursive(path: &Path, alarm_files: &mut Vec<HashMap<String, String>>, table_name: &str) -> Result<(), String> {
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();
        
        if file_path.is_file() {
            if let Some(file_name) = file_path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    let file_name_lower = file_name_str.to_lowercase();
                    
                    // Cerca file con estensione .hmialr o .movalr
                    if file_name_lower.ends_with(".hmialr") || file_name_lower.ends_with(".movalr") {
                        let file_type = if file_name_lower.ends_with(".hmialr") { "hmialr" } else { "movalr" };
                        let mut file_info = HashMap::new();
                        file_info.insert("file_name".to_string(), file_name_str.to_string());
                        file_info.insert("file_path".to_string(), file_path.to_string_lossy().to_string());
                        file_info.insert("file_type".to_string(), file_type.to_string());

                        println!("Found alarm file: {} (type: {})", file_name_str, file_type);
                        
                        // Check if file is already imported
                        let mut already_imported = false;
                        
                        // Check in the alarm imports table directly
                        let db_path = "../data/projects.db";
                        if let Ok(conn) = Connection::open(db_path) {
                            let imports_table_name = format!("{}_alarm_imports", table_name);
                            let check_sql = format!(
                                "SELECT COUNT(*) FROM `{}` WHERE file_path = ?",
                                imports_table_name
                            );
                            if let Ok(count) = conn.query_row(&check_sql, [file_path.to_string_lossy().to_string()], |row| row.get::<_, i32>(0)) {
                                already_imported = count > 0;
                            }
                        }
                        
                        file_info.insert("already_imported".to_string(), already_imported.to_string());
                        
                        // Only add to list if not already imported
                        if !already_imported {
                            alarm_files.push(file_info);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_alarm_files_in_directory(directory_path: String, table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let path = Path::new(&directory_path);
    
    println!("Searching for alarm files in: {}", directory_path);
    
    if !path.is_dir() {
        return Err(format!("Il percorso specificato non è una directory: {}", directory_path));
    }
    
    let mut alarm_files = Vec::new();
    
    // Scansiona ricorsivamente la directory e le sottodirectory
    scan_directory_recursive(path, &mut alarm_files, &table_name)?;
    
    println!("Total alarm files found: {}", alarm_files.len());
    
    Ok(alarm_files)
}

// Struttura per rappresentare un allarme parsato dal file XML
#[derive(Debug, Clone)]
struct ParsedAlarm {
    alarm_name: String,
    device: String,
    variable: String,
    area: String,
    enabled: String,
    threshold_name: String,
    threshold_title: String,
    threshold_help: String,
    severity: String,
    condition: String,
    threshold_value: String,
    sec_delay: String,
    support_ack: String,
    support_reset: String,
    log: String,
    print: String,
    beep_enabled: String,
    back_color: String,
    text_color: String,
    blink_back_color: String,
    blink_text_color: String,
    blink_on_new_alarm: String,
}

// Funzione helper per estrarre un attributo da un tag XML
fn extract_attribute(line: &str, attr_name: &str) -> String {
    let search = format!("{}=\"", attr_name);
    if let Some(start) = line.find(&search) {
        let value_start = start + search.len();
        if let Some(end) = line[value_start..].find('"') {
            return line[value_start..value_start + end].to_string();
        }
    }
    String::new()
}

// Funzione helper per estrarre il contenuto di un tag XML
fn extract_tag_content(line: &str, tag_name: &str) -> String {
    let open_tag = format!("<{}", tag_name);
    let close_tag = format!("</{}>", tag_name);
    
    if let Some(start) = line.find(&open_tag) {
        // Trova la fine del tag di apertura (dopo >)
        if let Some(tag_end) = line[start..].find('>') {
            let content_start = start + tag_end + 1;
            if let Some(end) = line[content_start..].find(&close_tag) {
                return line[content_start..content_start + end].to_string();
            }
        }
    }
    String::new()
}

// Funzione per parsare gli allarmi dal contenuto XML
fn parse_alarms_from_xml(content: &str) -> Vec<ParsedAlarm> {
    let mut alarms = Vec::new();
    
    // Splitta per ogni <Alarm>
    let alarm_blocks: Vec<&str> = content.split("<Alarm>").collect();
    
    for (i, block) in alarm_blocks.iter().enumerate() {
        if i == 0 { continue; } // Salta la parte prima del primo <Alarm>
        
        let mut alarm = ParsedAlarm {
            alarm_name: String::new(),
            device: String::new(),
            variable: String::new(),
            area: String::new(),
            enabled: String::new(),
            threshold_name: String::new(),
            threshold_title: String::new(),
            threshold_help: String::new(),
            severity: String::new(),
            condition: String::new(),
            threshold_value: String::new(),
            sec_delay: String::new(),
            support_ack: String::new(),
            support_reset: String::new(),
            log: String::new(),
            print: String::new(),
            beep_enabled: String::new(),
            back_color: String::new(),
            text_color: String::new(),
            blink_back_color: String::new(),
            blink_text_color: String::new(),
            blink_on_new_alarm: String::new(),
        };
        
        // Trova il tag <Name> dell'allarme (primo Name nel blocco)
        if let Some(name_start) = block.find("<Name ") {
            let name_section = &block[name_start..];
            if let Some(name_end) = name_section.find("</Name>") {
                let name_tag = &name_section[..name_end + 7];
                
                // Estrai attributi dal tag Name
                alarm.device = extract_attribute(name_tag, "Device");
                alarm.variable = extract_attribute(name_tag, "Variable");
                alarm.area = extract_attribute(name_tag, "Area");
                alarm.enabled = extract_attribute(name_tag, "Enabled");
                
                // Estrai il contenuto del tag Name (nome dell'allarme)
                alarm.alarm_name = extract_tag_content(name_tag, "Name");
            }
        }
        
        // Trova ThresholdList se presente
        if let Some(threshold_start) = block.find("<ThresholdList>") {
            let threshold_section = &block[threshold_start..];
            
            // Trova il primo Threshold
            if let Some(th_start) = threshold_section.find("<Threshold>") {
                let th_section = &threshold_section[th_start..];
                
                // Trova il Name del Threshold (secondo Name nel blocco)
                if let Some(name_start) = th_section.find("<Name ") {
                    let name_section = &th_section[name_start..];
                    if let Some(name_end) = name_section.find("</Name>") {
                        let name_tag = &name_section[..name_end + 7];
                        
                        alarm.threshold_title = extract_attribute(name_tag, "Title");
                        alarm.threshold_help = extract_attribute(name_tag, "Help");
                        alarm.threshold_name = extract_tag_content(name_tag, "Name");
                    }
                }
                
                // Trova Execution
                if let Some(exec_start) = th_section.find("<Execution ") {
                    let exec_section = &th_section[exec_start..];
                    if let Some(exec_end) = exec_section.find("/>") {
                        let exec_tag = &exec_section[..exec_end + 2];
                        
                        alarm.condition = extract_attribute(exec_tag, "Condition");
                        alarm.threshold_value = extract_attribute(exec_tag, "Threshold");
                        alarm.severity = extract_attribute(exec_tag, "Severity");
                        alarm.sec_delay = extract_attribute(exec_tag, "SecDelay");
                    }
                }
                
                // Trova Style
                if let Some(style_start) = th_section.find("<Style ") {
                    let style_section = &th_section[style_start..];
                    if let Some(style_end) = style_section.find("/>") {
                        let style_tag = &style_section[..style_end + 2];
                        
                        alarm.support_ack = extract_attribute(style_tag, "SupportAck");
                        alarm.support_reset = extract_attribute(style_tag, "SupportReset");
                        alarm.log = extract_attribute(style_tag, "Log");
                        alarm.print = extract_attribute(style_tag, "Print");
                        alarm.beep_enabled = extract_attribute(style_tag, "BeepEnabled");
                        alarm.back_color = extract_attribute(style_tag, "BackColor");
                        alarm.text_color = extract_attribute(style_tag, "TextColor");
                        alarm.blink_back_color = extract_attribute(style_tag, "BlinkBackColor");
                        alarm.blink_text_color = extract_attribute(style_tag, "BlinkTextColor");
                        alarm.blink_on_new_alarm = extract_attribute(style_tag, "BlinkOnNewAlarm");
                    }
                }
            }
        }
        
        // Aggiungi l'allarme solo se ha un nome
        if !alarm.alarm_name.is_empty() {
            alarms.push(alarm);
        }
    }
    
    alarms
}

// Funzione per verificare e aggiungere colonne mancanti alla tabella allarmi
fn ensure_alarm_columns_exist(conn: &Connection, table_name: &str) -> Result<(), String> {
    // Lista delle colonne che devono esistere con i loro tipi
    let required_columns = vec![
        ("back_color", "TEXT"),
        ("text_color", "TEXT"),
        ("blink_back_color", "TEXT"),
        ("blink_text_color", "TEXT"),
        ("blink_on_new_alarm", "TEXT"),
    ];
    
    // Ottieni le colonne esistenti nella tabella
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", table_name))
        .map_err(|e| e.to_string())?;
    
    let existing_columns: Vec<String> = stmt.query_map([], |row| {
        Ok(row.get::<_, String>(1)?)
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    // Aggiungi le colonne mancanti
    for (col_name, col_type) in required_columns {
        if !existing_columns.contains(&col_name.to_string()) {
            let alter_sql = format!(
                "ALTER TABLE `{}` ADD COLUMN {} {} DEFAULT ''",
                table_name, col_name, col_type
            );
            conn.execute(&alter_sql, []).map_err(|e| {
                format!("Errore aggiungendo colonna {}: {}", col_name, e)
            })?;
            println!("Aggiunta colonna {} alla tabella {}", col_name, table_name);
        }
    }
    
    Ok(())
}

#[tauri::command]
pub fn import_alarm_file_from_path(table_name: String, file_path: String) -> Result<String, String> {
    // Leggi il file dal filesystem
    let content = read_text_file_best_effort(Path::new(&file_path)).map_err(|e| format!("Errore lettura file {}: {}", file_path, e))?;
    
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Crea la tabella degli allarmi con le colonne per i dati estratti
    let alarms_table_name = format!("{}_alarms", table_name);
    let create_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            alarm_name TEXT NOT NULL,
            device TEXT,
            variable TEXT,
            area TEXT,
            enabled TEXT,
            threshold_name TEXT,
            threshold_title TEXT,
            threshold_help TEXT,
            severity TEXT,
            condition TEXT,
            threshold_value TEXT,
            sec_delay TEXT,
            support_ack TEXT,
            support_reset TEXT,
            log TEXT,
            print TEXT,
            beep_enabled TEXT,
            back_color TEXT,
            text_color TEXT,
            blink_back_color TEXT,
            blink_text_color TEXT,
            blink_on_new_alarm TEXT,
            source_file TEXT,
            import_date DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        alarms_table_name
    );
    conn.execute(&create_table_sql, []).map_err(|e| e.to_string())?;
    
    // Verifica e aggiungi colonne mancanti per tabelle esistenti
    ensure_alarm_columns_exist(&conn, &alarms_table_name)?;
    
    // Parsa gli allarmi dal file XML
    let alarms = parse_alarms_from_xml(&content);
    let alarm_count = alarms.len() as i32;
    
    // Estrai il nome del file
    let file_name = Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    
    // Inserisci ogni allarme come record separato
    let insert_sql = format!(
        "INSERT INTO `{}` (alarm_name, device, variable, area, enabled, threshold_name, threshold_title, threshold_help, severity, condition, threshold_value, sec_delay, support_ack, support_reset, log, print, beep_enabled, back_color, text_color, blink_back_color, blink_text_color, blink_on_new_alarm, source_file) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        alarms_table_name
    );
    
    for alarm in &alarms {
        conn.execute(&insert_sql, [
            &alarm.alarm_name,
            &alarm.device,
            &alarm.variable,
            &alarm.area,
            &alarm.enabled,
            &alarm.threshold_name,
            &alarm.threshold_title,
            &alarm.threshold_help,
            &alarm.severity,
            &alarm.condition,
            &alarm.threshold_value,
            &alarm.sec_delay,
            &alarm.support_ack,
            &alarm.support_reset,
            &alarm.log,
            &alarm.print,
            &alarm.beep_enabled,
            &alarm.back_color,
            &alarm.text_color,
            &alarm.blink_back_color,
            &alarm.blink_text_color,
            &alarm.blink_on_new_alarm,
            &file_name,
        ]).map_err(|e| e.to_string())?;
    }
    
    // Crea tabella per tracciare i file importati se non esiste
    let imports_table_name = format!("{}_alarm_imports", table_name);
    let create_imports_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_type TEXT NOT NULL,
            import_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            records_count INTEGER NOT NULL,
            UNIQUE(file_path)
        )",
        imports_table_name
    );
    conn.execute(&create_imports_table_sql, []).map_err(|e| e.to_string())?;
    
    // Registra l'importazione
    let insert_import_sql = format!(
        "INSERT OR REPLACE INTO `{}` (file_path, file_name, file_type, records_count) VALUES (?, ?, 'hmialr', ?)",
        imports_table_name
    );
    
    conn.execute(&insert_import_sql, [&file_path, &file_name, &alarm_count.to_string()]).map_err(|e| e.to_string())?;
    
    Ok(format!("File {} importato con successo. {} allarmi estratti e salvati come record separati.", file_name, alarm_count))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AlarmImportedFile {
    pub id: i32,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub import_date: String,
    pub records_count: i32,
}

#[tauri::command]
pub fn get_alarm_imported_files(table_name: String) -> Result<Vec<AlarmImportedFile>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let imports_table_name = format!("{}_alarm_imports", table_name);
    
    // Crea la tabella se non esiste
    let create_imports_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_type TEXT NOT NULL,
            import_date DATETIME DEFAULT CURRENT_TIMESTAMP,
            records_count INTEGER NOT NULL,
            UNIQUE(file_path)
        )",
        imports_table_name
    );
    conn.execute(&create_imports_table_sql, []).map_err(|e| e.to_string())?;
    
    let query_sql = format!(
        "SELECT id, file_path, file_name, file_type, import_date, records_count 
         FROM `{}` 
         ORDER BY import_date DESC",
        imports_table_name
    );
    
    let mut stmt = conn.prepare(&query_sql).map_err(|e| e.to_string())?;
    let file_iter = stmt.query_map([], |row| {
        Ok(AlarmImportedFile {
            id: row.get("id")?,
            file_path: row.get("file_path")?,
            file_name: row.get("file_name")?,
            file_type: row.get("file_type")?,
            import_date: row.get("import_date")?,
            records_count: row.get("records_count")?,
        })
    }).map_err(|e| e.to_string())?;
    
    let mut files = Vec::new();
    for file in file_iter {
        files.push(file.map_err(|e| e.to_string())?);
    }
    
    Ok(files)
}

// Delete an alarm record
#[tauri::command]
pub fn delete_alarm(table_name: String, alarm_id: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let alarms_table_name = format!("{}_alarms", table_name);
    let id: i64 = alarm_id.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    
    conn.execute(
        &format!("DELETE FROM `{}` WHERE id = ?", alarms_table_name),
        [id]
    ).map_err(|e| e.to_string())?;
    
    Ok("Allarme eliminato con successo.".to_string())
}

// Delete entire alarms database (drop table)
#[tauri::command]
pub fn delete_alarms_database(table_name: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Drop the alarms table
    conn.execute(
        &format!("DROP TABLE IF EXISTS `{}`", table_name),
        []
    ).map_err(|e| e.to_string())?;
    
    // Also drop the imported files tracking table
    let imported_table = format!("{}_imported_files", table_name.replace("_alarms", ""));
    conn.execute(
        &format!("DROP TABLE IF EXISTS `{}`", imported_table),
        []
    ).map_err(|e| e.to_string())?;
    
    Ok("Database allarmi eliminato con successo.".to_string())
}

// Add a new alarm record
#[tauri::command]
pub fn add_alarm(table_name: String, alarm_data: HashMap<String, String>) -> Result<i64, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let alarms_table_name = format!("{}_alarms", table_name);
    
    // Verifica e aggiungi colonne mancanti per tabelle esistenti
    ensure_alarm_columns_exist(&conn, &alarms_table_name)?;
    
    // Build insert statement dynamically
    let columns: Vec<&str> = alarm_data.keys().map(|s| s.as_str()).collect();
    let placeholders: Vec<&str> = columns.iter().map(|_| "?").collect();
    let values: Vec<&String> = alarm_data.values().collect();
    
    let sql = format!(
        "INSERT INTO `{}` ({}) VALUES ({})",
        alarms_table_name,
        columns.join(", "),
        placeholders.join(", ")
    );
    
    conn.execute(&sql, rusqlite::params_from_iter(values)).map_err(|e| e.to_string())?;
    
    let last_id = conn.last_insert_rowid();
    Ok(last_id)
}

// Update an alarm record
#[tauri::command]
pub fn update_alarm(table_name: String, alarm_id: String, updates: HashMap<String, String>) -> Result<String, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let alarms_table_name = format!("{}_alarms", table_name);
    
    // Verifica e aggiungi colonne mancanti per tabelle esistenti
    ensure_alarm_columns_exist(&conn, &alarms_table_name)?;
    
    let id: i64 = alarm_id.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    
    // Build update statement dynamically
    let set_clause: Vec<String> = updates.keys().map(|k| format!("{} = ?", k)).collect();
    let mut values: Vec<String> = updates.values().cloned().collect();
    values.push(id.to_string());
    
    let sql = format!(
        "UPDATE `{}` SET {} WHERE id = ?",
        alarms_table_name,
        set_clause.join(", ")
    );
    
    // Convert values to references for params
    let value_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    
    conn.execute(&sql, value_refs.as_slice()).map_err(|e| e.to_string())?;
    
    Ok("Allarme aggiornato con successo.".to_string())
}

// Get a single alarm by ID
#[tauri::command]
pub fn get_alarm(table_name: String, alarm_id: String) -> Result<HashMap<String, String>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let alarms_table_name = format!("{}_alarms", table_name);
    let id: i64 = alarm_id.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
    
    // Get column names first
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", alarms_table_name)).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.query_map([], |row| Ok(row.get::<_, String>(1)?))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    
    let sql = format!(
        "SELECT {} FROM `{}` WHERE id = ?",
        columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", "),
        alarms_table_name
    );
    
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let result = stmt.query_row([id], |row| {
        let mut map = HashMap::new();
        for (i, col) in columns.iter().enumerate() {
            let val: String = if col == "id" {
                match row.get::<_, i64>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => row.get::<_, String>(i).unwrap_or_else(|_| "".to_string()),
                }
            } else {
                row.get::<_, String>(i).unwrap_or_else(|_| "".to_string())
            };
            map.insert(col.clone(), val);
        }
        Ok(map)
    }).map_err(|e| e.to_string())?;
    
    Ok(result)
}

// Get translation keys for a project with all language translations
#[tauri::command]
pub fn get_translation_keys(table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Check if table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&table_name],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);
    
    if !table_exists {
        return Ok(Vec::new());
    }
    
    // Get column names to find language columns
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(`{}`);", table_name)).map_err(|e| e.to_string())?;
    let columns: Vec<String> = stmt.query_map([], |row| Ok(row.get::<_, String>(1)?))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    
    // Find all language columns (not id, key, keys_project, key_files, etc.)
    let tech_columns = vec!["id", "key", "keys_project", "key_files", "image_path", "order", "order_index", "project_id", "file_path", "source_file", "created_at", "updated_at"];
    let lang_columns: Vec<String> = columns.iter()
        .filter(|c| !tech_columns.contains(&c.as_str()) && !c.ends_with("_id") && !c.contains("path"))
        .cloned()
        .collect();
    
    // Build SQL to get keys with all language columns
    let mut select_cols = vec!["id".to_string(), "key".to_string()];
    for lang in &lang_columns {
        select_cols.push(format!("`{}`", lang));
    }
    
    let sql = format!("SELECT {} FROM `{}` WHERE key IS NOT NULL AND key != '' ORDER BY key", select_cols.join(", "), table_name);
    
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        let mut map = HashMap::new();
        map.insert("id".to_string(), row.get::<_, i64>(0)?.to_string());
        map.insert("key".to_string(), row.get::<_, String>(1)?);
        
        // Get all language translations
        for (i, lang) in lang_columns.iter().enumerate() {
            let val: String = row.get::<_, String>(2 + i).unwrap_or_default();
            map.insert(lang.clone(), val);
        }
        
        // Store available languages as comma-separated string
        map.insert("_languages".to_string(), lang_columns.join(","));
        
        Ok(map)
    }).map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    
    Ok(result)
}

/// Export alarms to .hmialr file with backup of existing files
#[tauri::command]
pub fn export_alarms(table_name: String) -> Result<String, String> {
    use crate::get_table_info_internal;
    
    // Get project info to find the project path
    let info = get_table_info_internal(table_name.clone())?;
    let project_path = info.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if project_path.is_empty() {
        return Err("Percorso progetto non disponibile".to_string());
    }
    
    // Find project name from .hmiprj or .movprj file
    let mut project_name = table_name.clone();
    for entry in fs::read_dir(&project_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let file_name_lower = file_name.to_lowercase();
                if file_name_lower.ends_with(".hmiprj") || file_name_lower.ends_with(".movprj") {
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
    
    // Create backup folder with timestamp
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Local> = now.into();
    let timestamp = datetime.format("%Y%m%d_%H%M%S").to_string();
    let backup_dir = Path::new(&project_path).join(format!("alarm_backups_{}", timestamp));
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    
    // Backup existing .hmialr files
    let mut backed_up_files = Vec::new();
    for entry in fs::read_dir(&project_path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                let file_name_lower = file_name.to_lowercase();
                if file_name_lower.ends_with(".hmialr") {
                    let dest = backup_dir.join(file_name);
                    // Copy the file to backup (don't delete original yet)
                    fs::copy(&path, &dest).map_err(|e| e.to_string())?;
                    backed_up_files.push(file_name.to_string());
                }
            }
        }
    }
    
    // Load alarms from database
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let alarms_table_name = format!("{}_alarms", table_name);
    
    // Check if alarms table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&alarms_table_name],
        |row| row.get::<_, i32>(0)
    ).map(|count| count > 0).unwrap_or(false);
    
    if !table_exists {
        return Err("Tabella allarmi non trovata".to_string());
    }
    
    // Verifica e aggiungi colonne mancanti per tabelle esistenti
    ensure_alarm_columns_exist(&conn, &alarms_table_name)?;
    
    // Get all alarms from database
    let query = format!(
        "SELECT alarm_name, device, variable, area, enabled, threshold_name, threshold_title, threshold_help, 
                severity, condition, threshold_value, sec_delay, support_ack, support_reset, log, print, beep_enabled, 
                back_color, text_color, blink_back_color, blink_text_color, blink_on_new_alarm, source_file
         FROM `{}` ORDER BY id",
        alarms_table_name
    );
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    struct AlarmRecord {
        alarm_name: String,
        device: String,
        variable: String,
        area: String,
        enabled: String,
        threshold_name: String,
        threshold_title: String,
        threshold_help: String,
        severity: String,
        condition: String,
        threshold_value: String,
        sec_delay: String,
        support_ack: String,
        support_reset: String,
        log: String,
        print: String,
        beep_enabled: String,
        back_color: String,
        text_color: String,
        blink_back_color: String,
        blink_text_color: String,
        blink_on_new_alarm: String,
        source_file: String,
    }
    
    let alarms_iter = stmt.query_map([], |row| {
        Ok(AlarmRecord {
            alarm_name: row.get::<_, String>(0).unwrap_or_default(),
            device: row.get::<_, String>(1).unwrap_or_default(),
            variable: row.get::<_, String>(2).unwrap_or_default(),
            area: row.get::<_, String>(3).unwrap_or_default(),
            enabled: row.get::<_, String>(4).unwrap_or_default(),
            threshold_name: row.get::<_, String>(5).unwrap_or_default(),
            threshold_title: row.get::<_, String>(6).unwrap_or_default(),
            threshold_help: row.get::<_, String>(7).unwrap_or_default(),
            severity: row.get::<_, String>(8).unwrap_or_default(),
            condition: row.get::<_, String>(9).unwrap_or_default(),
            threshold_value: row.get::<_, String>(10).unwrap_or_default(),
            sec_delay: row.get::<_, String>(11).unwrap_or_default(),
            support_ack: row.get::<_, String>(12).unwrap_or_default(),
            support_reset: row.get::<_, String>(13).unwrap_or_default(),
            log: row.get::<_, String>(14).unwrap_or_default(),
            print: row.get::<_, String>(15).unwrap_or_default(),
            beep_enabled: row.get::<_, String>(16).unwrap_or_default(),
            back_color: row.get::<_, String>(17).unwrap_or_default(),
            text_color: row.get::<_, String>(18).unwrap_or_default(),
            blink_back_color: row.get::<_, String>(19).unwrap_or_default(),
            blink_text_color: row.get::<_, String>(20).unwrap_or_default(),
            blink_on_new_alarm: row.get::<_, String>(21).unwrap_or_default(),
            source_file: row.get::<_, String>(22).unwrap_or_default(),
        })
    }).map_err(|e| e.to_string())?;
    
    // Group alarms by source file
    let mut alarms_by_file: HashMap<String, Vec<AlarmRecord>> = HashMap::new();
    
    for alarm_result in alarms_iter {
        let alarm = alarm_result.map_err(|e| e.to_string())?;
        let source = if alarm.source_file.is_empty() {
            format!("{}.hmialr", project_name)
        } else {
            alarm.source_file.clone()
        };
        alarms_by_file.entry(source).or_insert_with(Vec::new).push(alarm);
    }
    
    // If no alarms grouped by file, create a single default file
    if alarms_by_file.is_empty() {
        return Err("Nessun allarme da esportare".to_string());
    }
    
    // Generate XML for each file in Premium HMI format
    let mut exported_files = Vec::new();
    
    for (file_name, alarms) in &alarms_by_file {
        let mut xml_content = String::new();
        xml_content.push_str("<?xml version=\"1.0\" encoding=\"UTF-16\" ?>\n");
        xml_content.push_str("<Alarms>\n");
        xml_content.push_str("<AlarmList>\n");
        
        for alarm in alarms {
            // Check if alarm has threshold data
            let has_threshold = !alarm.threshold_name.is_empty() || 
                               !alarm.threshold_title.is_empty() || 
                               !alarm.condition.is_empty();
            
            xml_content.push_str("<Alarm>\n");
            xml_content.push_str(&format!(
                "<Name Device=\"{}\" Variable=\"{}\" Area=\"{}\" ThresholdExclusive=\"0\" Enabled=\"{}\" OnQualityGood=\"1\" VariableDuration=\"0\" EnableVariable=\"\" EnableDispMsg=\"\" Hysteresis=\"0\" EventsCache=\"1204\">{}</Name>\n",
                escape_xml(&alarm.device),
                escape_xml(&alarm.variable),
                escape_xml(&alarm.area),
                if alarm.enabled == "1" || alarm.enabled.to_lowercase() == "true" { "1" } else { "0" },
                escape_xml(&alarm.alarm_name)
            ));
            
            if has_threshold {
                xml_content.push_str("<ThresholdList>\n");
                xml_content.push_str("<Threshold>\n");
                xml_content.push_str(&format!(
                    "<Name Area=\"\" Title=\"{}\" Help=\"{}\" DurationFormat=\"\" ReadAccessLevel=\"4294901760\" WriteAccessLevel=\"4294901760\">{}</Name>\n",
                    escape_xml(&alarm.threshold_title),
                    escape_xml(&alarm.threshold_help),
                    escape_xml(&alarm.threshold_name)
                ));
                xml_content.push_str(&format!(
                    "<Execution Condition=\"{}\" Threshold=\"{}\" ThresholdVar=\"\" ThresholdLow=\"0\" ThresholdVarLow=\"\" VariableStatus=\"\" Severity=\"{}\" SeverityVar=\"\" SecDelay=\"{}\" RunCommandAtServer=\"0\"/>\n",
                    if alarm.condition.is_empty() { "0" } else { &alarm.condition },
                    if alarm.threshold_value.is_empty() { "1" } else { &alarm.threshold_value },
                    if alarm.severity.is_empty() { "1" } else { &alarm.severity },
                    if alarm.sec_delay.is_empty() { "0" } else { &alarm.sec_delay }
                ));
                xml_content.push_str("<Commands/>\n");
                xml_content.push_str("<CommandsOn/>\n");
                xml_content.push_str("<CommandsAck/>\n");
                xml_content.push_str("<CommandsReset/>\n");
                xml_content.push_str("<CommandsOff/>\n");
                xml_content.push_str(&format!(
                    "<Style BackColor=\"{}\" TextColor=\"{}\" BlinkBackColor=\"{}\" BlinkTextColor=\"{}\" Print=\"{}\" Log=\"{}\" BlinkOnNewAlarm=\"{}\" VarTimeStamp=\"0\" SupportAck=\"{}\" SupportReset=\"{}\" SupportResetConditionOn=\"0\" BmpFile=\"\" SndFile=\"\" BeepEnabled=\"{}\" PlaysoundContinuosly=\"0\" CommentOnAck=\"0\"/>\n",
                    if alarm.back_color.is_empty() { "4294967295" } else { &alarm.back_color },
                    if alarm.text_color.is_empty() { "4294967295" } else { &alarm.text_color },
                    if alarm.blink_back_color.is_empty() { "4294967295" } else { &alarm.blink_back_color },
                    if alarm.blink_text_color.is_empty() { "4294967295" } else { &alarm.blink_text_color },
                    if alarm.print == "1" || alarm.print.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.log == "1" || alarm.log.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.blink_on_new_alarm == "1" || alarm.blink_on_new_alarm.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.support_ack == "1" || alarm.support_ack.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.support_reset == "1" || alarm.support_reset.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.beep_enabled == "1" || alarm.beep_enabled.to_lowercase() == "true" { "1" } else { "0" }
                ));
                xml_content.push_str("<Recipient Attachment=\"\" DispatchingText=\"\"/>\n");
                xml_content.push_str("<SendEmail SendON=\"0\" SendACK=\"0\" SendRESET=\"0\" SendOFF=\"0\"/>\n");
                xml_content.push_str("<SendSMS SendON=\"0\" SendACK=\"0\" SendRESET=\"0\" SendOFF=\"0\"/>\n");
                xml_content.push_str("</Threshold>\n");
                xml_content.push_str("</ThresholdList>\n");
            }
            
            xml_content.push_str("</Alarm>\n");
        }
        
        xml_content.push_str("</AlarmList>\n");
        xml_content.push_str("</Alarms>\n");
        
        // Write file with UTF-16 LE encoding (with BOM)
        let output_path = Path::new(&project_path).join(file_name);
        
        // Convert to UTF-16 LE with BOM
        let mut utf16_bytes: Vec<u8> = vec![0xFF, 0xFE]; // UTF-16 LE BOM
        for ch in xml_content.encode_utf16() {
            utf16_bytes.push((ch & 0xFF) as u8);
            utf16_bytes.push((ch >> 8) as u8);
        }
        
        fs::write(&output_path, utf16_bytes).map_err(|e| e.to_string())?;
        
        exported_files.push(format!("{} ({} allarmi)", file_name, alarms.len()));
    }
    
    let backup_msg = if backed_up_files.is_empty() {
        String::new()
    } else {
        format!(" Backup creato in: {}", backup_dir.display())
    };
    
    Ok(format!("Esportazione completata. File creati: {}.{}", exported_files.join(", "), backup_msg))
}

// Helper function to escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
     .replace("\"", "&quot;")
     .replace("'", "&apos;")
}

/// Export alarms directly to machine directory with backup
#[tauri::command]
pub fn export_alarms_to_machine(table_name: String, machine_path: String) -> Result<String, String> {
    use crate::get_table_info_internal;
    
    // Validate machine path
    if machine_path.is_empty() {
        return Err("Percorso macchina non configurato. Impostalo nelle impostazioni.".to_string());
    }
    
    let machine_dir = Path::new(&machine_path);
    if !machine_dir.exists() {
        return Err(format!("Il percorso macchina non esiste: {}", machine_path));
    }
    
    // Get project info to find the project name
    let info = get_table_info_internal(table_name.clone())?;
    let project_path = info.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
    
    // Find project name from .hmiprj or .movprj file
    let mut project_name = table_name.clone();
    if !project_path.is_empty() {
        if let Ok(entries) = fs::read_dir(&project_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        let file_name_lower = file_name.to_lowercase();
                        if file_name_lower.ends_with(".hmiprj") || file_name_lower.ends_with(".movprj") {
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
        }
    }
    
    // Create backup folder with timestamp in machine directory
    let now = std::time::SystemTime::now();
    let datetime: chrono::DateTime<chrono::Local> = now.into();
    let timestamp = datetime.format("%Y%m%d_%H%M%S").to_string();
    let backup_dir = machine_dir.join(format!("alarm_backups_{}", timestamp));
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
    
    // Backup existing .hmialr files in machine directory
    let mut backed_up_files = Vec::new();
    if let Ok(entries) = fs::read_dir(machine_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    let file_name_lower = file_name.to_lowercase();
                    if file_name_lower.ends_with(".hmialr") {
                        let dest = backup_dir.join(file_name);
                        if fs::copy(&path, &dest).is_ok() {
                            backed_up_files.push(file_name.to_string());
                        }
                    }
                }
            }
        }
    }
    
    // Load alarms from database
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let alarms_table_name = format!("{}_alarms", table_name);
    
    // Check if alarms table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&alarms_table_name],
        |row| row.get::<_, i32>(0)
    ).map(|count| count > 0).unwrap_or(false);
    
    if !table_exists {
        return Err("Tabella allarmi non trovata".to_string());
    }
    
    // Ensure columns exist
    ensure_alarm_columns_exist(&conn, &alarms_table_name)?;
    
    // Get all alarms from database
    let query = format!(
        "SELECT alarm_name, device, variable, area, enabled, threshold_name, threshold_title, threshold_help, 
                severity, condition, threshold_value, sec_delay, support_ack, support_reset, log, print, beep_enabled, 
                back_color, text_color, blink_back_color, blink_text_color, blink_on_new_alarm, source_file
         FROM `{}` ORDER BY id",
        alarms_table_name
    );
    
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    
    struct AlarmRecord {
        alarm_name: String,
        device: String,
        variable: String,
        area: String,
        enabled: String,
        threshold_name: String,
        threshold_title: String,
        threshold_help: String,
        severity: String,
        condition: String,
        threshold_value: String,
        sec_delay: String,
        support_ack: String,
        support_reset: String,
        log: String,
        print: String,
        beep_enabled: String,
        back_color: String,
        text_color: String,
        blink_back_color: String,
        blink_text_color: String,
        blink_on_new_alarm: String,
        source_file: String,
    }
    
    let alarms_iter = stmt.query_map([], |row| {
        Ok(AlarmRecord {
            alarm_name: row.get::<_, String>(0).unwrap_or_default(),
            device: row.get::<_, String>(1).unwrap_or_default(),
            variable: row.get::<_, String>(2).unwrap_or_default(),
            area: row.get::<_, String>(3).unwrap_or_default(),
            enabled: row.get::<_, String>(4).unwrap_or_default(),
            threshold_name: row.get::<_, String>(5).unwrap_or_default(),
            threshold_title: row.get::<_, String>(6).unwrap_or_default(),
            threshold_help: row.get::<_, String>(7).unwrap_or_default(),
            severity: row.get::<_, String>(8).unwrap_or_default(),
            condition: row.get::<_, String>(9).unwrap_or_default(),
            threshold_value: row.get::<_, String>(10).unwrap_or_default(),
            sec_delay: row.get::<_, String>(11).unwrap_or_default(),
            support_ack: row.get::<_, String>(12).unwrap_or_default(),
            support_reset: row.get::<_, String>(13).unwrap_or_default(),
            log: row.get::<_, String>(14).unwrap_or_default(),
            print: row.get::<_, String>(15).unwrap_or_default(),
            beep_enabled: row.get::<_, String>(16).unwrap_or_default(),
            back_color: row.get::<_, String>(17).unwrap_or_default(),
            text_color: row.get::<_, String>(18).unwrap_or_default(),
            blink_back_color: row.get::<_, String>(19).unwrap_or_default(),
            blink_text_color: row.get::<_, String>(20).unwrap_or_default(),
            blink_on_new_alarm: row.get::<_, String>(21).unwrap_or_default(),
            source_file: row.get::<_, String>(22).unwrap_or_default(),
        })
    }).map_err(|e| e.to_string())?;
    
    // Group alarms by source file
    let mut alarms_by_file: HashMap<String, Vec<AlarmRecord>> = HashMap::new();
    
    for alarm_result in alarms_iter {
        let alarm = alarm_result.map_err(|e| e.to_string())?;
        let source = if alarm.source_file.is_empty() {
            format!("{}.hmialr", project_name)
        } else {
            alarm.source_file.clone()
        };
        alarms_by_file.entry(source).or_insert_with(Vec::new).push(alarm);
    }
    
    if alarms_by_file.is_empty() {
        return Err("Nessun allarme da esportare".to_string());
    }
    
    // Generate XML for each file in Premium HMI format
    let mut exported_files = Vec::new();
    
    for (file_name, alarms) in &alarms_by_file {
        let mut xml_content = String::new();
        xml_content.push_str("<?xml version=\"1.0\" encoding=\"UTF-16\" ?>\n");
        xml_content.push_str("<Alarms>\n");
        xml_content.push_str("<AlarmList>\n");
        
        for alarm in alarms {
            let has_threshold = !alarm.threshold_name.is_empty() || 
                               !alarm.threshold_title.is_empty() || 
                               !alarm.condition.is_empty();
            
            xml_content.push_str("<Alarm>\n");
            xml_content.push_str(&format!(
                "<Name Device=\"{}\" Variable=\"{}\" Area=\"{}\" ThresholdExclusive=\"0\" Enabled=\"{}\" OnQualityGood=\"1\" VariableDuration=\"0\" EnableVariable=\"\" EnableDispMsg=\"\" Hysteresis=\"0\" EventsCache=\"1204\">{}</Name>\n",
                escape_xml(&alarm.device),
                escape_xml(&alarm.variable),
                escape_xml(&alarm.area),
                if alarm.enabled == "1" || alarm.enabled.to_lowercase() == "true" { "1" } else { "0" },
                escape_xml(&alarm.alarm_name)
            ));
            
            if has_threshold {
                xml_content.push_str("<ThresholdList>\n");
                xml_content.push_str("<Threshold>\n");
                xml_content.push_str(&format!(
                    "<Name Area=\"\" Title=\"{}\" Help=\"{}\" DurationFormat=\"\" ReadAccessLevel=\"4294901760\" WriteAccessLevel=\"4294901760\">{}</Name>\n",
                    escape_xml(&alarm.threshold_title),
                    escape_xml(&alarm.threshold_help),
                    escape_xml(&alarm.threshold_name)
                ));
                xml_content.push_str(&format!(
                    "<Execution Condition=\"{}\" Threshold=\"{}\" ThresholdVar=\"\" ThresholdLow=\"0\" ThresholdVarLow=\"\" VariableStatus=\"\" Severity=\"{}\" SeverityVar=\"\" SecDelay=\"{}\" RunCommandAtServer=\"0\"/>\n",
                    if alarm.condition.is_empty() { "0" } else { &alarm.condition },
                    if alarm.threshold_value.is_empty() { "1" } else { &alarm.threshold_value },
                    if alarm.severity.is_empty() { "1" } else { &alarm.severity },
                    if alarm.sec_delay.is_empty() { "0" } else { &alarm.sec_delay }
                ));
                xml_content.push_str("<Commands/>\n");
                xml_content.push_str("<CommandsOn/>\n");
                xml_content.push_str("<CommandsAck/>\n");
                xml_content.push_str("<CommandsReset/>\n");
                xml_content.push_str("<CommandsOff/>\n");
                xml_content.push_str(&format!(
                    "<Style BackColor=\"{}\" TextColor=\"{}\" BlinkBackColor=\"{}\" BlinkTextColor=\"{}\" Print=\"{}\" Log=\"{}\" BlinkOnNewAlarm=\"{}\" VarTimeStamp=\"0\" SupportAck=\"{}\" SupportReset=\"{}\" SupportResetConditionOn=\"0\" BmpFile=\"\" SndFile=\"\" BeepEnabled=\"{}\" PlaysoundContinuosly=\"0\" CommentOnAck=\"0\"/>\n",
                    if alarm.back_color.is_empty() { "4294967295" } else { &alarm.back_color },
                    if alarm.text_color.is_empty() { "4294967295" } else { &alarm.text_color },
                    if alarm.blink_back_color.is_empty() { "4294967295" } else { &alarm.blink_back_color },
                    if alarm.blink_text_color.is_empty() { "4294967295" } else { &alarm.blink_text_color },
                    if alarm.print == "1" || alarm.print.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.log == "1" || alarm.log.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.blink_on_new_alarm == "1" || alarm.blink_on_new_alarm.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.support_ack == "1" || alarm.support_ack.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.support_reset == "1" || alarm.support_reset.to_lowercase() == "true" { "1" } else { "0" },
                    if alarm.beep_enabled == "1" || alarm.beep_enabled.to_lowercase() == "true" { "1" } else { "0" }
                ));
                xml_content.push_str("<Recipient Attachment=\"\" DispatchingText=\"\"/>\n");
                xml_content.push_str("<SendEmail SendON=\"0\" SendACK=\"0\" SendRESET=\"0\" SendOFF=\"0\"/>\n");
                xml_content.push_str("<SendSMS SendON=\"0\" SendACK=\"0\" SendRESET=\"0\" SendOFF=\"0\"/>\n");
                xml_content.push_str("</Threshold>\n");
                xml_content.push_str("</ThresholdList>\n");
            }
            
            xml_content.push_str("</Alarm>\n");
        }
        
        xml_content.push_str("</AlarmList>\n");
        xml_content.push_str("</Alarms>\n");
        
        // Write file with UTF-16 LE encoding (with BOM) to machine directory
        let output_path = machine_dir.join(file_name);
        
        // Convert to UTF-16 LE with BOM
        let mut utf16_bytes: Vec<u8> = vec![0xFF, 0xFE]; // UTF-16 LE BOM
        for ch in xml_content.encode_utf16() {
            utf16_bytes.push((ch & 0xFF) as u8);
            utf16_bytes.push((ch >> 8) as u8);
        }
        
        fs::write(&output_path, utf16_bytes).map_err(|e| e.to_string())?;
        
        exported_files.push(format!("{} ({} allarmi)", file_name, alarms.len()));
    }
    
    let backup_msg = if backed_up_files.is_empty() {
        String::new()
    } else {
        format!(" Backup creato in: {}", backup_dir.display())
    };
    
    Ok(format!("Caricamento in macchina completato. File: {}.{}", exported_files.join(", "), backup_msg))
}