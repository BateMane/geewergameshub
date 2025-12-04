#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{State, Manager}; // AJOUT: Manager est nécessaire pour accéder aux paths dans le setup
use winreg::enums::*;
use winreg::RegKey;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use regex::Regex;

// --- STRUCTURES DE DONNÉES ---

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Game {
    id: String,
    title: String,
    platform: String,
    image_path: String,
    exe_path: String,
    install_dir: String,
    is_favorite: bool,
}

// Configuration complète du thème
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ThemeConfig {
    accent: String,       // Couleur principale (boutons, favoris)
    bg_from: String,      // Dégradé fond haut
    bg_to: String,        // Dégradé fond bas
    card_bg: String,      // Couleur de fond des cartes
    text_primary: String, // Couleur du texte principal
}

// Valeurs par défaut (Thème sombre standard)
impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: "#5865F2".to_string(),     // Discord Blurple
            bg_from: "#121212".to_string(),    // Dark Grey
            bg_to: "#0a0a0a".to_string(),      // Black
            card_bg: "#1e1e1e".to_string(),    // Card Grey
            text_primary: "#f3f4f6".to_string(), // White/Gray
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct UserData {
    favorites: HashSet<String>,
    custom_games: Vec<Game>,
    selected_drives: HashSet<String>,
    
    // On utilise #[serde(default)] pour que les anciens fichiers de config 
    // (qui n'ont pas encore 'theme') chargent les valeurs par défaut sans planter.
    #[serde(default)] 
    theme: ThemeConfig, 
}

#[derive(Serialize, Clone, Default)]
struct HltbTime {
    main: i32,
    main_extra: i32,
    completionist: i32,
}

#[derive(Serialize, Clone, Default)]
struct ModInfo {
    nexus: Option<String>,
    thunderstore: Option<String>,
}

struct AppState {
    data_path: PathBuf,
    data: Mutex<UserData>,
}

// ===================== 1. GESTION SAUVEGARDE =====================

fn load_data(path: &Path) -> UserData {
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            // On tente de charger. Si le champ 'theme' manque, serde utilise Default::default() grâce à l'attribut.
            if let Ok(data) = serde_json::from_str::<UserData>(&content) {
                return data;
            }
        }
    }
    UserData::default()
}

fn save_data(path: &Path, data: &UserData) {
    // MODIF IMPORTANTE : On crée le dossier parent s'il n'existe pas
    // C'est crucial pour le premier lancement dans AppData
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, json);
    }
}

// ===================== 2. OUTILS WEB & FICHIERS =====================

fn scavenge_image(install_dir: &str) -> String {
    let path = Path::new(install_dir);
    if install_dir.is_empty() || !path.exists() { return "".to_string(); }

    let candidates = ["cover", "banner", "poster", "splash", "header", "logo", "background", "boxart"];
    let extensions = ["jpg", "png", "jpeg", "webp"];

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(name) = p.file_stem().and_then(|n| n.to_str()) {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        let name_lower = name.to_lowercase();
                        let ext_lower = ext.to_lowercase();
                        if candidates.iter().any(|&c| name_lower.contains(c)) && extensions.contains(&ext_lower.as_str()) {
                            return p.to_string_lossy().to_string();
                        }
                    }
                }
            }
        }
    }
    "".to_string()
}

#[tauri::command]
fn find_image_online(title: String) -> String {
    let clean_title = title.replace("™", "").replace("®", "").replace("-", " ");
    let encoded_title = urlencoding::encode(&clean_title);
    let url = format!("https://store.steampowered.com/api/storesearch/?term={}&l=english&cc=US", encoded_title);
    
    if let Ok(response) = ureq::get(&url).call() {
        if let Ok(json) = response.into_json::<Value>() {
             if let Some(items) = json["items"].as_array() {
                 if let Some(first_match) = items.first() {
                     if let Some(steam_id) = first_match["id"].as_i64() {
                         return format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{}/library_600x900.jpg", steam_id);
                     }
                 }
             }
        }
    }
    "".to_string()
}

#[tauri::command]
fn get_steam_details(steam_id: String, language: String) -> String {
    // On utilise le paramètre 'language' reçu du frontend
    let url = format!("https://store.steampowered.com/api/appdetails?appids={}&l={}", steam_id, language);
    
    if let Ok(response) = ureq::get(&url).call() {
        if let Ok(json_str) = response.into_string() {
            return json_str; 
        }
    }
    "{}".to_string()
}

#[tauri::command]
fn get_hltb(title: String) -> HltbTime {
    let clean_title = title
        .to_lowercase()
        .replace("goty", "")
        .replace("edition", "")
        .replace(":", "")
        .replace("-", " ")
        .replace("™", "")
        .replace("®", "")
        .trim()
        .to_string();

    let url = "https://html.duckduckgo.com/html/";
    let query = format!("site:howlongtobeat.com {}", clean_title);
    
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let response = agent.post(url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&format!("q={}", urlencoding::encode(&query)));

    match response {
        Ok(resp) => {
            if let Ok(html) = resp.into_string() {
                let mut time = HltbTime::default();
                let mut found = false;

                if let Ok(re_main) = Regex::new(r"(?i)Main Story\s*[-:]?\s*(\d+)") {
                    if let Some(caps) = re_main.captures(&html) {
                        if let Some(m) = caps.get(1) {
                            time.main = m.as_str().parse().unwrap_or(0);
                            found = true;
                        }
                    }
                }
                if let Ok(re_extra) = Regex::new(r"(?i)Main \+ Extras\s*[-:]?\s*(\d+)") {
                    if let Some(caps) = re_extra.captures(&html) {
                        if let Some(m) = caps.get(1) {
                            time.main_extra = m.as_str().parse().unwrap_or(0);
                            found = true;
                        }
                    }
                }
                if let Ok(re_comp) = Regex::new(r"(?i)Completionist\s*[-:]?\s*(\d+)") {
                    if let Some(caps) = re_comp.captures(&html) {
                        if let Some(m) = caps.get(1) {
                            time.completionist = m.as_str().parse().unwrap_or(0);
                            found = true;
                        }
                    }
                }

                if found {
                    return time;
                }
            }
        },
        Err(_) => {}
    }
    
    HltbTime::default()
}

#[tauri::command]
fn check_mod_support(title: String) -> ModInfo {
    let mut info = ModInfo::default();
    let clean_title_lower = title.to_lowercase();

    // 1. Thunderstore
    let ts_url = "https://thunderstore.io/api/experimental/community/";
    if let Ok(response) = ureq::get(ts_url).call() {
        if let Ok(json) = response.into_json::<Value>() {
            if let Some(results) = json["results"].as_array() {
                for community in results {
                    let name = community["name"].as_str().unwrap_or("").to_lowercase();
                    let identifier = community["identifier"].as_str().unwrap_or("");
                    if name == clean_title_lower || clean_title_lower.contains(&name) {
                        info.thunderstore = Some(format!("https://thunderstore.io/c/{}/", identifier));
                        break;
                    }
                }
            }
        }
    }

    // 2. Nexus Mods (FIXED & ROBUST)
    let slug = title.to_lowercase()
        .replace(":", "").replace("'", "").replace("-", "")
        .replace(" ", "").replace("™", "").replace("®", "").replace(".", "");
        
    let nexus_url = format!("https://www.nexusmods.com/{}", slug);
    
    // On se fait passer pour un vrai navigateur Chrome Windows pour éviter les blocages
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .redirects(10)
        .build();

    let resp = agent.get(&nexus_url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .call();

    match resp {
        Ok(r) => {
            // Si le site répond 200 OK, c'est bon
            if r.status() == 200 {
                info.nexus = Some(nexus_url);
            }
        },
        Err(ureq::Error::Status(404, _)) => {
            // Si c'est une vraie 404, le jeu n'existe pas sur Nexus -> Pas de bouton
            info.nexus = None;
        },
        Err(_) => {
            // Pour TOUTE autre erreur (Timeout, Réseau, 429 Too Many Requests...)
            // On affiche le bouton de manière OPTIMISTE.
            // Cela règle le problème "une fois sur deux" : si ça lag, le bouton sera là quand même.
            info.nexus = Some(nexus_url);
        }
    }

    info
}

// ===================== 3. SCANNERS (PROVIDERS) =====================

fn get_steam_library_folders(steam_root: &Path) -> Vec<PathBuf> {
    let mut folder_set = HashSet::new();
    let mut folders = Vec::new();

    let root_str = steam_root.to_string_lossy().to_string().to_lowercase();
    folder_set.insert(root_str);
    folders.push(steam_root.to_path_buf());

    let vdf_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    if vdf_path.exists() {
        if let Ok(content) = fs::read_to_string(vdf_path) {
            if let Ok(re) = Regex::new(r#""path"\s+"((?:[^"\\]|\\.)+)""#) {
                for cap in re.captures_iter(&content) {
                    if let Some(path_match) = cap.get(1) {
                        let raw_path = path_match.as_str().replace("\\\\", "\\");
                        let p = PathBuf::from(&raw_path);
                        let p_lower = raw_path.to_lowercase();
                        if !folder_set.contains(&p_lower) && p.exists() {
                            folder_set.insert(p_lower);
                            folders.push(p);
                        }
                    }
                }
            }
        }
    }
    folders
}

fn get_steam_games() -> Vec<Game> {
    let mut games = Vec::new();
    
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(steam_key) = hkcu.open_subkey("Software\\Valve\\Steam") {
        if let Ok(steam_path_str) = steam_key.get_value::<String, _>("SteamPath") {
            let steam_root = Path::new(&steam_path_str);
            let library_folders = get_steam_library_folders(steam_root);

            for lib_path in library_folders {
                let apps_path = lib_path.join("steamapps");
                if let Ok(entries) = fs::read_dir(&apps_path) {
                    for entry in entries.flatten() {
                        let filename = entry.file_name();
                        let filename_str = filename.to_string_lossy();

                        if filename_str.starts_with("appmanifest_") && filename_str.ends_with(".acf") {
                            if let Ok(content) = fs::read_to_string(entry.path()) {
                                let id = filename_str.replace("appmanifest_", "").replace(".acf", "");
                                if let Some(name_line) = content.lines().find(|l| l.contains("\"name\"")) {
                                    let title = name_line.split("\"").nth(3).unwrap_or("Jeu Steam").to_string();
                                    
                                    let library_cache = steam_root.join("appcache").join("librarycache");
                                    let cover = library_cache.join(format!("{}_library_600x900.jpg", id));
                                    let img = if cover.exists() { cover.to_string_lossy().to_string() } else { "".to_string() };

                                    let install_dir = lib_path.to_string_lossy().to_string();

                                    games.push(Game { 
                                        id, title, platform: "Steam".to_string(), 
                                        image_path: img, exe_path: "".to_string(), install_dir,
                                        is_favorite: false
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    games
}

fn get_gog_games() -> Vec<Game> {
    let mut games = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let keys = ["SOFTWARE\\GOG.com\\Games", "SOFTWARE\\WOW6432Node\\GOG.com\\Games"];

    for key_path in keys {
        if let Ok(gog_key) = hklm.open_subkey(key_path) {
            for id in gog_key.enum_keys().map(|x| x.unwrap_or_default()) {
                if id.is_empty() { continue; }
                if let Ok(game_key) = gog_key.open_subkey(&id) {
                    let title: String = game_key.get_value("gameName").unwrap_or_default();
                    let install_dir: String = game_key.get_value("path").unwrap_or_default();
                    let exe: String = game_key.get_value("exe").unwrap_or_default();
                    
                    if !title.is_empty() && !install_dir.is_empty() && Path::new(&install_dir).exists() {
                        let found_img = scavenge_image(&install_dir);
                        let full_exe = Path::new(&install_dir).join(exe).to_string_lossy().to_string();
                        games.push(Game { 
                            id: id.clone(), title, platform: "GOG".to_string(), 
                            image_path: found_img, exe_path: full_exe, install_dir, is_favorite: false 
                        });
                    }
                }
            }
        }
    }
    games
}

fn get_epic_games() -> Vec<Game> {
    let mut games = Vec::new();
    let manifest_path = Path::new("C:\\ProgramData\\Epic\\EpicGamesLauncher\\Data\\Manifests");
    if manifest_path.exists() {
        if let Ok(entries) = fs::read_dir(manifest_path) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let app_name = json["AppName"].as_str().unwrap_or("").to_string();
                        let display_name = json["DisplayName"].as_str().unwrap_or("").to_string();
                        let install_loc = json["InstallLocation"].as_str().unwrap_or("").to_string();
                        if !display_name.is_empty() && app_name != "HelloNeighborModKit" {
                            let found_img = scavenge_image(&install_loc);
                            games.push(Game { 
                                id: app_name, title: display_name, platform: "Epic".to_string(), 
                                image_path: found_img, exe_path: "".to_string(), install_dir: install_loc,
                                is_favorite: false 
                            });
                        }
                    }
                }
            }
        }
    }
    games
}

fn get_ea_and_ubi_games() -> Vec<Game> {
    let mut games = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let uninstall_paths = [
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher\\Installs" 
    ];

    for path in uninstall_paths {
        if let Ok(key) = hklm.open_subkey(path) {
            for subkey_name in key.enum_keys().map(|x| x.unwrap_or_default()) {
                if subkey_name.is_empty() { continue; }
                if let Ok(subkey) = key.open_subkey(&subkey_name) {
                    
                    let publisher: String = subkey.get_value("Publisher").unwrap_or_default();
                    if publisher.contains("Electronic Arts") || publisher == "EA" {
                        let title: String = subkey.get_value("DisplayName").unwrap_or_default();
                        let install_loc: String = subkey.get_value("InstallLocation").unwrap_or_default();
                        if !title.is_empty() && !install_loc.is_empty() && Path::new(&install_loc).exists() {
                            let found_img = scavenge_image(&install_loc);
                            games.push(Game { 
                                id: subkey_name.clone(), title, platform: "EA".to_string(), 
                                image_path: found_img, exe_path: install_loc.clone(), install_dir: install_loc, is_favorite: false 
                            });
                        }
                    }

                    if path.contains("Ubisoft") {
                         if let Ok(install_dir) = subkey.get_value::<String, _>("InstallDir") {
                            if Path::new(&install_dir).exists() {
                                let p = Path::new(&install_dir);
                                let title = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                                let found_img = scavenge_image(&install_dir);
                                games.push(Game { 
                                    id: subkey_name.clone(), title, platform: "Ubisoft".to_string(), 
                                    image_path: found_img, exe_path: "".to_string(), install_dir, is_favorite: false 
                                });
                            }
                         }
                    }
                }
            }
        }
    }
    games
}

// ===================== 4. COMMANDES TAURI =====================

#[tauri::command]
fn add_custom_game(title: String, exe_path: String, image_path: String, state: State<AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    let id = format!("Custom-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    
    let new_game = Game {
        id, title, platform: "Custom".to_string(), image_path, exe_path, install_dir: "".to_string(), is_favorite: false,
    };

    data.custom_games.push(new_game);
    save_data(&state.data_path, &data);
    Ok(())
}

#[tauri::command]
fn toggle_favorite(game_id: String, platform: String, state: State<AppState>) -> Result<bool, String> {
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    let key = format!("{}-{}", platform, game_id);
    
    let is_fav = if data.favorites.contains(&key) {
        data.favorites.remove(&key);
        false
    } else {
        data.favorites.insert(key);
        true
    };
    save_data(&state.data_path, &data);
    Ok(is_fav)
}

#[tauri::command]
fn get_system_drives() -> Vec<String> {
    let mut drives = Vec::new();
    if cfg!(windows) {
        for drive in ('A'..='Z').map(|c| format!("{}:\\", c)) {
            let path = PathBuf::from(&drive);
            if path.exists() {
                drives.push(drive);
            }
        }
    } else {
        drives.push("/".to_string()); 
    }
    drives
}

// Sauvegarder les paramètres complets (Theme + Disques)
#[tauri::command]
fn update_settings(theme: ThemeConfig, selected_drives: Vec<String>, state: State<AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    data.theme = theme;
    data.selected_drives = selected_drives.into_iter().collect();
    save_data(&state.data_path, &data);
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<UserData, String> {
    let data = state.data.lock().map_err(|_| "Lock error")?;
    Ok(data.clone())
}

#[tauri::command]
fn get_games(state: State<AppState>) -> Vec<Game> {
    let mut all_scanned_games = Vec::new();
    
    // 1. On récupère tout
    all_scanned_games.extend(get_steam_games());
    all_scanned_games.extend(get_epic_games());
    all_scanned_games.extend(get_gog_games());
    all_scanned_games.extend(get_ea_and_ubi_games());

    let data = state.data.lock().unwrap();
    
    // 2. On filtre selon les disques sélectionnés
    let mut filtered_games = Vec::new();
    let use_filter = !data.selected_drives.is_empty();
    
    for game in all_scanned_games {
        if use_filter {
            // FIX: On remplace tous les / par des \ pour normaliser la comparaison
            // Cela règle le problème des jeux Ubisoft qui utilisent souvent des "/"
            let game_drive_norm = game.install_dir.to_lowercase().replace("/", "\\");
            
            let match_found = data.selected_drives.iter().any(|d| {
                let selected_drive_norm = d.to_lowercase().replace("/", "\\");
                game_drive_norm.starts_with(&selected_drive_norm)
            });
            
            if !match_found && !game.install_dir.is_empty() {
                // Si le jeu n'est pas sur le disque sélectionné, on l'ignore
                continue; 
            }
        }
        filtered_games.push(game);
    }

    // 3. On ajoute les jeux Custom (qui ne sont pas filtrés par disque pour éviter de les perdre)
    filtered_games.extend(data.custom_games.clone());

    // 4. Gestion des favoris et dédoublonnage
    let mut unique_games = Vec::new();
    let mut seen_ids = HashSet::new();

    for mut game in filtered_games {
        let unique_key = format!("{}-{}", game.platform, game.id);
        if data.favorites.contains(&unique_key) {
            game.is_favorite = true;
        }
        if !seen_ids.contains(&unique_key) {
            seen_ids.insert(unique_key);
            unique_games.push(game);
        }
    }

    unique_games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    unique_games
}

#[tauri::command]
fn launch_game(id: String, platform: String, exe_path: String) {
    match platform.as_str() {
        "Steam" => { opener::open(format!("steam://run/{}", id)).ok(); },
        "Epic" => { opener::open(format!("com.epicgames.launcher://apps/{}?action=launch&silent=true", id)).ok(); },
        "Ubisoft" => { opener::open(format!("uplay://launch/{}/0", id)).ok(); },
        "GOG" => { opener::open(format!("goggalaxy://openGameView/{}", id)).ok(); },
        "EA" => {
             if let Ok(entries) = fs::read_dir(Path::new(&exe_path)) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "exe" && !path.to_string_lossy().contains("Cleanup") && !path.to_string_lossy().contains("Touchup") {
                            opener::open(path.to_string_lossy().to_string()).ok();
                            return;
                        }
                    }
                }
            }
            opener::open(exe_path).ok();
        },
        "Custom" => { opener::open(exe_path).ok(); },
        _ => {}
    }
}

#[tauri::command]
fn open_launcher_page(id: String, platform: String) {
    let url = match platform.as_str() {
        "Steam" => format!("steam://nav/games/details/{}", id),
        "GOG" => format!("goggalaxy://openGameView/{}", id),
        "Epic" => "com.epicgames.launcher://library".to_string(),
        "Ubisoft" => "uplay://".to_string(),
        "EA" => "origin2://library".to_string(),
        "Custom" => id, 
        _ => return,
    };
    opener::open(url).ok();
}

fn main() {
    tauri::Builder::default()
        // Plugins initiaux
        .plugin(tauri_plugin_fs::init()) // Nécessaire pour la gestion de fichiers
        .plugin(tauri_plugin_dialog::init())
        // Initialisation de l'Autostart
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        // Setup : C'est ici qu'on gère le state et le path AppData
        .setup(|app| {
            // On récupère le chemin standard "Local AppData"
            // ex: C:\Users\Nom\AppData\Local\com.geewer.gamehub\
            let app_data_dir = app.path().app_local_data_dir().expect("Impossible de trouver le dossier AppData");
            let data_path = app_data_dir.join("geewers_data.json");

            // Chargement des données existantes
            let initial_data = load_data(&data_path);

            // Injection du state dans l'app
            app.manage(AppState {
                data_path,
                data: Mutex::new(initial_data),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_games, 
            launch_game, 
            open_launcher_page, 
            find_image_online, 
            get_steam_details,
            toggle_favorite,
            add_custom_game,
            get_hltb,
            check_mod_support,
            get_system_drives,
            update_settings,
            get_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}