use std::collections::HashMap;
use vk_macros::{EnumCount, ToString, Vector};

const EN_LOCALE: &str = include_str!("../locales/en.lang");
const ES_LOCALE: &str = include_str!("../locales/es.lang");
const DE_LOCALE: &str = include_str!("../locales/de.lang");

pub struct I18N {
    locale: HashMap<String, String>,
    fallback_locale: HashMap<String, String>,
}

impl Default for I18N {
    fn default() -> Self {
        let locale = parse_locale(EN_LOCALE);
        Self {
            fallback_locale: locale.clone(),
            locale,
        }
    }
}

#[derive(Clone, Copy, Vector, ToString, PartialEq)]
pub enum Language {
    English,
    German,
    Spanish,
}

impl From<String> for Language {
    fn from(s: String) -> Self {
        if s.to_lowercase() == "english" {
            Language::English
        } else if s.to_lowercase() == "german" {
            Language::German
        } else if s.to_lowercase() == "spanish" {
            Language::Spanish
        } else {
            Language::from(s)
        }
    }
}

#[derive(Clone, Copy, EnumCount)]
pub enum LangKey {
    AddConnection,
    AliasRequiredField,
    AppName,
    Browser,
    Cancel,
    CannotConvertValkeyUrl,
    Connect,
    Connections,
    ConnectionString,
    ConnectionSuccess,
    ConnectionType,
    DatabaseAlias,
    DatabaseIndex,
    Delete,
    Edit,
    EmptyRequiredFields,
    FailedToSendErrorMessage(&'static str),
    HideSidebar,
    HostRequiredField,
    LastConnection,
    NewWindow,
    NoConnections,
    NothingToDisplay,
    Ok,
    Password,
    Port,
    QuickConnect,
    Quit,
    Save,
    SearchConnections,
    ShowPassword,
    ShowSidebar,
    TestConnection,
    Undefined,
    Username,
    ValkeyDatabase,
    Window,
    Workbench,
    Insights,
    Documentation,
    Host,
    ConnectingToServer,
    ConnectingToServerTakesAWhile,
    NoValidAddress,
    AuthFailed,
    SelectDbFail,
    ServerConnectionFailed,
    IdentifyServerFailed,
    GetServerVersionFailed,
    UnsupportedValkeyServerError(u8, u8, &'static str),
    UnsupportedServer,
    PartiallySupportedServerError(u8, u8, &'static str),
    YourServer,
    Version,
    Settings,
    Apply,
    Language,
    German,
    English,
    Spanish,
    SelectLanguage,
    KeyType,
    NewKey,
    EditKey,
    FilterByKeyNameOrPattern,
    Index,
    Type,
    Length,
    Ttl,
    Key,
    Rename,
    SetTtlFor,
    DeleteKey,
    NewKeyName,
    AreYouSure,
    Yes,
    No,
    SetTtl,
    Size,
    ErrorSendingRefreshWinMsg,
    CantAccessValkeyDb,
    ErrorSendMsg,
    Copy,
    Keys,
    UnknownKeyType,
    FailedSpawnDetachedInstance,
    RespCommand,
    Exec,
    Executing,
    NoResponse,
    AnErrorOccurred,
    Result,
    OverallTimeoutExceeded,
    ConnectionClosedWithoutResponse,
    IncompleteData,
    IncompleteSimpleType,
    InvalidBulkStringLength,
    IncompleteBulkStringData,
    IncompleteBulkStringHeader,
    InvalidArrayHeader,
    InvalidArrayCount,
    IncompleteArrayHeader,
    InvalidBulkErrorHeader,
    InvalidBulkErrorLength,
    IncompleteBulkErrorData,
    IncompleteBulkErrorHeader,
    InvalidMapHeader,
    InvalidMapLength,
    IncompleteMapHeader,
    InvalidCollectionHeader,
    InvalidCollectionLength,
    InvalidVerbatimStringHeader,
    InvalidVerbatimStringLength,
    IncompleteVerbatimStringData,
    IncompleteVerbatimStringHeader,
    UnknownRespType,
    NoData,
    AddNew,
    BloomFilterInformation,
    NumberOfItemsInserted,
    Capacity,
    MaxScaledCapacity,
    NumberOfFilters,
    ErrorRate,
    ExpansionRate,
    TighteningRatio,
    Items,
    MaxCapacity,
    Filters,
    Expansion,
    Tightening,
    MaxCap,
    Error,
    Tight,
    Summary,
    Fill,
    Element,
    Member,
    Score,
    Value,
    Add,
    LoadingKeyData,
    CommandHistory,
}

impl I18N {
    pub fn new(language: Language) -> Self {
        let data = match language {
            Language::English => EN_LOCALE,
            Language::Spanish => ES_LOCALE,
            Language::German => DE_LOCALE,
        };
        Self {
            locale: parse_locale(data),
            fallback_locale: Default::default(),
        }
    }

    pub fn get(&self, key: LangKey) -> String {
        match key {
            LangKey::AddConnection => self.get_lang("ADD_CONNECTION"),
            LangKey::FailedToSendErrorMessage(s) => {
                let template = self.get_lang("FAILED_TO_SEND_ERROR_MESSAGE");
                let mut params = HashMap::new();
                params.insert("message", s);
                fill_template(&template, &params)
            }
            LangKey::Connect => self.get_lang("CONNECT"),
            LangKey::QuickConnect => self.get_lang("QUICK_CONNECT"),
            LangKey::AppName => self.get_lang("APP_NAME"),
            LangKey::SearchConnections => self.get_lang("SEARCH_CONNECTIONS"),
            LangKey::NoConnections => self.get_lang("NO_CONNECTIONS"),
            LangKey::DatabaseAlias => self.get_lang("DATABASE_ALIAS"),
            LangKey::ConnectionType => self.get_lang("CONNECTION_TYPE"),
            LangKey::LastConnection => self.get_lang("LAST_CONNECTION"),
            LangKey::Edit => self.get_lang("EDIT"),
            LangKey::Delete => self.get_lang("DELETE"),
            LangKey::Connections => self.get_lang("CONNECTIONS"),
            LangKey::Browser => self.get_lang("BROWSER"),
            LangKey::Workbench => self.get_lang("WORKBENCH"),
            LangKey::Insights => self.get_lang("INSIGHTS"),
            LangKey::Documentation => self.get_lang("DOCUMENTATION"),
            LangKey::Window => self.get_lang("WINDOW"),
            LangKey::NewWindow => self.get_lang("NEW_WINDOW"),
            LangKey::ShowSidebar => self.get_lang("SHOW_SIDEBAR"),
            LangKey::HideSidebar => self.get_lang("HIDE_SIDEBAR"),
            LangKey::Quit => self.get_lang("QUIT"),
            LangKey::Ok => self.get_lang("OK"),
            LangKey::Cancel => self.get_lang("CANCEL"),
            LangKey::Undefined => self.get_lang("UNDEFINED"),
            LangKey::NothingToDisplay => self.get_lang("NOTHING_TO_DISPLAY"),
            LangKey::ConnectionString => self.get_lang("CONNECTION_STRING"),
            LangKey::ShowPassword => self.get_lang("SHOW_PASSWORD"),
            LangKey::ValkeyDatabase => self.get_lang("VALKEY_DATABASE"),
            LangKey::Port => self.get_lang("PORT"),
            LangKey::Username => self.get_lang("USERNAME"),
            LangKey::Password => self.get_lang("PASSWORD"),
            LangKey::DatabaseIndex => self.get_lang("DATABASE_INDEX"),
            LangKey::TestConnection => self.get_lang("TEST_CONNECTION"),
            LangKey::ConnectionSuccess => self.get_lang("CONNECTION_SUCCESS"),
            LangKey::Save => self.get_lang("SAVE"),
            LangKey::AliasRequiredField => self.get_lang("ALIAS_REQUIRED_FIELD"),
            LangKey::HostRequiredField => self.get_lang("HOST_REQUIRED_FIELD"),
            LangKey::EmptyRequiredFields => self.get_lang("EMPTY_REQUIRED_FIELDS"),
            LangKey::CannotConvertValkeyUrl => self.get_lang("CANNOT_CONVERT_VALKEY_URL"),
            LangKey::Host => self.get_lang("HOST"),
            LangKey::ConnectingToServer => self.get_lang("CONNECTING_TO_SERVER"),
            LangKey::ConnectingToServerTakesAWhile => {
                self.get_lang("CONNECTING_TO_SERVER_TAKES_A_WHILE")
            }
            LangKey::NoValidAddress => self.get_lang("NO_VALID_ADDRESS"),
            LangKey::AuthFailed => self.get_lang("AUTH_FAILED"),
            LangKey::SelectDbFail => self.get_lang("SELECT_DB_FAIL"),
            LangKey::ServerConnectionFailed => self.get_lang("SERVER_CONNECTION_FAILED"),
            LangKey::IdentifyServerFailed => self.get_lang("IDENTIFY_SERVER_FAILED"),
            LangKey::GetServerVersionFailed => self.get_lang("GET_SERVER_VERSION_FAILED"),
            LangKey::UnsupportedValkeyServerError(major, minor, protocol) => {
                let template = self.get_lang("UNSUPPORTED_VALKEY_SERVER_ERROR");
                let mut params: HashMap<&str, &str> = HashMap::new();
                let major = major.to_string();
                let minor = minor.to_string();
                params.insert("expected_version_major", &major);
                params.insert("expected_version_minor", &minor);
                params.insert("expected_protocols", protocol);
                fill_template(&template, &params)
            }
            LangKey::UnsupportedServer => self.get_lang("UNSUPPORTED_SERVER"),
            LangKey::PartiallySupportedServerError(major, minor, protocol) => {
                let template = self.get_lang("PARTIALLY_SUPPORTED_SERVER_ERROR");
                let mut params: HashMap<&str, &str> = HashMap::new();
                let major = major.to_string();
                let minor = minor.to_string();
                params.insert("expected_version_major", &major);
                params.insert("expected_version_minor", &minor);
                params.insert("expected_protocols", protocol);
                fill_template(&template, &params)
            }
            LangKey::YourServer => self.get_lang("YOUR_SERVER"),
            LangKey::Version => self.get_lang("VERSION"),
            LangKey::Settings => self.get_lang("SETTINGS"),
            LangKey::Apply => self.get_lang("APPLY"),
            LangKey::Language => self.get_lang("LANGUAGE"),
            LangKey::German => self.get_lang("GERMAN"),
            LangKey::English => self.get_lang("ENGLISH"),
            LangKey::Spanish => self.get_lang("SPANISH"),
            LangKey::SelectLanguage => self.get_lang("SELECT_LANGUAGE"),
            LangKey::KeyType => self.get_lang("KEY_TYPE"),
            LangKey::NewKey => self.get_lang("NEW_KEY"),
            LangKey::EditKey => self.get_lang("EDIT_KEY"),
            LangKey::FilterByKeyNameOrPattern => self.get_lang("FILTER_BY_KEY_NAME_OR_PATTERN"),
            LangKey::Index => self.get_lang("INDEX"),
            LangKey::Type => self.get_lang("TYPE"),
            LangKey::Length => self.get_lang("LENGTH"),
            LangKey::Ttl => self.get_lang("TTL"),
            LangKey::Key => self.get_lang("KEY"),
            LangKey::Rename => self.get_lang("RENAME"),
            LangKey::SetTtlFor => self.get_lang("SET_TTL_FOR"),
            LangKey::DeleteKey => self.get_lang("DELETE_KEY"),

            LangKey::NewKeyName => self.get_lang("NEW_KEY_NAME"),
            LangKey::AreYouSure => self.get_lang("ARE_YOU_SURE"),
            LangKey::Yes => self.get_lang("YES"),
            LangKey::No => self.get_lang("NO"),
            LangKey::SetTtl => self.get_lang("SET_TTL"),
            LangKey::Size => self.get_lang("SIZE"),
            LangKey::ErrorSendingRefreshWinMsg => self.get_lang("ERROR_SENDING_REFRESH_WIN_MSG"),
            LangKey::CantAccessValkeyDb => self.get_lang("CANT_ACCESS_VALKEY_DB"),
            LangKey::ErrorSendMsg => self.get_lang("ERROR_SEND_MSG"),
            LangKey::Copy => self.get_lang("COPY"),
            LangKey::Keys => self.get_lang("KEYS"),
            LangKey::UnknownKeyType => self.get_lang("UNKNOWN_KEY_TYPE"),
            LangKey::FailedSpawnDetachedInstance => self.get_lang("FAILED_SPAWN_DETACHED_INSTANCE"),
            LangKey::RespCommand => self.get_lang("RESP_COMMAND"),
            LangKey::Exec => self.get_lang("EXEC"),
            LangKey::Executing => self.get_lang("EXECUTING"),
            LangKey::NoResponse => self.get_lang("NO_RESPONSE"),
            LangKey::AnErrorOccurred => self.get_lang("AN_ERROR_OCCURRED"),
            LangKey::Result => self.get_lang("RESULT"),
            LangKey::OverallTimeoutExceeded => self.get_lang("OVERALL_TIMEOUT_EXCEEDED"),
            LangKey::ConnectionClosedWithoutResponse => {
                self.get_lang("CONNECTION_CLOSED_WITHOUT_RESPONSE")
            }
            LangKey::IncompleteData => self.get_lang("INCOMPLETE_DATA"),
            LangKey::IncompleteSimpleType => self.get_lang("INCOMPLETE_SIMPLE_TYPE"),
            LangKey::InvalidBulkStringLength => self.get_lang("INVALID_BULK_STRING_LENGTH"),
            LangKey::IncompleteBulkStringData => self.get_lang("INCOMPLETE_BULK_STRING_DATA"),
            LangKey::IncompleteBulkStringHeader => self.get_lang("INCOMPLETE_BULK_STRING_HEADER"),
            LangKey::InvalidArrayHeader => self.get_lang("INVALID_ARRAY_HEADER"),
            LangKey::InvalidArrayCount => self.get_lang("INVALID_ARRAY_COUNT"),
            LangKey::IncompleteArrayHeader => self.get_lang("INCOMPLETE_ARRAY_HEADER"),
            LangKey::InvalidBulkErrorHeader => self.get_lang("INVALID_BULK_ERROR_HEADER"),
            LangKey::InvalidBulkErrorLength => self.get_lang("INVALID_BULK_ERROR_LENGTH"),
            LangKey::IncompleteBulkErrorData => self.get_lang("INCOMPLETE_BULK_ERROR_DATA"),
            LangKey::IncompleteBulkErrorHeader => self.get_lang("INCOMPLETE_BULK_ERROR_HEADER"),
            LangKey::InvalidMapHeader => self.get_lang("INVALID_MAP_HEADER"),
            LangKey::InvalidMapLength => self.get_lang("INVALID_MAP_LENGTH"),
            LangKey::IncompleteMapHeader => self.get_lang("INCOMPLETE_MAP_HEADER"),
            LangKey::InvalidCollectionHeader => self.get_lang("INVALID_COLLECTION_HEADER"),
            LangKey::InvalidCollectionLength => self.get_lang("INVALID_COLLECTION_LENGTH"),
            LangKey::InvalidVerbatimStringHeader => self.get_lang("INVALID_VERBATIM_STRING_HEADER"),
            LangKey::InvalidVerbatimStringLength => self.get_lang("INVALID_VERBATIM_STRING_LENGTH"),
            LangKey::IncompleteVerbatimStringData => {
                self.get_lang("INCOMPLETE_VERBATIM_STRING_DATA")
            }
            LangKey::IncompleteVerbatimStringHeader => {
                self.get_lang("INCOMPLETE_VERBATIM_STRING_HEADER")
            }
            LangKey::UnknownRespType => self.get_lang("UNKNOWN_RESP_TYPE"),
            LangKey::NoData => self.get_lang("NO_DATA"),
            LangKey::AddNew => self.get_lang("ADD_NEW"),
            LangKey::BloomFilterInformation => self.get_lang("BLOOM_FILTER_INFORMATION"),
            LangKey::NumberOfItemsInserted => self.get_lang("NUMBER_OF_ITEMS_INSERTED"),
            LangKey::Capacity => self.get_lang("Capacity"),
            LangKey::MaxScaledCapacity => self.get_lang("MAX_SCALED_CAPACITY"),
            LangKey::NumberOfFilters => self.get_lang("NUMBER_OF_FILTERS"),
            LangKey::ErrorRate => self.get_lang("ERROR_RATE"),
            LangKey::ExpansionRate => self.get_lang("EXPANSION_RATE"),
            LangKey::TighteningRatio => self.get_lang("TIGHTENING_RATIO"),
            LangKey::Items => self.get_lang("ITEMS"),
            LangKey::MaxCapacity => self.get_lang("MAX_CAPACITY"),
            LangKey::Filters => self.get_lang("FILTERS"),
            LangKey::Expansion => self.get_lang("EXPANSION"),
            LangKey::Tightening => self.get_lang("TIGHTENING"),
            LangKey::MaxCap => self.get_lang("MAX_CAP"),
            LangKey::Error => self.get_lang("ERROR"),
            LangKey::Tight => self.get_lang("TIGHT"),
            LangKey::Summary => self.get_lang("SUMMARY"),
            LangKey::Fill => self.get_lang("FILL"),
            LangKey::Element => self.get_lang("ELEMENT"),
            LangKey::Member => self.get_lang("MEMBER"),
            LangKey::Score => self.get_lang("SCORE"),
            LangKey::Value => self.get_lang("VALUE"),
            LangKey::Add => self.get_lang("ADD"),
            LangKey::LoadingKeyData => self.get_lang("LOADING_KEY_DATA"),
            LangKey::CommandHistory => self.get_lang("COMMAND_HISTORY"),
        }
    }

    fn get_lang(&self, key: &str) -> String {
        self.locale
            .get(key)
            .cloned()
            .or_else(|| self.fallback_locale.get(key).cloned())
            .unwrap_or(key.into())
    }
}

fn parse_locale(data: &str) -> HashMap<String, String> {
    let mut locale = HashMap::new();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            locale.insert(key.to_string(), value.to_string());
        }
    }
    locale
}

fn fill_template(template: &str, params: &HashMap<&str, &str>) -> String {
    let mut result = template.to_owned();
    for (&key, &value) in params {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn find_duplicate_keys(data: &str) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut duplicates = HashSet::new();

        for line in data.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                if !seen.insert(key.to_string()) {
                    duplicates.insert(key.to_string());
                }
            }
        }
        let mut duplicates: Vec<String> = duplicates.into_iter().collect();
        duplicates.sort();
        duplicates
    }

    fn compare_keys(map1: &HashMap<String, String>, map2: &HashMap<String, String>) -> bool {
        if map1.len() != map2.len() {
            return false;
        }
        map1.keys().all(|key| map2.contains_key(key))
    }

    #[test]
    fn test_parse_locale() {
        assert_eq!(parse_locale(EN_LOCALE).len(), LangKey::COUNT);
        assert_eq!(parse_locale(ES_LOCALE).len(), LangKey::COUNT);
        assert_eq!(parse_locale(DE_LOCALE).len(), LangKey::COUNT);
    }

    #[test]
    fn test_no_duplicates() {
        assert!(find_duplicate_keys(EN_LOCALE).is_empty());
        assert!(find_duplicate_keys(ES_LOCALE).is_empty());
        assert!(find_duplicate_keys(DE_LOCALE).is_empty());
    }

    #[test]
    fn test_inconsistent_keys() {
        assert!(compare_keys(
            &parse_locale(EN_LOCALE),
            &parse_locale(ES_LOCALE)
        ));
        assert!(compare_keys(
            &parse_locale(EN_LOCALE),
            &parse_locale(DE_LOCALE)
        ));
    }
}
