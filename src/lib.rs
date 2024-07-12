use std::{collections::HashMap, path::Path};

use bevy_ecs::system::Resource;

#[cfg(feature = "ods")]
use spreadsheet_ods::CellContent;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// An enum describing the currently supported types of table storage, As well as some reference data for loading different columns
pub enum TableFile {
    #[cfg(feature = "csv")]
    Csv(String),
    #[cfg(feature = "csv")]
    CsvRaw(String),
    #[cfg(feature = "ods")]
    Ods(String),
    None,
}
#[derive(Clone, Debug, Default)]
/// A helper struct for storing the two segments commonly used to denote a locale and region.
pub struct LocaleCode {
    lang: String,
    region: String,
}

impl PartialEq for LocaleCode {
    fn eq(&self, other: &Self) -> bool {
        self.lang.to_lowercase() == other.lang.to_lowercase()
            && self.region.to_lowercase() == other.region.to_lowercase()
    }
}

impl LocaleCode {
    // TODO: make this overrideable.
    /// The delimiter expected and produced when combining language and region codes in a LocalCode
    pub const REGION_DELIMITER: &'static str = "-";
}

impl From<LocaleCode> for String {
    fn from(value: LocaleCode) -> Self {
        if value.region.is_empty() {
            value.lang.clone()
        } else {
            format!(
                "{}{}{}",
                value.lang,
                LocaleCode::REGION_DELIMITER,
                value.region
            )
        }
    }
}

impl<T> From<T> for LocaleCode
where
    T: ToString,
{
    fn from(value: T) -> Self {
        let code = value.to_string();
        if let Some((lang, region)) = code.split_once(Self::REGION_DELIMITER) {
            return LocaleCode {
                lang: lang.trim().into(),
                region: region.trim().into(),
            };
        } else {
            return LocaleCode {
                lang: code.trim().into(),
                region: "".into(),
            };
        }
    }
}

#[derive(Clone, PartialEq, Debug, Resource)]
/// The main Resource type that stores translation data.
pub struct Translations {
    locale: LocaleCode,
    path: TableFile,
    available_locales: Vec<LocaleCode>,
    mappings: HashMap<String, String>,
}

impl Default for Translations {
    fn default() -> Self {
        Self {
            locale: LocaleCode::default(),
            path: TableFile::None,
            available_locales: Vec::new(),
            mappings: HashMap::new(),
        }
    }
}

impl Translations {
    pub fn new() -> Self {
        Self::default()
    }
    /// The short call to acquire a translation. Translations work through a key-value pair that are loaded based on the currently selected locale.
    /// Here we specificially take a generic argument for the key such that any value that implements `ToString` can be translated. This creates a decent amount of flexibility for users as they will be able to "translate" custom types if they so choose.
    pub fn tr(&self, key: impl ToString) -> String {
        if let Some(value) = self.mappings.get(&key.to_string()).cloned() {
            value
        } else {
            if cfg!(feature = "catch-missing-values") {
                eprintln!(
                    "missing translation value : {} has no translation value for locale {:?}",
                    key.to_string(),
                    self.locale
                );
            }
            key.to_string()
        }
    }

    /// Modifies the current Translations data to load from a specified ODS file and load a particular locale.
    #[cfg(feature = "ods")]
    fn ods_file(&mut self, file: &Path, locale: &String) -> &mut Self {
        // note: remember that ODS (and any other spreadsheet) will index starting at 1, not 0!!

        use std::{fs::File, io::BufReader};

        let Ok(f) = File::open(file) else {
            eprintln!("Failed to locate file: {}", file.display());
            return self;
        };
        let reader = BufReader::new(f);

        let Ok(workbook) = spreadsheet_ods::OdsOptions::default()
            .content_only()
            .read_ods(reader)
        else {
            eprintln!("Failed to load ODS spreadsheet file at {:?}", file);
            return self;
        };
        if workbook.num_sheets() == 0 {
            eprintln!("Attempted to load empty spreadsheet file at {:?}", file);
            return self;
        }
        self.path = TableFile::Ods(file.to_str().unwrap_or_default().into());
        let sheet = workbook.sheet(0);
        let size = sheet.used_grid_size();

        self.available_locales = Vec::new();
        for x in 0..size.1 {
            if let Some(cell) = sheet.cell(0, x) {
                let str_value = Self::get_cell_text(&cell);
                if !str_value.is_empty() {
                    self.available_locales.push(str_value.into());
                }
            } else {
                eprintln!("Failed to load cell at row={}, col={}", 0, x);
            }
        }

        let pref_code: LocaleCode = locale.into();
        let locale_index: u32 = match self.available_locales.iter().position(|p| *p == pref_code) {
            Some(index) => u32::try_from(index).unwrap_or_default(),
            None => 0,
        };
        self.locale = locale.into();
        self.mappings = HashMap::new();
        for y in 1..size.0 {
            let Some(key) = sheet.cell(y, 0) else {
                continue;
            };
            let Some(value) = sheet.cell(y, locale_index) else {
                continue;
            };
            self.mappings
                .insert(Self::get_cell_text(&key), Self::get_cell_text(&value));
        }
        self
    }

    #[cfg(feature = "ods")]
    fn get_cell_text(cell: &CellContent) -> String {
        match &cell.value {
            spreadsheet_ods::Value::Empty => "".into(),
            spreadsheet_ods::Value::Boolean(b) => b.to_string(),
            spreadsheet_ods::Value::Number(n) => n.to_string(),
            spreadsheet_ods::Value::Percentage(p) => format!("{}%", p * 100.),
            spreadsheet_ods::Value::Currency(v, c) => format!("{}{}", c, v),
            spreadsheet_ods::Value::Text(t) => t.clone(),
            spreadsheet_ods::Value::TextXml(x) => {
                for tag in x {
                    for c in tag.content() {
                        if let spreadsheet_ods::xmltree::XmlContent::Text(t) = c {
                            return t.clone();
                        }
                    }
                }
                "".into()
            }
            spreadsheet_ods::Value::DateTime(dt) => dt.to_string(),
            spreadsheet_ods::Value::TimeDuration(dur) => dur.to_string(),
        }
    }

    /// Modifies the current Translations data to load from a specified CSV file and load a particular locale.
    #[cfg(feature = "csv")]
    pub fn csv_file(&mut self, path: &Path, locale: &String) -> &mut Self {
        let Ok(mut reader) = csv::ReaderBuilder::new()
            .has_headers(true)
            .double_quote(false)
            .escape(Some(b'\\'))
            .flexible(true)
            .from_path(path)
        else {
            eprintln!("Failed to load csv file: {}", path.display());
            return self;
        };
        self.path = TableFile::Csv(path.to_str().unwrap_or_default().into());

        let Ok(head) = reader.headers() else {
            eprintln!("Failed to collect header row from reader");
            return self;
        };
        let locales = head
            .into_iter()
            .map(|s| s.to_string().trim().into())
            .collect::<Vec<String>>();
        if locales.is_empty() {
            eprintln!("Collected empty locale list!");
        }

        let locale_index = locales.iter().position(|p| p == locale).unwrap_or_default();
        if locale_index == 0 {
            eprintln!(
                "Locale index not found for locale {:?} in set {:#?}",
                locale, locales
            )
        }
        self.locale = locale.into();
        let mapping = reader
            .records()
            .map(|p| {
                let rec = p.unwrap_or_default();
                (
                    rec.get(0).unwrap_or_default().to_string(),
                    rec.get(locale_index).unwrap_or_default().to_string(),
                )
            })
            .collect::<Vec<(String, String)>>();
        self.data(locales.into_iter(), mapping.into_iter(), true)
    }

    /// Modifies the current Translations data to load from a raw string in CSV format and load a particular locale.
    #[cfg(feature = "csv")]
    pub fn csv_raw(&mut self, csv_data: String, locale: &String) -> &mut Self {
        let mut reader = csv::ReaderBuilder::new()
            .double_quote(false)
            .escape(Some(b'\\'))
            .flexible(true)
            .has_headers(true)
            .from_reader(csv_data.as_bytes());
        self.path = TableFile::CsvRaw(csv_data.clone());

        let Ok(head) = reader.headers() else {
            eprintln!("Failed to collect header row from reader");
            return self;
        };
        let locales = head
            .into_iter()
            .map(|s| s.to_string().trim().into())
            .collect::<Vec<String>>();
        if locales.is_empty() {
            eprintln!("Collected empty locale list!");
        }

        let locale_index = locales.iter().position(|p| p == locale).unwrap_or_default();
        if locale_index == 0 {
            eprintln!(
                "Locale index not found for locale {:?} in set {:#?}",
                locale, locales
            )
        }
        self.locale = locale.into();

        let mapping = reader
            .records()
            .map(|p| {
                let rec = p.unwrap_or_default();
                (
                    rec.get(0).unwrap_or_default().to_string(),
                    rec.get(locale_index).unwrap_or_default().to_string(),
                )
            })
            .collect::<Vec<(String, String)>>();
        self.data(locales.into_iter(), mapping.into_iter(), true)
    }

    /// Modifies the current Translations data to load from raw data.
    /// Note that using this method directly does not support changing locales. If you want that feature, you must use CSV or ODS
    pub fn data<S>(
        &mut self,
        locales: impl Iterator<Item = S>,
        mapping: impl Iterator<Item = (S, S)>,
        clear_old_data: bool,
    ) -> &mut Self
    where
        S: ToString,
    {
        if clear_old_data {
            self.available_locales.clear();
            self.mappings.clear();
        }
        self.available_locales = locales.map(|code| code.to_string().trim().into()).collect();
        for (key, value) in mapping {
            self.mappings.insert(
                key.to_string().trim().into(),
                value.to_string().trim().into(),
            );
        }
        self
    }

    /// A convenience method for calling `use_locale` with the system's default locale.
    #[cfg(feature = "auto")]
    pub fn use_system_locale(&mut self) -> &mut Self {
        self.use_locale(Self::get_system_language().unwrap_or(String::from(
            self.available_locales.first().cloned().unwrap_or_default(),
        )))
    }

    /// Change the current locale to the new locale if available. Also loads the new mapping data allowing for translations to be loaded immediately.
    pub fn use_locale<S>(&mut self, locale: S) -> &mut Self
    where
        S: ToString + Clone,
    {
        // validate this format has a way to load different locales
        let path = self.path.clone();
        if path == TableFile::None {
            eprintln!("Current data format does not allow loading different translation columns.");
            return self;
        }

        // validate the requested locale is available
        let code: LocaleCode = locale.clone().into();
        if !self.available_locales.contains(&code) {
            eprintln!("Requested locale is not available: requested {:?}", code);
            return self;
        }
        // self.locale = code;

        // collect the key-value pairs based on the current file format
        match path {
            #[cfg(feature = "csv")]
            TableFile::Csv(str_path) => self.csv_file(Path::new(&str_path), &String::from(code)),

            #[cfg(feature = "csv")]
            TableFile::CsvRaw(raw_data) => self.csv_raw(raw_data, &String::from(code)),

            #[cfg(feature = "ods")]
            TableFile::Ods(str_path) => self.ods_file(Path::new(&str_path), &String::from(code)),

            TableFile::None => {
                unreachable!()
            }
        }
    }

    /// Returns an optional string if able to acquire the system's current locale code. Basically just a small wrapper around `bevy_device_lang` for convenience
    #[cfg(feature = "auto")]
    pub fn get_system_language() -> Option<String> {
        bevy_device_lang::get_lang()
    }

    /// Consumes and clones the instance to make inserting the resource into a bevy App or World a bit easier when using the builder pattern.
    pub fn build(&self) -> Self {
        self.clone() // probably not best practice :/
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    // need to escape out of the target directory

    // assets/lang.ods
    // target/debug/__.rlib
    const FILE_CSV: &str = "assets/lang.csv";
    const FILE_ODS: &str = "assets/lang.ods";

    #[test]
    fn locale_code_lang() {
        const LOCALE: [&str; 3] = ["en", "es", "fr"];
        for loc in LOCALE.into_iter() {
            let code: LocaleCode = loc.into();
            assert_eq!(code.lang, loc.to_string());
            assert_eq!(code.region, "".to_string());
        }
    }
    #[test]
    fn locale_code_lang_region() {
        const LOCALE: [(&str, &str); 3] = [("en", "AU"), ("es", "CL"), ("fr", "CI")];
        for (lang, region) in LOCALE.into_iter() {
            let code: LocaleCode =
                format!("{}{}{}", lang, LocaleCode::REGION_DELIMITER, region).into();
            assert_eq!(code.lang, lang.to_string());
            assert_eq!(code.region, region.to_string());
        }
    }

    #[test]
    #[cfg(feature = "csv")]
    fn load_csv_file() {
        if let Ok(pwd) = std::env::current_dir() {
            eprintln!("PWD ==> {}", pwd.display());
        }
        let mut t = Translations::default();
        t.csv_file(&Path::new(FILE_CSV), &"en".into());
        validate_translation_data(&mut t);
    }

    #[test]
    #[cfg(feature = "csv")]
    pub fn load_csv_raw() {
        const CSV_DATA_RAW: &'static str = r#"key, en, es
hello, hello, hola,
green, green, verde"#;

        let mut t = Translations::default();
        t.csv_raw(CSV_DATA_RAW.into(), &"en".into());
        validate_translation_data(&mut t);
    }
    #[test]
    #[cfg(feature = "ods")]
    fn load_ods() {
        let mut t = Translations::default();
        t.ods_file(&Path::new(FILE_ODS), &"en".into());
        validate_translation_data(&mut t);
    }

    #[test]
    fn load_data_raw() {
        let locales: &[&str; 1] = &["es"];
        let mappings = vec![(&"hello", &"hola"), (&"green", &"verde")];

        let mut t = Translations::default();
        t.data(locales.iter(), mappings.into_iter(), true);
        assert_eq!(t.tr("hello"), "hola");
        assert_eq!(t.tr("green"), "verde");
        assert_eq!(t.tr("invalid"), "invalid");
    }

    fn validate_translation_data(trans: &mut Translations) {
        // eprintln!("Raw Loaded: {:#?}\n", trans);

        trans.use_locale("en");
        // eprintln!("EN: {:#?}", trans);
        assert_eq!(trans.tr("hello"), "hello");
        assert_eq!(trans.tr("green"), "green");
        assert_eq!(trans.tr("invalid"), "invalid");

        trans.use_locale("es");
        eprintln!("ES: {:#?}", trans);
        assert_eq!(trans.tr("hello"), "hola");
        assert_eq!(trans.tr("green"), "verde");
        assert_eq!(trans.tr("invalid"), "invalid");
    }
}
