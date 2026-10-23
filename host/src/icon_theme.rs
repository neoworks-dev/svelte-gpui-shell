//! Finding the file for a freedesktop icon name at a size, as the Icon Theme Specification
//! describes: the current theme, then the themes it inherits, then `hicolor`, then the
//! unthemed icons in `/usr/share/pixmaps`. The answers are kept, so asking again is a lookup.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Svg,
    Raster,
}

/// An icon found on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct IconFile {
    pub path: PathBuf,
    pub format: Format,
    /// A symbolic icon is a mask: drawn in one colour, which the text colour tints.
    pub symbolic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Fixed,
    Scalable,
    Threshold,
}

/// One size directory of a theme, from its `index.theme`.
#[derive(Debug, Clone)]
struct Directory {
    path: String,
    size: i32,
    scale: i32,
    kind: Kind,
    min: i32,
    max: i32,
    threshold: i32,
}

impl Directory {
    /// Whether the icons here are made for this size.
    fn matches(&self, size: i32, scale: i32) -> bool {
        if self.scale != scale {
            return false;
        }
        match self.kind {
            Kind::Fixed => self.size == size,
            Kind::Scalable => self.min <= size && size <= self.max,
            Kind::Threshold => self.size - self.threshold <= size && size <= self.size + self.threshold,
        }
    }

    /// How far the icons here are from this size, in device pixels.
    fn distance(&self, size: i32, scale: i32) -> i32 {
        let wanted = size * scale;
        let (low, high) = match self.kind {
            Kind::Fixed => (self.size * self.scale, self.size * self.scale),
            Kind::Scalable => (self.min * self.scale, self.max * self.scale),
            Kind::Threshold => ((self.size - self.threshold) * self.scale, (self.size + self.threshold) * self.scale),
        };
        if wanted < low {
            low - wanted
        } else if wanted > high {
            wanted - high
        } else {
            0
        }
    }
}

struct Theme {
    directories: Vec<Directory>,
    inherits: Vec<String>,
    /// Every `<base>/<name>` that exists: a theme can be spread over the user's and the system's.
    roots: Vec<PathBuf>,
}

/// What `find` remembers: the answer for a name at a size in a theme.
type Question = (String, i32, i32, String);

pub struct IconThemes {
    bases: Vec<PathBuf>,
    pixmaps: Vec<PathBuf>,
    default_theme: String,
    themes: HashMap<String, Option<Arc<Theme>>>,
    answers: HashMap<Question, Option<IconFile>>,
}

const EXTENSIONS: [(&str, Format); 2] = [("png", Format::Raster), ("svg", Format::Svg)];

impl IconThemes {
    pub fn new(bases: Vec<PathBuf>, pixmaps: Vec<PathBuf>, default_theme: String) -> Self {
        IconThemes { bases, pixmaps, default_theme, themes: HashMap::new(), answers: HashMap::new() }
    }

    /// The directories and the theme this session uses.
    pub fn from_environment() -> Self {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        let data_home = match std::env::var_os("XDG_DATA_HOME") {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => home.join(".local/share"),
        };
        let data_dirs = match std::env::var("XDG_DATA_DIRS") {
            Ok(dirs) if !dirs.is_empty() => dirs,
            _ => "/usr/local/share:/usr/share".to_string(),
        };
        let mut bases = vec![home.join(".icons"), data_home.join("icons")];
        let mut pixmaps = vec![data_home.join("pixmaps")];
        for dir in data_dirs.split(':').filter(|dir| !dir.is_empty()) {
            bases.push(Path::new(dir).join("icons"));
            pixmaps.push(Path::new(dir).join("pixmaps"));
        }
        IconThemes::new(bases, pixmaps, session_theme())
    }

    /// The file for `name` at `size` logical pixels on a display of `scale`, in `theme` or the
    /// session's. An absolute `name` is the file itself.
    pub fn find(&mut self, name: &str, size: u32, scale: u32, theme: Option<&str>) -> Option<IconFile> {
        if name.starts_with('/') {
            return file_at(Path::new(name));
        }
        let name = strip_extension(name);
        let theme = theme.filter(|theme| !theme.is_empty()).unwrap_or(&self.default_theme).to_string();
        let question = (name.to_string(), size as i32, scale.max(1) as i32, theme);
        if let Some(known) = self.answers.get(&question) {
            return known.clone();
        }
        let answer = self.look_up(&question.0, question.1, question.2, &question.3);
        self.answers.insert(question, answer.clone());
        answer
    }

    /// Forgets themes and answers, so the next lookup reads the disk again.
    pub fn forget(&mut self) {
        self.themes.clear();
        self.answers.clear();
    }

    fn look_up(&mut self, name: &str, size: i32, scale: i32, theme: &str) -> Option<IconFile> {
        let mut visited = HashSet::new();
        let found = self.in_theme_and_parents(name, size, scale, theme, &mut visited);
        found
            .or_else(|| self.in_theme_and_parents(name, size, scale, "hicolor", &mut visited))
            .or_else(|| self.unthemed(name))
    }

    fn in_theme_and_parents(
        &mut self,
        name: &str,
        size: i32,
        scale: i32,
        theme: &str,
        visited: &mut HashSet<String>,
    ) -> Option<IconFile> {
        if !visited.insert(theme.to_string()) {
            return None;
        }
        let loaded = self.theme(theme)?;
        if let Some(found) = in_theme(&loaded, name, size, scale) {
            return Some(found);
        }
        for parent in &loaded.inherits {
            if let Some(found) = self.in_theme_and_parents(name, size, scale, parent, visited) {
                return Some(found);
            }
        }
        None
    }

    fn unthemed(&self, name: &str) -> Option<IconFile> {
        for directory in &self.pixmaps {
            for (extension, format) in EXTENSIONS {
                let path = directory.join(format!("{name}.{extension}"));
                if path.is_file() {
                    return Some(IconFile { path, format, symbolic: name.ends_with("-symbolic") });
                }
            }
        }
        None
    }

    fn theme(&mut self, name: &str) -> Option<Arc<Theme>> {
        if let Some(known) = self.themes.get(name) {
            return known.clone();
        }
        let loaded = load_theme(&self.bases, name).map(Arc::new);
        self.themes.insert(name.to_string(), loaded.clone());
        loaded
    }
}

/// The best file in one theme: one from a directory made for the size, else the nearest.
fn in_theme(theme: &Theme, name: &str, size: i32, scale: i32) -> Option<IconFile> {
    let symbolic = name.ends_with("-symbolic");
    let mut nearest: Option<(i32, bool, i32, IconFile)> = None;
    for directory in &theme.directories {
        let Some(file) = in_directory(theme, directory, name, symbolic) else {
            continue;
        };
        let distance = directory.distance(size, scale);
        if distance == 0 && directory.matches(size, scale) {
            return Some(file);
        }
        // Among equals a vector icon is sharp at any size, and a larger raster shrinks better than a smaller grows.
        let rank = (distance, file.format != Format::Svg, -directory.size);
        let better = match &nearest {
            Some((best, raster, larger, _)) => rank < (*best, *raster, *larger),
            None => true,
        };
        if better {
            nearest = Some((rank.0, rank.1, rank.2, file));
        }
    }
    nearest.map(|(_, _, _, file)| file)
}

fn in_directory(theme: &Theme, directory: &Directory, name: &str, symbolic: bool) -> Option<IconFile> {
    for root in &theme.roots {
        for (extension, format) in EXTENSIONS {
            let path = root.join(&directory.path).join(format!("{name}.{extension}"));
            if path.is_file() {
                return Some(IconFile { path, format, symbolic });
            }
        }
    }
    None
}

/// A file at a path the app gave, such as a `.desktop` file's `Icon=/opt/app/icon.png`.
fn file_at(path: &Path) -> Option<IconFile> {
    if !path.is_file() {
        return None;
    }
    let format = match path.extension().and_then(|extension| extension.to_str()) {
        Some("svg") => Format::Svg,
        Some("png" | "jpg" | "jpeg" | "webp" | "gif") => Format::Raster,
        _ => return None,
    };
    let symbolic = path.file_stem().and_then(|stem| stem.to_str()).is_some_and(|stem| stem.ends_with("-symbolic"));
    Some(IconFile { path: path.to_path_buf(), format, symbolic })
}

/// `firefox.png` asks for `firefox`: some `.desktop` files name the file, not the icon.
fn strip_extension(name: &str) -> &str {
    for extension in [".png", ".svg", ".xpm"] {
        if let Some(stem) = name.strip_suffix(extension) {
            return stem;
        }
    }
    name
}

fn load_theme(bases: &[PathBuf], name: &str) -> Option<Theme> {
    let roots: Vec<PathBuf> = bases.iter().map(|base| base.join(name)).filter(|root| root.is_dir()).collect();
    let index = roots.iter().find_map(|root| std::fs::read_to_string(root.join("index.theme")).ok())?;
    let mut theme = parse_index(&index);
    theme.roots = roots;
    Some(theme)
}

/// An `index.theme`: the sections for the theme and for each size directory.
fn parse_index(text: &str) -> Theme {
    let mut sections: Vec<(String, HashMap<String, String>)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(title) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            sections.push((title.to_string(), HashMap::new()));
        } else if let (Some((_, fields)), Some((key, value))) = (sections.last_mut(), line.split_once('=')) {
            fields.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    let list = |fields: Option<&HashMap<String, String>>, key: &str| -> Vec<String> {
        let value = fields.and_then(|fields| fields.get(key)).map(String::as_str).unwrap_or("");
        value.split(',').map(str::trim).filter(|part| !part.is_empty()).map(String::from).collect()
    };
    let header = sections.iter().find(|(title, _)| title == "Icon Theme").map(|(_, fields)| fields);
    let mut order = list(header, "Directories");
    order.extend(list(header, "ScaledDirectories"));
    let directories = order
        .iter()
        .filter_map(|path| {
            let (_, fields) = sections.iter().find(|(title, _)| title == path)?;
            directory(path, fields)
        })
        .collect();
    Theme { directories, inherits: list(header, "Inherits"), roots: Vec::new() }
}

fn directory(path: &str, fields: &HashMap<String, String>) -> Option<Directory> {
    let number = |key: &str| fields.get(key).and_then(|value| value.parse::<i32>().ok());
    let size = number("Size")?;
    let kind = match fields.get("Type").map(String::as_str) {
        Some("Fixed") => Kind::Fixed,
        Some("Scalable") => Kind::Scalable,
        _ => Kind::Threshold,
    };
    Some(Directory {
        path: path.to_string(),
        size,
        scale: number("Scale").unwrap_or(1),
        kind,
        min: number("MinSize").unwrap_or(size),
        max: number("MaxSize").unwrap_or(size),
        threshold: number("Threshold").unwrap_or(2),
    })
}

/// The icon theme of the session: GNOME's setting, else GTK's `settings.ini`, else `hicolor`.
fn session_theme() -> String {
    if let Some(theme) = gsettings_theme() {
        return theme;
    }
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let config = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => home.join(".config"),
    };
    for gtk in ["gtk-4.0", "gtk-3.0"] {
        let Ok(text) = std::fs::read_to_string(config.join(gtk).join("settings.ini")) else {
            continue;
        };
        if let Some(theme) = theme_from_settings_ini(&text) {
            return theme;
        }
    }
    "hicolor".to_string()
}

fn gsettings_theme() -> Option<String> {
    let output = Command::new("gsettings").args(["get", "org.gnome.desktop.interface", "icon-theme"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    theme_from_gsettings(&String::from_utf8_lossy(&output.stdout))
}

/// `'Adwaita'` as `gsettings get` prints it.
fn theme_from_gsettings(output: &str) -> Option<String> {
    let theme = output.trim().trim_matches('\'').trim();
    (!theme.is_empty()).then(|| theme.to_string())
}

fn theme_from_settings_ini(text: &str) -> Option<String> {
    let theme = text.lines().find_map(|line| line.trim().strip_prefix("gtk-icon-theme-name="))?.trim();
    (!theme.is_empty()).then(|| theme.trim_matches('"').to_string())
}

static SHARED: OnceLock<Mutex<IconThemes>> = OnceLock::new();

fn shared() -> &'static Mutex<IconThemes> {
    SHARED.get_or_init(|| Mutex::new(IconThemes::from_environment()))
}

/// The file for an icon name in the session's theme (or `theme`), at a size on a display of `scale`.
pub fn find(name: &str, size: u32, scale: u32, theme: Option<&str>) -> Option<IconFile> {
    shared().lock().expect("icon themes").find(name, size, scale, theme)
}

/// Makes the next lookup read the theme from disk again, for a reload.
pub fn forget() {
    if let Some(shared) = SHARED.get() {
        shared.lock().expect("icon themes").forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A scratch directory that is removed when it goes out of scope.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            static COUNT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "svelte-gpui-icons-{}-{}",
                std::process::id(),
                COUNT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).expect("scratch directory");
            Scratch(path)
        }

        fn file(&self, relative: &str, contents: &str) -> PathBuf {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("directories");
            std::fs::write(&path, contents).expect("a file");
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const SIZES: &str = "[Icon Theme]\nName=Test\nDirectories=16x16/apps,32x32/apps,48x48/apps,scalable/apps\n\
        [16x16/apps]\nSize=16\nType=Fixed\n[32x32/apps]\nSize=32\nType=Fixed\n[48x48/apps]\nSize=48\nType=Fixed\n\
        [scalable/apps]\nSize=128\nMinSize=64\nMaxSize=256\nType=Scalable\n";

    fn themes(scratch: &Scratch, default: &str) -> IconThemes {
        IconThemes::new(vec![scratch.0.join("icons")], vec![scratch.0.join("pixmaps")], default.to_string())
    }

    #[test]
    fn a_size_with_its_own_directory_is_taken_from_it() {
        let scratch = Scratch::new();
        scratch.file("icons/Test/index.theme", SIZES);
        scratch.file("icons/Test/16x16/apps/app.png", "");
        let wanted = scratch.file("icons/Test/32x32/apps/app.png", "");
        scratch.file("icons/Test/48x48/apps/app.png", "");
        let found = themes(&scratch, "Test").find("app", 32, 1, None).expect("an icon");
        assert_eq!(found.path, wanted);
        assert_eq!(found.format, Format::Raster);
    }

    #[test]
    fn between_two_sizes_the_larger_wins_a_tie_and_the_nearer_otherwise() {
        let scratch = Scratch::new();
        scratch.file("icons/Test/index.theme", SIZES);
        scratch.file("icons/Test/16x16/apps/app.png", "");
        let thirty_two = scratch.file("icons/Test/32x32/apps/app.png", "");
        let forty_eight = scratch.file("icons/Test/48x48/apps/app.png", "");
        let mut themes = themes(&scratch, "Test");
        assert_eq!(themes.find("app", 40, 1, None).expect("an icon").path, forty_eight);
        assert_eq!(themes.find("app", 35, 1, None).expect("an icon").path, thirty_two);
        assert_eq!(
            themes.find("app", 20, 1, None).expect("an icon").path,
            scratch.0.join("icons/Test/16x16/apps/app.png")
        );
    }

    #[test]
    fn a_scalable_directory_serves_every_size_in_its_range() {
        let scratch = Scratch::new();
        scratch.file("icons/Test/index.theme", SIZES);
        scratch.file("icons/Test/48x48/apps/app.png", "");
        let vector = scratch.file("icons/Test/scalable/apps/app.svg", "");
        let found = themes(&scratch, "Test").find("app", 100, 1, None).expect("an icon");
        assert_eq!(found.path, vector);
        assert_eq!(found.format, Format::Svg);
    }

    #[test]
    fn a_vector_beats_a_raster_that_is_equally_far() {
        let scratch = Scratch::new();
        scratch.file(
            "icons/Test/index.theme",
            "[Icon Theme]\nDirectories=64x64/apps,scalable/apps\n[64x64/apps]\nSize=64\nType=Fixed\n\
             [scalable/apps]\nSize=64\nMinSize=64\nMaxSize=64\nType=Scalable\n",
        );
        scratch.file("icons/Test/64x64/apps/app.png", "");
        let vector = scratch.file("icons/Test/scalable/apps/app.svg", "");
        assert_eq!(themes(&scratch, "Test").find("app", 24, 1, None).expect("an icon").path, vector);
    }

    #[test]
    fn a_display_scale_picks_the_directories_made_for_it() {
        let scratch = Scratch::new();
        scratch.file(
            "icons/Test/index.theme",
            "[Icon Theme]\nDirectories=32x32/apps\nScaledDirectories=32x32@2/apps\n\
             [32x32/apps]\nSize=32\nType=Fixed\n[32x32@2/apps]\nSize=32\nScale=2\nType=Fixed\n",
        );
        scratch.file("icons/Test/32x32/apps/app.png", "");
        let sharp = scratch.file("icons/Test/32x32@2/apps/app.png", "");
        let mut themes = themes(&scratch, "Test");
        assert_eq!(themes.find("app", 32, 2, None).expect("an icon").path, sharp);
        assert_eq!(
            themes.find("app", 32, 1, None).expect("an icon").path,
            scratch.0.join("icons/Test/32x32/apps/app.png")
        );
    }

    #[test]
    fn a_theme_falls_back_on_the_themes_it_inherits() {
        let scratch = Scratch::new();
        scratch.file(
            "icons/Child/index.theme",
            "[Icon Theme]\nInherits=Parent,hicolor\nDirectories=32x32/apps\n[32x32/apps]\nSize=32\nType=Fixed\n",
        );
        scratch.file(
            "icons/Parent/index.theme",
            "[Icon Theme]\nInherits=Grand\nDirectories=32x32/apps\n[32x32/apps]\nSize=32\nType=Fixed\n",
        );
        scratch.file(
            "icons/Grand/index.theme",
            "[Icon Theme]\nDirectories=32x32/apps\n[32x32/apps]\nSize=32\nType=Fixed\n",
        );
        let own = scratch.file("icons/Child/32x32/apps/own.png", "");
        let inherited = scratch.file("icons/Grand/32x32/apps/old.png", "");
        scratch.file("icons/Parent/32x32/apps/shared.png", "from the parent");
        let overridden = scratch.file("icons/Child/32x32/apps/shared.png", "from the child");
        let mut themes = themes(&scratch, "Child");
        assert_eq!(themes.find("own", 32, 1, None).expect("an icon").path, own);
        assert_eq!(themes.find("old", 32, 1, None).expect("an icon").path, inherited);
        assert_eq!(themes.find("shared", 32, 1, None).expect("an icon").path, overridden);
        assert_eq!(themes.find("nothing", 32, 1, None), None);
    }

    #[test]
    fn inheriting_in_a_circle_ends() {
        let scratch = Scratch::new();
        scratch.file("icons/A/index.theme", "[Icon Theme]\nInherits=B\nDirectories=\n");
        scratch.file("icons/B/index.theme", "[Icon Theme]\nInherits=A\nDirectories=\n");
        assert_eq!(themes(&scratch, "A").find("anything", 32, 1, None), None);
    }

    #[test]
    fn hicolor_and_then_pixmaps_catch_what_the_theme_lacks() {
        let scratch = Scratch::new();
        scratch.file("icons/Plain/index.theme", "[Icon Theme]\nDirectories=\n");
        scratch.file("icons/hicolor/index.theme", SIZES);
        let hicolor = scratch.file("icons/hicolor/48x48/apps/tool.png", "");
        let pixmap = scratch.file("pixmaps/legacy.svg", "");
        let mut themes = themes(&scratch, "Plain");
        assert_eq!(themes.find("tool", 48, 1, None).expect("an icon").path, hicolor);
        let legacy = themes.find("legacy", 48, 1, None).expect("an icon");
        assert_eq!((legacy.path, legacy.format), (pixmap, Format::Svg));
    }

    #[test]
    fn a_theme_that_is_not_installed_uses_hicolor() {
        let scratch = Scratch::new();
        scratch.file("icons/hicolor/index.theme", SIZES);
        let hicolor = scratch.file("icons/hicolor/32x32/apps/tool.png", "");
        assert_eq!(themes(&scratch, "Missing").find("tool", 32, 1, None).expect("an icon").path, hicolor);
    }

    #[test]
    fn a_theme_given_with_the_lookup_replaces_the_session_theme() {
        let scratch = Scratch::new();
        scratch.file("icons/One/index.theme", SIZES);
        scratch.file("icons/Two/index.theme", SIZES);
        scratch.file("icons/One/32x32/apps/app.png", "");
        let two = scratch.file("icons/Two/32x32/apps/app.png", "");
        assert_eq!(themes(&scratch, "One").find("app", 32, 1, Some("Two")).expect("an icon").path, two);
    }

    #[test]
    fn a_theme_split_over_several_base_directories_is_read_across_them() {
        let scratch = Scratch::new();
        scratch.file("user/Test/index.theme", SIZES);
        let system = scratch.file("system/Test/32x32/apps/app.png", "");
        let mut themes =
            IconThemes::new(vec![scratch.0.join("user"), scratch.0.join("system")], Vec::new(), "Test".into());
        assert_eq!(themes.find("app", 32, 1, None).expect("an icon").path, system);
    }

    #[test]
    fn the_symbolic_flag_follows_the_name() {
        let scratch = Scratch::new();
        scratch.file("icons/Test/index.theme", SIZES);
        scratch.file("icons/Test/scalable/apps/audio-volume-high-symbolic.svg", "");
        scratch.file("icons/Test/scalable/apps/audio-volume-high.svg", "");
        let mut themes = themes(&scratch, "Test");
        assert!(themes.find("audio-volume-high-symbolic", 16, 1, None).expect("an icon").symbolic);
        assert!(!themes.find("audio-volume-high", 16, 1, None).expect("an icon").symbolic);
    }

    #[test]
    fn an_absolute_path_is_the_file_itself() {
        let scratch = Scratch::new();
        let png = scratch.file("opt/app/icon.png", "");
        let svg = scratch.file("opt/app/logo-symbolic.svg", "");
        let mut themes = themes(&scratch, "Test");
        let png = themes.find(png.to_str().expect("utf-8"), 32, 1, None).expect("an icon");
        assert_eq!(png.format, Format::Raster);
        assert!(!png.symbolic);
        let svg = themes.find(svg.to_str().expect("utf-8"), 32, 1, None).expect("an icon");
        assert_eq!(svg.format, Format::Svg);
        assert!(svg.symbolic);
        assert_eq!(themes.find("/nowhere/icon.png", 32, 1, None), None);
        assert_eq!(themes.find(scratch.0.join("opt/app").to_str().expect("utf-8"), 32, 1, None), None, "a directory");
    }

    #[test]
    fn a_file_name_is_taken_for_the_icon_name() {
        let scratch = Scratch::new();
        scratch.file("icons/Test/index.theme", SIZES);
        let wanted = scratch.file("icons/Test/32x32/apps/firefox.png", "");
        assert_eq!(themes(&scratch, "Test").find("firefox.png", 32, 1, None).expect("an icon").path, wanted);
    }

    #[test]
    fn the_themes_are_read_once() {
        let scratch = Scratch::new();
        scratch.file("icons/Test/index.theme", SIZES);
        let wanted = scratch.file("icons/Test/32x32/apps/app.png", "");
        let mut themes = themes(&scratch, "Test");
        assert!(themes.find("app", 32, 1, None).is_some());
        std::fs::remove_file(&wanted).expect("removed");
        assert!(themes.find("app", 32, 1, None).is_some(), "answered from what it knew");
        themes.forget();
        assert!(themes.find("app", 32, 1, None).is_none(), "looked at the disk again");
    }

    #[test]
    fn index_theme_defaults_follow_the_specification() {
        let theme = parse_index("[Icon Theme]\nDirectories=a\n[a]\nSize=24\n");
        let directory = &theme.directories[0];
        assert_eq!((directory.scale, directory.threshold, directory.min, directory.max), (1, 2, 24, 24));
        assert_eq!(directory.kind, Kind::Threshold);
        assert!(directory.matches(26, 1) && !directory.matches(27, 1));
    }

    #[test]
    fn the_session_theme_is_read_from_gsettings_and_gtk_settings() {
        assert_eq!(theme_from_gsettings("'Adwaita'\n"), Some("Adwaita".to_string()));
        assert_eq!(theme_from_gsettings("''\n"), None);
        let ini = "[Settings]\ngtk-theme-name=adw-gtk3\ngtk-icon-theme-name=Papirus-Dark\n";
        assert_eq!(theme_from_settings_ini(ini), Some("Papirus-Dark".to_string()));
        assert_eq!(theme_from_settings_ini("[Settings]\n"), None);
    }
}
