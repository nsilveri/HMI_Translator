use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;
use std::collections::HashMap;

use crate::read_text_file_best_effort;

// Funzione helper per scansionare ricorsivamente le directory
fn scan_directory_recursive(path: &Path, variable_files: &mut Vec<HashMap<String, String>>, table_name: &str) -> Result<(), String> {
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_path = entry.path();
        
        if file_path.is_dir() {
            // Ricorsivamente scansiona le sottodirectory
            let _ = scan_directory_recursive(&file_path, variable_files, table_name);
        } else if file_path.is_file() {
            if let Some(file_name) = file_path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    let file_name_lower = file_name_str.to_lowercase();
                    
                    // Cerca file con estensione .hmirealtimedb
                    if file_name_lower.ends_with(".hmirealtimedb") {
                        let mut file_info = HashMap::new();
                        file_info.insert("file_name".to_string(), file_name_str.to_string());
                        file_info.insert("file_path".to_string(), file_path.to_string_lossy().to_string());
                        file_info.insert("file_type".to_string(), "hmirealtimedb".to_string());

                        println!("Found variable file: {}", file_name_str);
                        
                        // Check if file is already imported
                        let mut already_imported = false;
                        
                        // Check in the variable imports table directly
                        let db_path = "../data/projects.db";
                        if let Ok(conn) = Connection::open(db_path) {
                            let imports_table_name = format!("{}_variable_imports", table_name);
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
                            variable_files.push(file_info);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_variable_files_in_directory(directory_path: String, table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let path = Path::new(&directory_path);
    
    println!("Searching for variable files in: {}", directory_path);
    
    if !path.is_dir() {
        return Err(format!("Il percorso specificato non è una directory: {}", directory_path));
    }
    
    let mut variable_files = Vec::new();
    
    // Scansiona ricorsivamente la directory e le sottodirectory
    scan_directory_recursive(path, &mut variable_files, &table_name)?;
    
    println!("Total variable files found: {}", variable_files.len());
    
    Ok(variable_files)
}

// Struttura per rappresentare una variabile parsata dal file XML
#[derive(Debug, Clone)]
struct ParsedVariable {
    name: String,
    var_type: String,
    area_type: String,
    address: String,
    bit: String,
    group: String,
    description: String,
    shared: String,
    retentive: String,
    dynamic_settings: String,
    struct_type: String,
    initial_value: String,
    enable_trace: String,
    enable_opcua_server: String,
    enable_opc_server: String,
    enable_network_client: String,
    enable_map_realtime_to_db: String,
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

// Funzione helper per estrarre il contenuto di un tag XML (il testo tra > e </)
fn extract_tag_content(content: &str, tag_name: &str) -> String {
    let open_tag = format!("<{}", tag_name);
    let close_tag = format!("</{}>", tag_name);
    
    if let Some(start) = content.find(&open_tag) {
        // Trova la fine del tag di apertura (dopo >)
        if let Some(tag_end) = content[start..].find('>') {
            let content_start = start + tag_end + 1;
            if let Some(end) = content[content_start..].find(&close_tag) {
                return content[content_start..content_start + end].to_string();
            }
        }
    }
    String::new()
}

// Funzione per parsare le variabili dal contenuto XML
fn parse_variables_from_xml(content: &str) -> Vec<ParsedVariable> {
    let mut variables = Vec::new();
    
    // Splitta per ogni <Variable>
    let variable_blocks: Vec<&str> = content.split("<Variable>").collect();
    
    for (i, block) in variable_blocks.iter().enumerate() {
        if i == 0 { continue; } // Salta la parte prima del primo <Variable>
        
        // Trova la fine del blocco Variable
        let block_content = if let Some(end) = block.find("</Variable>") {
            &block[..end]
        } else {
            continue;
        };
        
        // Trova il tag <Name> che contiene tutti gli attributi della variabile
        if let Some(name_start) = block_content.find("<Name") {
            if let Some(name_end) = block_content[name_start..].find(">") {
                let name_tag = &block_content[name_start..name_start + name_end + 1];
                
                // Estrai il nome della variabile (contenuto tra > e </Name>)
                let var_name = if let Some(content_end) = block_content[name_start + name_end + 1..].find("</Name>") {
                    block_content[name_start + name_end + 1..name_start + name_end + 1 + content_end].to_string()
                } else {
                    String::new()
                };
                
                if var_name.is_empty() {
                    continue;
                }
                
                let variable = ParsedVariable {
                    name: var_name,
                    var_type: extract_attribute(name_tag, "Type"),
                    area_type: extract_attribute(name_tag, "AreaType"),
                    address: extract_attribute(name_tag, "Address"),
                    bit: extract_attribute(name_tag, "Bit"),
                    group: extract_attribute(name_tag, "Group"),
                    description: extract_attribute(name_tag, "Description"),
                    shared: extract_attribute(name_tag, "Shared"),
                    retentive: extract_attribute(name_tag, "Retentive"),
                    dynamic_settings: extract_attribute(name_tag, "DynamicSettings"),
                    struct_type: extract_attribute(name_tag, "StructType"),
                    initial_value: extract_attribute(name_tag, "InitialValue"),
                    enable_trace: extract_tag_content(block_content, "EnableTrace"),
                    enable_opcua_server: extract_tag_content(block_content, "EnableOPCUAServer"),
                    enable_opc_server: extract_tag_content(block_content, "EnableOPCServer"),
                    enable_network_client: extract_tag_content(block_content, "EnableNetworkClient"),
                    enable_map_realtime_to_db: extract_tag_content(block_content, "EnableMapRealTimeToDB"),
                };
                
                variables.push(variable);
            }
        }
    }
    
    variables
}

// Struttura per rappresentare una struttura dati parsata dal file XML
#[derive(Debug, Clone)]
struct ParsedStructure {
    name: String,
    description: String,
    members: Vec<ParsedStructMember>,
}

#[derive(Debug, Clone)]
struct ParsedStructMember {
    name: String,
    member_type: String,
}

// Funzione per parsare le strutture dal contenuto XML
fn parse_structures_from_xml(content: &str) -> Vec<ParsedStructure> {
    let mut structures = Vec::new();
    
    // Trova la sezione StructureList
    if let Some(struct_list_start) = content.find("<StructureList>") {
        if let Some(struct_list_end) = content.find("</StructureList>") {
            let struct_list_content = &content[struct_list_start..struct_list_end];
            
            // Splitta per ogni <Structure>
            let structure_blocks: Vec<&str> = struct_list_content.split("<Structure>").collect();
            
            for (i, block) in structure_blocks.iter().enumerate() {
                if i == 0 { continue; }
                
                let block_content = if let Some(end) = block.find("</Structure>") {
                    &block[..end]
                } else {
                    continue;
                };
                
                // Estrai nome e descrizione dalla struttura
                let struct_name = if let Some(name_start) = block_content.find("<Name") {
                    if let Some(name_end) = block_content[name_start..].find(">") {
                        let name_tag = &block_content[name_start..name_start + name_end + 1];
                        let description = extract_attribute(name_tag, "Description");
                        
                        if let Some(content_end) = block_content[name_start + name_end + 1..].find("</Name>") {
                            let name = block_content[name_start + name_end + 1..name_start + name_end + 1 + content_end].to_string();
                            (name, description)
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                
                // Parsa i membri
                let mut members = Vec::new();
                let member_blocks: Vec<&str> = block_content.split("<Member>").collect();
                
                for (j, member_block) in member_blocks.iter().enumerate() {
                    if j == 0 { continue; }
                    
                    if let Some(member_name_start) = member_block.find("<Name") {
                        if let Some(member_name_end) = member_block[member_name_start..].find(">") {
                            let member_name_tag = &member_block[member_name_start..member_name_start + member_name_end + 1];
                            let member_type = extract_attribute(member_name_tag, "Type");
                            
                            if let Some(content_end) = member_block[member_name_start + member_name_end + 1..].find("</Name>") {
                                let member_name = member_block[member_name_start + member_name_end + 1..member_name_start + member_name_end + 1 + content_end].to_string();
                                
                                members.push(ParsedStructMember {
                                    name: member_name,
                                    member_type,
                                });
                            }
                        }
                    }
                }
                
                structures.push(ParsedStructure {
                    name: struct_name.0,
                    description: struct_name.1,
                    members,
                });
            }
        }
    }
    
    structures
}

#[tauri::command]
pub fn import_variable_file(table_name: String, file_path: String) -> Result<String, String> {
    let path = Path::new(&file_path);
    
    if !path.exists() {
        return Err(format!("Il file non esiste: {}", file_path));
    }
    
    // Leggi il contenuto del file
    let content = read_text_file_best_effort(path).map_err(|e| e.to_string())?;
    
    // Parsa le variabili
    let variables = parse_variables_from_xml(&content);
    
    if variables.is_empty() {
        return Err("Nessuna variabile trovata nel file".to_string());
    }
    
    // Parsa le strutture
    let structures = parse_structures_from_xml(&content);
    
    // Connetti al database
    let db_path = "../data/projects.db";
    fs::create_dir_all("../data").map_err(|e| e.to_string())?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    // Crea la tabella delle variabili se non esiste
    let variables_table_name = format!("{}_variables", table_name);
    let create_variables_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            var_type TEXT,
            area_type TEXT,
            address TEXT,
            bit TEXT,
            var_group TEXT,
            description TEXT,
            shared TEXT,
            retentive TEXT,
            dynamic_settings TEXT,
            struct_type TEXT,
            initial_value TEXT,
            enable_trace TEXT,
            enable_opcua_server TEXT,
            enable_opc_server TEXT,
            enable_network_client TEXT,
            enable_map_realtime_to_db TEXT,
            source_file TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name)
        )",
        variables_table_name
    );
    conn.execute(&create_variables_table_sql, []).map_err(|e| e.to_string())?;
    
    // Crea la tabella delle strutture se non esiste
    let structures_table_name = format!("{}_structures", table_name);
    let create_structures_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            description TEXT,
            source_file TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(name)
        )",
        structures_table_name
    );
    conn.execute(&create_structures_table_sql, []).map_err(|e| e.to_string())?;
    
    // Crea la tabella dei membri delle strutture se non esiste
    let struct_members_table_name = format!("{}_struct_members", table_name);
    let create_struct_members_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            structure_name TEXT NOT NULL,
            member_name TEXT NOT NULL,
            member_type TEXT,
            source_file TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(structure_name, member_name)
        )",
        struct_members_table_name
    );
    conn.execute(&create_struct_members_table_sql, []).map_err(|e| e.to_string())?;
    
    // Crea la tabella per tracciare i file importati se non esiste
    let imports_table_name = format!("{}_variable_imports", table_name);
    let create_imports_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS `{}` (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            file_name TEXT NOT NULL,
            variables_count INTEGER,
            structures_count INTEGER,
            import_date DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        imports_table_name
    );
    conn.execute(&create_imports_table_sql, []).map_err(|e| e.to_string())?;
    
    // Inserisci le variabili
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
    let mut inserted_vars = 0;
    let mut updated_vars = 0;
    
    for var in &variables {
        let insert_sql = format!(
            "INSERT INTO `{}` (name, var_type, area_type, address, bit, var_group, description, shared, retentive, dynamic_settings, struct_type, initial_value, enable_trace, enable_opcua_server, enable_opc_server, enable_network_client, enable_map_realtime_to_db, source_file)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET
                var_type = excluded.var_type,
                area_type = excluded.area_type,
                address = excluded.address,
                bit = excluded.bit,
                var_group = excluded.var_group,
                description = excluded.description,
                shared = excluded.shared,
                retentive = excluded.retentive,
                dynamic_settings = excluded.dynamic_settings,
                struct_type = excluded.struct_type,
                initial_value = excluded.initial_value,
                enable_trace = excluded.enable_trace,
                enable_opcua_server = excluded.enable_opcua_server,
                enable_opc_server = excluded.enable_opc_server,
                enable_network_client = excluded.enable_network_client,
                enable_map_realtime_to_db = excluded.enable_map_realtime_to_db,
                source_file = excluded.source_file",
            variables_table_name
        );
        
        match conn.execute(&insert_sql, params![
            var.name,
            var.var_type,
            var.area_type,
            var.address,
            var.bit,
            var.group,
            var.description,
            var.shared,
            var.retentive,
            var.dynamic_settings,
            var.struct_type,
            var.initial_value,
            var.enable_trace,
            var.enable_opcua_server,
            var.enable_opc_server,
            var.enable_network_client,
            var.enable_map_realtime_to_db,
            file_name
        ]) {
            Ok(changes) => {
                if changes > 0 {
                    inserted_vars += 1;
                } else {
                    updated_vars += 1;
                }
            },
            Err(e) => {
                eprintln!("Error inserting variable {}: {}", var.name, e);
                updated_vars += 1;
            }
        }
    }
    
    // Inserisci le strutture e i loro membri
    let mut inserted_structs = 0;
    
    for structure in &structures {
        let insert_struct_sql = format!(
            "INSERT OR REPLACE INTO `{}` (name, description, source_file) VALUES (?, ?, ?)",
            structures_table_name
        );
        
        if conn.execute(&insert_struct_sql, params![
            structure.name,
            structure.description,
            file_name
        ]).is_ok() {
            inserted_structs += 1;
        }
        
        // Inserisci i membri
        for member in &structure.members {
            let insert_member_sql = format!(
                "INSERT OR REPLACE INTO `{}` (structure_name, member_name, member_type, source_file) VALUES (?, ?, ?, ?)",
                struct_members_table_name
            );
            
            let _ = conn.execute(&insert_member_sql, params![
                structure.name,
                member.name,
                member.member_type,
                file_name
            ]);
        }
    }
    
    // Registra l'importazione
    let insert_import_sql = format!(
        "INSERT OR REPLACE INTO `{}` (file_path, file_name, variables_count, structures_count) VALUES (?, ?, ?, ?)",
        imports_table_name
    );
    
    conn.execute(&insert_import_sql, params![
        file_path,
        file_name,
        variables.len() as i32,
        structures.len() as i32
    ]).map_err(|e| e.to_string())?;
    
    Ok(format!(
        "Importazione completata: {} variabili, {} strutture importate dal file '{}'",
        inserted_vars + updated_vars,
        inserted_structs,
        file_name
    ))
}

#[tauri::command]
pub fn get_variable_imported_files(table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let imports_table_name = format!("{}_variable_imports", table_name);
    
    // Check if table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&imports_table_name],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);
    
    if !table_exists {
        return Ok(Vec::new());
    }
    
    let mut stmt = conn.prepare(&format!(
        "SELECT file_path, file_name, variables_count, structures_count, import_date FROM `{}`",
        imports_table_name
    )).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
        let mut map = HashMap::new();
        map.insert("file_path".to_string(), row.get::<_, String>(0)?);
        map.insert("file_name".to_string(), row.get::<_, String>(1)?);
        map.insert("variables_count".to_string(), row.get::<_, i32>(2)?.to_string());
        map.insert("structures_count".to_string(), row.get::<_, i32>(3)?.to_string());
        map.insert("import_date".to_string(), row.get::<_, String>(4)?);
        Ok(map)
    }).map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    
    Ok(result)
}

#[tauri::command]
pub fn get_variables(table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let variables_table_name = format!("{}_variables", table_name);
    
    // Check if table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&variables_table_name],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);
    
    if !table_exists {
        return Ok(Vec::new());
    }
    
    let mut stmt = conn.prepare(&format!(
        "SELECT id, name, var_type, area_type, address, bit, var_group, description, shared, retentive, dynamic_settings, struct_type, initial_value, source_file FROM `{}`",
        variables_table_name
    )).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
        let mut map = HashMap::new();
        map.insert("id".to_string(), row.get::<_, i32>(0)?.to_string());
        map.insert("name".to_string(), row.get::<_, String>(1)?);
        map.insert("var_type".to_string(), row.get::<_, Option<String>>(2)?.unwrap_or_default());
        map.insert("area_type".to_string(), row.get::<_, Option<String>>(3)?.unwrap_or_default());
        map.insert("address".to_string(), row.get::<_, Option<String>>(4)?.unwrap_or_default());
        map.insert("bit".to_string(), row.get::<_, Option<String>>(5)?.unwrap_or_default());
        map.insert("var_group".to_string(), row.get::<_, Option<String>>(6)?.unwrap_or_default());
        map.insert("description".to_string(), row.get::<_, Option<String>>(7)?.unwrap_or_default());
        map.insert("shared".to_string(), row.get::<_, Option<String>>(8)?.unwrap_or_default());
        map.insert("retentive".to_string(), row.get::<_, Option<String>>(9)?.unwrap_or_default());
        map.insert("dynamic_settings".to_string(), row.get::<_, Option<String>>(10)?.unwrap_or_default());
        map.insert("struct_type".to_string(), row.get::<_, Option<String>>(11)?.unwrap_or_default());
        map.insert("initial_value".to_string(), row.get::<_, Option<String>>(12)?.unwrap_or_default());
        map.insert("source_file".to_string(), row.get::<_, Option<String>>(13)?.unwrap_or_default());
        Ok(map)
    }).map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    
    Ok(result)
}

#[tauri::command]
pub fn get_structures(table_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let structures_table_name = format!("{}_structures", table_name);
    
    // Check if table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&structures_table_name],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);
    
    if !table_exists {
        return Ok(Vec::new());
    }
    
    let mut stmt = conn.prepare(&format!(
        "SELECT id, name, description, source_file FROM `{}`",
        structures_table_name
    )).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([], |row| {
        let mut map = HashMap::new();
        map.insert("id".to_string(), row.get::<_, i32>(0)?.to_string());
        map.insert("name".to_string(), row.get::<_, String>(1)?);
        map.insert("description".to_string(), row.get::<_, Option<String>>(2)?.unwrap_or_default());
        map.insert("source_file".to_string(), row.get::<_, Option<String>>(3)?.unwrap_or_default());
        Ok(map)
    }).map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    
    Ok(result)
}

#[tauri::command]
pub fn get_structure_members(table_name: String, structure_name: String) -> Result<Vec<HashMap<String, String>>, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let struct_members_table_name = format!("{}_struct_members", table_name);
    
    // Check if table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [&struct_members_table_name],
        |row| Ok(row.get::<_, i32>(0)? > 0)
    ).unwrap_or(false);
    
    if !table_exists {
        return Ok(Vec::new());
    }
    
    let mut stmt = conn.prepare(&format!(
        "SELECT id, structure_name, member_name, member_type FROM `{}` WHERE structure_name = ?",
        struct_members_table_name
    )).map_err(|e| e.to_string())?;
    
    let rows = stmt.query_map([&structure_name], |row| {
        let mut map = HashMap::new();
        map.insert("id".to_string(), row.get::<_, i32>(0)?.to_string());
        map.insert("structure_name".to_string(), row.get::<_, String>(1)?);
        map.insert("member_name".to_string(), row.get::<_, String>(2)?);
        map.insert("member_type".to_string(), row.get::<_, Option<String>>(3)?.unwrap_or_default());
        Ok(map)
    }).map_err(|e| e.to_string())?;
    
    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    
    Ok(result)
}

#[tauri::command]
pub fn delete_variable(table_name: String, variable_id: i32) -> Result<String, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let variables_table_name = format!("{}_variables", table_name);
    
    let delete_sql = format!("DELETE FROM `{}` WHERE id = ?", variables_table_name);
    conn.execute(&delete_sql, params![variable_id]).map_err(|e| e.to_string())?;
    
    Ok("Variabile eliminata con successo".to_string())
}

#[tauri::command]
pub fn update_variable(table_name: String, variable_id: i32, field: String, value: String) -> Result<String, String> {
    let db_path = "../data/projects.db";
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    
    let variables_table_name = format!("{}_variables", table_name);
    
    // Validate field name to prevent SQL injection
    let valid_fields = vec![
        "name", "var_type", "area_type", "address", "bit", "var_group", 
        "description", "shared", "retentive", "dynamic_settings", 
        "struct_type", "initial_value"
    ];
    
    if !valid_fields.contains(&field.as_str()) {
        return Err(format!("Campo non valido: {}", field));
    }
    
    let update_sql = format!("UPDATE `{}` SET `{}` = ? WHERE id = ?", variables_table_name, field);
    conn.execute(&update_sql, params![value, variable_id]).map_err(|e| e.to_string())?;
    
    Ok("Variabile aggiornata con successo".to_string())
}

// Mappa dei tipi di variabili PremiumHMI
pub fn get_variable_type_name(type_code: &str) -> &'static str {
    match type_code {
        "0" => "Bool",
        "1" => "Byte",
        "2" => "SByte",
        "3" => "Int16",
        "4" => "UInt16",
        "5" => "Int32",
        "6" => "UInt32",
        "7" => "Int64",
        "8" => "Real (Float)",
        "9" => "String",
        "10" => "DateTime",
        "11" => "Struct",
        "12" => "Array",
        _ => "Unknown"
    }
}

// Mappa dei tipi di area
pub fn get_area_type_name(area_code: &str) -> &'static str {
    match area_code {
        "0" => "Input",
        "1" => "Output",
        "2" => "Flag",
        "3" => "Memory",
        _ => "Unknown"
    }
}
