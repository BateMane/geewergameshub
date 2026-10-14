#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
use tauri::{State, Manager}; // AJOUT: Manager est nécessaire pour accéder aux paths dans le setup
use winreg::enums::*;
use winreg::RegKey;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use regex::Regex;

// --- STRUCTURES DE DONNÉES ---

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct Game {
    id: String,
    title: String,
    platform: String,
    image_path: String,
    exe_path: String,
    install_dir: String,
    is_favorite: bool,
    #[serde(default)]
    hero_path: String,
    #[serde(default)]
    is_hidden: bool,
    #[serde(default)]
    last_played: u64,
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
    #[serde(default)]
    favorites: HashSet<String>,
    #[serde(default)]
    custom_games: Vec<Game>,
    #[serde(default)]
    selected_drives: HashSet<String>,
    #[serde(default)]
    hidden: HashSet<String>,
    #[serde(default)]
    last_played: HashMap<String, u64>,
    // Titre -> URL de jaquette trouvée en ligne (évite de requêter Steam à chaque lancement)
    #[serde(default)]
    image_cache: HashMap<String, String>,

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

// Visuels Steam officiels (chemins hachés pour les jeux récents)
#[derive(Serialize, Clone, Default, Debug)]
struct SteamArt {
    cover: String, // library_capsule_2x : 1200x1800
    wide: String,  // main_capsule : 616x353
    hero: String,  // library_hero : 3840x1240
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
            match serde_json::from_str::<UserData>(&content) {
                Ok(data) => return data,
                // Fichier corrompu : on le met de côté au lieu de l'écraser à la prochaine sauvegarde
                Err(_) => { let _ = fs::copy(path, path.with_extension("json.bak")); }
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

    // Écriture atomique : un crash pendant l'écriture ne détruit pas les données existantes
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, json).is_ok() {
            let _ = fs::rename(&tmp, path);
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn game_key(platform: &str, id: &str) -> String {
    format!("{}-{}", platform, id)
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

#[tauri::command(async)]
fn find_image_online(title: String, state: State<AppState>) -> String {
    if let Some(url) = state.data.lock().ok().and_then(|d| d.image_cache.get(&title).cloned()) {
        return url;
    }
    let url = search_steam_cover(&title);
    if !url.is_empty() {
        if let Ok(mut data) = state.data.lock() {
            data.image_cache.insert(title, url.clone());
            save_data(&state.data_path, &data);
        }
    }
    url
}

fn search_steam_cover(title: &str) -> String {
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

#[tauri::command(async)]
fn get_steam_details(steam_id: String, language: String) -> String {
    if steam_id.is_empty() || !steam_id.chars().all(|c| c.is_ascii_digit()) { return "{}".to_string(); }
    let url = format!("https://store.steampowered.com/api/appdetails?appids={}&l={}", steam_id, urlencoding::encode(&language));

    if let Ok(response) = ureq::get(&url).call() {
        if let Ok(json_str) = response.into_string() {
            return json_str;
        }
    }
    "{}".to_string()
}

// Les jeux récents ont des visuels à chemin haché : l'URL statique .../apps/<id>/library_600x900.jpg
// renvoie alors une image grise (HTTP 200). IStoreBrowseService (public, sans clé) donne les vrais chemins.
#[tauri::command(async)]
fn get_steam_art(app_ids: Vec<String>) -> HashMap<String, SteamArt> {
    let mut out = HashMap::new();
    let ids: Vec<u64> = app_ids.iter().filter_map(|id| id.parse().ok()).collect();
    for chunk in ids.chunks(50) {
        let input = serde_json::json!({
            "ids": chunk.iter().map(|id| serde_json::json!({ "appid": id })).collect::<Vec<_>>(),
            "context": { "language": "english", "country_code": "US" },
            "data_request": { "include_assets": true }
        });
        let resp = ureq::get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
            .timeout(std::time::Duration::from_secs(10))
            .query("input_json", &input.to_string())
            .call();
        if let Ok(r) = resp {
            if let Ok(json) = r.into_json::<Value>() {
                out.extend(parse_steam_art(&json));
            }
        }
    }
    out
}

fn parse_steam_art(json: &Value) -> HashMap<String, SteamArt> {
    let mut out = HashMap::new();
    for item in json["response"]["store_items"].as_array().into_iter().flatten() {
        let assets = &item["assets"];
        let (Some(appid), Some(format)) = (item["appid"].as_u64(), assets["asset_url_format"].as_str()) else { continue };
        let url = |keys: &[&str]| keys.iter()
            .find_map(|k| assets[*k].as_str())
            .map(|file| format!("https://shared.akamai.steamstatic.com/store_item_assets/{}", format.replace("${FILENAME}", file)))
            .unwrap_or_default();
        out.insert(appid.to_string(), SteamArt {
            cover: url(&["library_capsule_2x", "library_capsule"]),
            wide: url(&["main_capsule", "header"]),
            hero: url(&["library_hero"]),
        });
    }
    out
}

#[tauri::command(async)]
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

#[tauri::command(async)]
fn check_mod_support(title: String) -> ModInfo {
    let mut info = ModInfo::default();
    let clean_title_lower = title.to_lowercase();

    // 1. Thunderstore (liste des communautés téléchargée une seule fois par session)
    // Comparaison exacte sur le titre normalisé : l'ancien `contains` faisait matcher "Rust" avec "Rusty Lake"
    let wanted = normalize_title(&clean_title_lower);
    if let Some(communities) = thunderstore_communities() {
        if let Some((_, identifier)) = communities.iter().find(|(name, _)| !name.is_empty() && *name == wanted) {
            info.thunderstore = Some(format!("https://thunderstore.io/c/{}/", identifier));
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

fn normalize_title(t: &str) -> String {
    t.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

fn thunderstore_communities() -> Option<&'static Vec<(String, String)>> {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    if let Some(c) = CACHE.get() { return Some(c); }
    let json = ureq::get("https://thunderstore.io/api/experimental/community/").call().ok()?.into_json::<Value>().ok()?;
    let list = json["results"].as_array()?.iter().map(|c| (
        normalize_title(c["name"].as_str().unwrap_or("")),
        c["identifier"].as_str().unwrap_or("").to_string(),
    )).collect();
    // Seul un succès est mis en cache : une coupure réseau ne bloque pas la session
    let _ = CACHE.set(list);
    CACHE.get()
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

fn acf_value(content: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    content.lines()
        .find(|l| l.trim_start().to_lowercase().starts_with(&needle))
        .and_then(|l| l.split('"').nth(3))
        .map(|v| v.to_string())
}

// Redistribuables / runtimes que Steam installe mais qui ne sont pas des jeux
fn is_steam_tool(id: &str, title: &str) -> bool {
    id == "228980"
        || title.starts_with("Proton ")
        || title.starts_with("Steam Linux Runtime")
        || title.starts_with("Steamworks Common")
}

// Le client Steam (2024+) range les visuels dans librarycache/<appid>/[<hash>/]<fichier>
fn find_steam_art(library_cache: &Path, id: &str, names: &[&str]) -> String {
    let legacy = library_cache.join(format!("{}_{}", id, names[0]));
    if legacy.exists() { return legacy.to_string_lossy().to_string(); }

    let app_dir = library_cache.join(id);
    let mut dirs = vec![app_dir.clone()];
    if let Ok(entries) = fs::read_dir(&app_dir) {
        dirs.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    for name in names {
        for dir in &dirs {
            let p = dir.join(name);
            if p.exists() { return p.to_string_lossy().to_string(); }
        }
    }
    "".to_string()
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
                                if let Some(title) = acf_value(&content, "name") {
                                    if is_steam_tool(&id, &title) { continue; }

                                    let library_cache = steam_root.join("appcache").join("librarycache");
                                    let img = find_steam_art(&library_cache, &id, &["library_600x900.jpg", "library_capsule.jpg"]);
                                    let hero = find_steam_art(&library_cache, &id, &["library_hero.jpg"]);

                                    let install_dir = match acf_value(&content, "installdir") {
                                        Some(dir) => apps_path.join("common").join(dir),
                                        None => lib_path.clone(),
                                    }.to_string_lossy().to_string();

                                    games.push(Game {
                                        id, title, platform: "Steam".to_string(),
                                        image_path: img, exe_path: "".to_string(), install_dir,
                                        hero_path: hero, ..Default::default()
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
                            image_path: found_img, exe_path: full_exe, install_dir, is_favorite: false, ..Default::default()
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
                        // Moteurs et plugins (Unreal Engine, Quixel Bridge...) ne sont pas des jeux
                        let is_game = json["AppCategories"].as_array()
                            .map_or(true, |cats| cats.iter().any(|c| c.as_str() == Some("games")));
                        if is_game && !display_name.is_empty() && app_name != "HelloNeighborModKit" {
                            let found_img = scavenge_image(&install_loc);
                            games.push(Game {
                                id: app_name, title: display_name, platform: "Epic".to_string(),
                                image_path: found_img, exe_path: "".to_string(), install_dir: install_loc,
                                is_favorite: false, ..Default::default()
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
                        if title == "EA app" || title == "Origin" { continue; }
                        let install_loc: String = subkey.get_value("InstallLocation").unwrap_or_default();
                        if !title.is_empty() && !install_loc.is_empty() && Path::new(&install_loc).exists() {
                            let found_img = scavenge_image(&install_loc);
                            games.push(Game {
                                id: subkey_name.clone(), title, platform: "EA".to_string(),
                                image_path: found_img, exe_path: install_loc.clone(), install_dir: install_loc, is_favorite: false, ..Default::default()
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
                                    image_path: found_img, exe_path: "".to_string(), install_dir, is_favorite: false, ..Default::default()
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
    let title = title.trim().to_string();
    if title.is_empty() || exe_path.is_empty() { return Err("Titre et exécutable requis".into()); }
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    // Millisecondes : deux ajouts dans la même seconde ne partagent plus le même id
    let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let id = format!("Custom-{}", millis);

    let new_game = Game {
        id, title, platform: "Custom".to_string(), image_path, exe_path, ..Default::default()
    };

    data.custom_games.push(new_game);
    save_data(&state.data_path, &data);
    Ok(())
}

#[tauri::command]
fn remove_custom_game(game_id: String, state: State<AppState>) -> Result<(), String> {
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    data.custom_games.retain(|g| g.id != game_id);
    let key = game_key("Custom", &game_id);
    data.favorites.remove(&key);
    data.hidden.remove(&key);
    data.last_played.remove(&key);
    save_data(&state.data_path, &data);
    Ok(())
}

#[tauri::command]
fn toggle_hidden(game_id: String, platform: String, state: State<AppState>) -> Result<bool, String> {
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    let key = game_key(&platform, &game_id);
    let hidden = if data.hidden.remove(&key) { false } else { data.hidden.insert(key); true };
    save_data(&state.data_path, &data);
    Ok(hidden)
}

#[tauri::command]
fn toggle_favorite(game_id: String, platform: String, state: State<AppState>) -> Result<bool, String> {
    let mut data = state.data.lock().map_err(|_| "Lock error")?;
    let key = game_key(&platform, &game_id);

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

// ===================== DÉMARRAGE AUTOMATIQUE =====================
// Version MSIX (Store) : MSIX virtualise les écritures HKCU, la clé Run du plugin autostart
// est donc invisible pour Windows. On passe par la StartupTask déclarée dans le manifeste.
// Hors MSIX (MSI, dev), GetAsync échoue faute d'identité de package -> plugin autostart.
#[cfg(windows)]
fn msix_startup_task() -> Option<windows::ApplicationModel::StartupTask> {
    // Les commandes async tournent sur des threads sans COM initialisé
    unsafe { let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED); }
    windows::ApplicationModel::StartupTask::GetAsync(&windows::core::HSTRING::from("GeewerGameHubStartup")).ok()?.get().ok()
}

#[tauri::command(async)]
fn get_autostart(app: tauri::AppHandle) -> bool {
    #[cfg(windows)]
    if let Some(task) = msix_startup_task() {
        use windows::ApplicationModel::StartupTaskState;
        return matches!(task.State(), Ok(s) if s == StartupTaskState::Enabled || s == StartupTaskState::EnabledByPolicy);
    }
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command(async)]
fn set_autostart(enabled: bool, app: tauri::AppHandle) -> Result<bool, String> {
    #[cfg(windows)]
    if let Some(task) = msix_startup_task() {
        use windows::ApplicationModel::StartupTaskState;
        if enabled {
            let state = task.RequestEnableAsync().and_then(|op| op.get()).map_err(|e| e.to_string())?;
            return Ok(state == StartupTaskState::Enabled);
        }
        task.Disable().map_err(|e| e.to_string())?;
        return Ok(false);
    }
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if enabled { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    Ok(enabled)
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

#[tauri::command(async)]
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
        let unique_key = game_key(&game.platform, &game.id);
        game.is_favorite = data.favorites.contains(&unique_key);
        game.is_hidden = data.hidden.contains(&unique_key);
        game.last_played = data.last_played.get(&unique_key).copied().unwrap_or(0);
        if !seen_ids.contains(&unique_key) {
            seen_ids.insert(unique_key);
            unique_games.push(game);
        }
    }

    unique_games.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    unique_games
}

#[tauri::command]
fn launch_game(id: String, platform: String, exe_path: String, state: State<AppState>) -> u64 {
    let ts = now_secs();
    if let Ok(mut data) = state.data.lock() {
        data.last_played.insert(game_key(&platform, &id), ts);
        save_data(&state.data_path, &data);
    }
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
                            return ts;
                        }
                    }
                }
            }
            opener::open(exe_path).ok();
        },
        "Custom" => { opener::open(exe_path).ok(); },
        _ => {}
    }
    ts
}

#[tauri::command]
fn open_launcher_page(id: String, platform: String) {
    let url = match platform.as_str() {
        "Steam" => format!("steam://nav/games/details/{}", id),
        "GOG" => format!("goggalaxy://openGameView/{}", id),
        "Epic" => "com.epicgames.launcher://library".to_string(),
        "Ubisoft" => "uplay://".to_string(),
        "EA" => "origin2://library".to_string(),
        // Ouverture libre depuis l'interface : limitée aux liens web/mail et aux dossiers existants
        "Custom" => {
            let lower = id.to_lowercase();
            let is_link = lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:");
            if !is_link && !Path::new(&id).is_dir() { return; }
            id
        },
        _ => return,
    };
    opener::open(url).ok();
}

fn main() {
    tauri::Builder::default()
        // Plugins initiaux
        // Une seule instance : relancer l'app (raccourci, démarrage auto) remet la fenêtre existante au premier plan
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
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
            get_steam_art,
            toggle_favorite,
            add_custom_game,
            remove_custom_game,
            toggle_hidden,
            get_autostart,
            set_autostart,
            get_hltb,
            check_mod_support,
            get_system_drives,
            update_settings,
            get_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acf_value_lit_name_et_installdir() {
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"1478500\"\n\t\"name\"\t\t\"Big Walk\"\n\t\"installdir\"\t\t\"Big Walk Dir\"\n}";
        assert_eq!(acf_value(acf, "name").as_deref(), Some("Big Walk"));
        assert_eq!(acf_value(acf, "installdir").as_deref(), Some("Big Walk Dir"));
        assert_eq!(acf_value(acf, "absent"), None);
    }

    #[test]
    fn outils_steam_filtres() {
        assert!(is_steam_tool("228980", "Steamworks Common Redistributables"));
        assert!(is_steam_tool("1493710", "Proton Experimental"));
        assert!(!is_steam_tool("1478500", "Big Walk"));
    }

    #[test]
    fn visuels_steam_haches_et_classiques() {
        let json: Value = serde_json::from_str(r#"{"response":{"store_items":[
            {"appid":2807960,"assets":{"asset_url_format":"steam/apps/2807960/${FILENAME}?t=1",
              "library_capsule":"aa/library_capsule.jpg","library_capsule_2x":"aa/library_capsule_2x.jpg",
              "main_capsule":"bb/capsule_616x353.jpg","library_hero":"cc/library_hero.jpg"}},
            {"appid":219150,"assets":{"asset_url_format":"steam/apps/219150/${FILENAME}?t=2",
              "library_capsule":"library_600x900.jpg","header":"header.jpg"}},
            {"appid":1,"assets":{}}
        ]}}"#).unwrap();
        let art = parse_steam_art(&json);
        assert_eq!(art["2807960"].cover, "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/2807960/aa/library_capsule_2x.jpg?t=1");
        assert!(art["2807960"].hero.ends_with("cc/library_hero.jpg?t=1"));
        assert!(art["219150"].cover.ends_with("219150/library_600x900.jpg?t=2"));
        assert!(art["219150"].wide.ends_with("header.jpg?t=2"));
        assert_eq!(art["219150"].hero, "");
        assert!(!art.contains_key("1"));
    }

    #[test]
    fn normalisation_titres() {
        assert_eq!(normalize_title("Risk of Rain 2"), "riskofrain2");
        assert_ne!(normalize_title("Rust"), normalize_title("Rusty Lake"));
    }

    #[test]
    fn jaquette_steam_nouveau_format() {
        let cache = std::env::temp_dir().join(format!("ggh_test_{}", std::process::id()));
        let hashed = cache.join("42").join("abc123");
        fs::create_dir_all(&hashed).unwrap();
        fs::write(hashed.join("library_capsule.jpg"), b"x").unwrap();
        fs::write(cache.join("42").join("library_hero.jpg"), b"x").unwrap();

        assert!(find_steam_art(&cache, "42", &["library_600x900.jpg", "library_capsule.jpg"]).ends_with("library_capsule.jpg"));
        assert!(find_steam_art(&cache, "42", &["library_hero.jpg"]).ends_with("library_hero.jpg"));
        assert_eq!(find_steam_art(&cache, "99", &["library_hero.jpg"]), "");
        let _ = fs::remove_dir_all(&cache);
    }

    #[test]
    fn sauvegarde_atomique_et_fichier_corrompu() {
        let dir = std::env::temp_dir().join(format!("ggh_save_{}", std::process::id()));
        let path = dir.join("geewers_data.json");
        let mut data = UserData::default();
        data.favorites.insert("Steam-1".into());
        save_data(&path, &data);
        assert!(load_data(&path).favorites.contains("Steam-1"));

        fs::write(&path, "{ pas du json").unwrap();
        assert!(load_data(&path).favorites.is_empty());
        assert!(path.with_extension("json.bak").exists(), "le fichier corrompu doit être sauvegardé");
        let _ = fs::remove_dir_all(&dir);
    }
}
