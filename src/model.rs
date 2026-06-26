use std::collections::{BTreeMap, HashMap};

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::Provenance;

// -----------------------------------------------------------------------------
// Core model types
// -----------------------------------------------------------------------------

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct PersonId(pub u64);

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct EventId(pub u64);

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct FamilyId(pub u64);

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct PlaceId(pub u64);

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct NoteId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
    Other,
    Unknown,
}

/// Precision carried by source date/time values.
///
/// Wikidata represents date/time precision separately from the timestamp string
/// itself. Keeping that precision in the core model prevents lossy conversions
/// such as treating a year-only value as January 1st of that year.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub enum DatePrecision {
    /// A broad millennium bucket, as used by Wikidata precision `6`.
    Millennium,
    /// A broad century bucket, as used by Wikidata precision `7`.
    Century,
    /// A decade bucket, as used by Wikidata precision `8`.
    Decade,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

impl DatePrecision {
    /// Convert a Wikidata time precision code into Kleio's precision enum.
    ///
    /// Wikidata precision values relevant to historical/person data are:
    /// `6 = millennium`, `7 = century`, `8 = decade`, `9 = year`,
    /// `10 = month`, `11 = day`, `12 = hour`, `13 = minute`, `14 = second`.
    pub fn from_wikidata_precision(value: i32) -> Option<Self> {
        match value {
            6 => Some(Self::Millennium),
            7 => Some(Self::Century),
            8 => Some(Self::Decade),
            9 => Some(Self::Year),
            10 => Some(Self::Month),
            11 => Some(Self::Day),
            12 => Some(Self::Hour),
            13 => Some(Self::Minute),
            14 => Some(Self::Second),
            _ => None,
        }
    }

    pub fn includes_month(self) -> bool {
        self >= Self::Month
    }

    pub fn includes_day(self) -> bool {
        self >= Self::Day
    }

    pub fn includes_hour(self) -> bool {
        self >= Self::Hour
    }

    pub fn includes_minute(self) -> bool {
        self >= Self::Minute
    }

    pub fn includes_second(self) -> bool {
        self >= Self::Second
    }
}

/// Calendar model attached to a historical date.
///
/// Wikidata commonly uses Gregorian (`Q1985727`) or Julian (`Q1985786`) calendar
/// models. Other model URIs are preserved losslessly.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum CalendarModel {
    #[default]
    Gregorian,
    Julian,
    Other(String),
}

impl CalendarModel {
    pub const WIKIDATA_GREGORIAN_URI: &'static str = "http://www.wikidata.org/entity/Q1985727";
    pub const WIKIDATA_JULIAN_URI: &'static str = "http://www.wikidata.org/entity/Q1985786";

    pub fn from_wikidata_calendar_model(value: &str) -> Self {
        match value.trim() {
            Self::WIKIDATA_GREGORIAN_URI
            | "https://www.wikidata.org/entity/Q1985727"
            | "Q1985727" => Self::Gregorian,
            Self::WIKIDATA_JULIAN_URI | "https://www.wikidata.org/entity/Q1985786" | "Q1985786" => {
                Self::Julian
            }
            other => Self::Other(other.to_string()),
        }
    }

    pub fn as_wikidata_calendar_model(&self) -> Option<&str> {
        match self {
            Self::Gregorian => Some(Self::WIKIDATA_GREGORIAN_URI),
            Self::Julian => Some(Self::WIKIDATA_JULIAN_URI),
            Self::Other(value) => Some(value.as_str()),
        }
    }
}

/// A historical date/time with explicit precision and calendar.
///
/// This is intentionally not a `chrono`/`jiff` concrete date. A value like
/// "1983" or "1800s" is a precise assertion at year/century precision, not a
/// full day that should be coerced to `1983-01-01` or `1800-01-01`.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct HistoricalDate {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub precision: DatePrecision,
    pub calendar: CalendarModel,

    /// Original Wikidata time string, when this value came from Wikidata.
    ///
    /// Non-Wikidata importers can leave this as `None` and use
    /// `DateValue::original` for their source text.
    pub raw_wikidata_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoricalDateParseError {
    EmptyTime,
    MissingSign,
    InvalidYear,
    InvalidComponent(&'static str),
    UnsupportedPrecision(i32),
}

impl std::fmt::Display for HistoricalDateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTime => write!(f, "empty time value"),
            Self::MissingSign => write!(f, "Wikidata time value must begin with '+' or '-'"),
            Self::InvalidYear => write!(f, "invalid year in time value"),
            Self::InvalidComponent(name) => write!(f, "invalid {name} in time value"),
            Self::UnsupportedPrecision(value) => {
                write!(f, "unsupported Wikidata precision {value}")
            }
        }
    }
}

impl std::error::Error for HistoricalDateParseError {}

impl HistoricalDate {
    pub fn new(year: i32, precision: DatePrecision, calendar: CalendarModel) -> Self {
        Self {
            year,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            precision,
            calendar,
            raw_wikidata_time: None,
        }
    }

    /// Build a `HistoricalDate` from a Wikidata time triple.
    ///
    /// `time` is the raw Wikidata time string (for example
    /// `+1983-10-29T00:00:00Z`), `precision` is the numeric Wikidata precision,
    /// and `calendar_model` is the Wikidata calendar URI/entity ID.
    ///
    /// Components more detailed than the declared precision are deliberately
    /// discarded, so `+1983-01-01T00:00:00Z` with precision `9` remains a
    /// year-only `1983` value.
    pub fn from_wikidata_time(
        time: &str,
        precision: i32,
        calendar_model: &str,
    ) -> Result<Self, HistoricalDateParseError> {
        let precision = DatePrecision::from_wikidata_precision(precision)
            .ok_or(HistoricalDateParseError::UnsupportedPrecision(precision))?;
        let raw = time.trim();
        if raw.is_empty() {
            return Err(HistoricalDateParseError::EmptyTime);
        }

        let (sign, rest) = raw.split_at(1);
        let sign = match sign {
            "+" => 1,
            "-" => -1,
            _ => return Err(HistoricalDateParseError::MissingSign),
        };

        let (date_part, time_part) = rest.split_once('T').unwrap_or((rest, ""));
        let mut date_fields = date_part.split('-');
        let year_abs: i32 = date_fields
            .next()
            .filter(|s| !s.is_empty())
            .ok_or(HistoricalDateParseError::InvalidYear)?
            .parse()
            .map_err(|_| HistoricalDateParseError::InvalidYear)?;
        let year = year_abs * sign;

        let parsed_month = date_fields
            .next()
            .map(|s| parse_u8_component(s, "month", 1, 12))
            .transpose()?;
        let parsed_day = date_fields
            .next()
            .map(|s| parse_u8_component(s, "day", 1, 31))
            .transpose()?;

        let mut time_fields = time_part.trim_end_matches('Z').split(':');
        let parsed_hour = time_fields
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| parse_u8_component(s, "hour", 0, 23))
            .transpose()?;
        let parsed_minute = time_fields
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| parse_u8_component(s, "minute", 0, 59))
            .transpose()?;
        let parsed_second = time_fields
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| parse_u8_component(s, "second", 0, 59))
            .transpose()?;

        Ok(Self {
            year,
            month: precision.includes_month().then_some(parsed_month).flatten(),
            day: precision.includes_day().then_some(parsed_day).flatten(),
            hour: precision.includes_hour().then_some(parsed_hour).flatten(),
            minute: precision
                .includes_minute()
                .then_some(parsed_minute)
                .flatten(),
            second: precision
                .includes_second()
                .then_some(parsed_second)
                .flatten(),
            precision,
            calendar: CalendarModel::from_wikidata_calendar_model(calendar_model),
            raw_wikidata_time: Some(raw.to_string()),
        })
    }

    /// Year range used for coarse indexing/searching.
    ///
    /// Century/decade/millennium precision is interpreted as a bucket starting
    /// at the stored year (`1800s` => `1800..=1899`). This matches the common
    /// person/event search use case and avoids inventing a specific date.
    pub fn year_range(&self) -> DateRange {
        let (earliest_year, latest_year) = match self.precision {
            DatePrecision::Millennium => bucket_year_range(self.year, 1000),
            DatePrecision::Century => bucket_year_range(self.year, 100),
            DatePrecision::Decade => bucket_year_range(self.year, 10),
            DatePrecision::Year
            | DatePrecision::Month
            | DatePrecision::Day
            | DatePrecision::Hour
            | DatePrecision::Minute
            | DatePrecision::Second => (self.year, self.year),
        };

        DateRange {
            earliest_year: Some(earliest_year),
            latest_year: Some(latest_year),
        }
    }

    /// Precision-aware display that never fills missing components with
    /// January/1st/midnight placeholders.
    pub fn display(&self) -> String {
        match self.precision {
            DatePrecision::Millennium => format!("{}s", format_year(bucket_start(self.year, 1000))),
            DatePrecision::Century => format!("{}s", format_year(bucket_start(self.year, 100))),
            DatePrecision::Decade => format!("{}s", format_year(bucket_start(self.year, 10))),
            DatePrecision::Year => format_year(self.year),
            DatePrecision::Month => format!(
                "{}-{:02}",
                format_year(self.year),
                self.month.unwrap_or_default()
            ),
            DatePrecision::Day => format!(
                "{}-{:02}-{:02}",
                format_year(self.year),
                self.month.unwrap_or_default(),
                self.day.unwrap_or_default()
            ),
            DatePrecision::Hour => format!(
                "{}-{:02}-{:02} {:02}",
                format_year(self.year),
                self.month.unwrap_or_default(),
                self.day.unwrap_or_default(),
                self.hour.unwrap_or_default()
            ),
            DatePrecision::Minute => format!(
                "{}-{:02}-{:02} {:02}:{:02}",
                format_year(self.year),
                self.month.unwrap_or_default(),
                self.day.unwrap_or_default(),
                self.hour.unwrap_or_default(),
                self.minute.unwrap_or_default()
            ),
            DatePrecision::Second => format!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                format_year(self.year),
                self.month.unwrap_or_default(),
                self.day.unwrap_or_default(),
                self.hour.unwrap_or_default(),
                self.minute.unwrap_or_default(),
                self.second.unwrap_or_default()
            ),
        }
    }
}

fn parse_u8_component(
    value: &str,
    name: &'static str,
    min: u8,
    max: u8,
) -> Result<u8, HistoricalDateParseError> {
    let parsed: u8 = value
        .parse()
        .map_err(|_| HistoricalDateParseError::InvalidComponent(name))?;
    if parsed < min || parsed > max {
        return Err(HistoricalDateParseError::InvalidComponent(name));
    }
    Ok(parsed)
}

fn bucket_start(year: i32, bucket_size: i32) -> i32 {
    year.div_euclid(bucket_size) * bucket_size
}

fn bucket_year_range(year: i32, bucket_size: i32) -> (i32, i32) {
    let start = bucket_start(year, bucket_size);
    (start, start + bucket_size - 1)
}

fn format_year(year: i32) -> String {
    if (-9999..0).contains(&year) {
        format!("-{:04}", year.abs())
    } else if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        year.to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Name {
    pub display: String,
    pub given: Option<String>,
    pub surname: Option<String>,

    /// Alternate spellings, maiden names, etc.
    pub aliases: Vec<String>,

    /// Source/provenance for this name assertion.
    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize, Default)]
pub struct DateRange {
    pub earliest_year: Option<i32>,
    pub latest_year: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct DateValue {
    pub original: String,

    /// Precision-aware parsed historical date/time, when available.
    ///
    /// Importers should prefer this over coercing partial dates into concrete
    /// calendar days. For example, a Wikidata value with year precision should
    /// be represented as `HistoricalDate { precision: DatePrecision::Year, .. }`
    /// rather than as January 1st.
    pub historical: Option<HistoricalDate>,

    /// Parsed approximation used for indexing.
    pub range: Option<DateRange>,

    pub provenance: Provenance,
}

impl DateValue {
    pub fn from_original(original: impl Into<String>, provenance: Provenance) -> Self {
        let original = original.into();
        let range = parse_year_from_genealogy_date(&original).map(|year| DateRange {
            earliest_year: Some(year),
            latest_year: Some(year),
        });

        Self {
            original,
            historical: None,
            range,
            provenance,
        }
    }

    pub fn from_historical(historical: HistoricalDate, provenance: Provenance) -> Self {
        let original = historical
            .raw_wikidata_time
            .clone()
            .unwrap_or_else(|| historical.display());
        let range = Some(historical.year_range());

        Self {
            original,
            historical: Some(historical),
            range,
            provenance,
        }
    }

    pub fn from_wikidata_time(
        time: &str,
        precision: i32,
        calendar_model: &str,
        provenance: Provenance,
    ) -> Result<Self, HistoricalDateParseError> {
        let historical = HistoricalDate::from_wikidata_time(time, precision, calendar_model)?;
        Ok(Self::from_historical(historical, provenance))
    }

    pub fn display(&self) -> String {
        self.historical
            .as_ref()
            .map(HistoricalDate::display)
            .unwrap_or_else(|| self.original.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub enum EventKind {
    Birth,
    Death,
    Marriage,
    Baptism,
    Burial,
    Residence,
    Occupation,

    /// Fallback for source-specific kinds.
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: EventKind,

    pub date: Option<DateValue>,

    /// Source-specific time string (may be local time).
    pub time: Option<String>,

    /// Source-specific time zone string.
    pub time_zone: Option<String>,

    pub place: Option<PlaceId>,

    pub description: Option<String>,

    pub participants: Vec<PersonId>,

    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Family {
    pub id: FamilyId,
    pub spouses: Vec<PersonId>,
    pub children: Vec<PersonId>,
    pub events: Vec<EventId>,

    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
pub struct Place {
    pub id: PlaceId,
    pub name: String,

    pub lat_lon: Option<(f64, f64)>,

    /// Optional pointer into a separate geocoding index.
    pub geosuggest_id: Option<u64>,

    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Note {
    pub id: NoteId,
    pub text: String,
    pub copyright: Option<String>,

    pub provenance: Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub names: Vec<Name>,
    pub sex: Option<Sex>,

    pub events: Vec<EventId>,

    pub families_as_child: Vec<FamilyId>,
    pub families_as_spouse: Vec<FamilyId>,

    pub notes: Vec<NoteId>,

    /// Primary source record ID (if imported from a single upstream record).
    pub source_record: Option<crate::attribution::SourceRef>,

    pub provenance: Provenance,
}

// -----------------------------------------------------------------------------
// Derived/index types
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchIndex {
    pub postings: HashMap<String, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DateIndex {
    pub events_by_year: HashMap<i32, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Archive, Serialize, Deserialize)]
pub struct PersonRelations {
    pub person_id: PersonId,
    pub parents: Vec<PersonId>,
    pub spouses: Vec<PersonId>,
    pub children: Vec<PersonId>,
    pub siblings: Vec<PersonId>,
}

/// Runtime index with derived structures.
#[derive(Debug, Clone, PartialEq)]
pub struct GenealogyIndex {
    pub people: Vec<Person>,
    pub events: Vec<Event>,
    pub families: Vec<Family>,
    pub places: Vec<Place>,
    pub notes: Vec<Note>,

    pub person_by_id: HashMap<PersonId, usize>,
    pub event_by_id: HashMap<EventId, usize>,
    pub family_by_id: HashMap<FamilyId, usize>,
    pub place_by_id: HashMap<PlaceId, usize>,
    pub note_by_id: HashMap<NoteId, usize>,

    pub name_index: SearchIndex,
    pub place_index: SearchIndex,
    pub event_index: SearchIndex,
    pub date_index: DateIndex,

    pub relations: HashMap<PersonId, PersonRelations>,
}

// --- Rkyv-friendly archive types ---

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize, Default)]
pub struct SearchIndexArchive {
    pub postings: BTreeMap<String, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize, Default)]
pub struct DateIndexArchive {
    pub events_by_year: BTreeMap<i32, Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Archive, Serialize, Deserialize)]
pub struct GenealogyArchive {
    pub people: Vec<Person>,
    pub events: Vec<Event>,
    pub families: Vec<Family>,
    pub places: Vec<Place>,
    pub notes: Vec<Note>,

    pub person_by_id: BTreeMap<PersonId, u32>,
    pub event_by_id: BTreeMap<EventId, u32>,
    pub family_by_id: BTreeMap<FamilyId, u32>,
    pub place_by_id: BTreeMap<PlaceId, u32>,
    pub note_by_id: BTreeMap<NoteId, u32>,

    pub name_index: SearchIndexArchive,
    pub place_index: SearchIndexArchive,
    pub event_index: SearchIndexArchive,
    pub date_index: DateIndexArchive,

    pub relations: BTreeMap<PersonId, PersonRelations>,
}

impl SearchIndex {
    pub fn insert(&mut self, token: impl Into<String>, record_index: u32) {
        let token = token.into();
        let entry = self.postings.entry(token).or_default();
        if entry.last().copied() != Some(record_index) {
            entry.push(record_index);
        }
    }

    pub fn tokenize(s: &str) -> impl Iterator<Item = String> + '_ {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_ascii_lowercase())
    }
}

impl GenealogyIndex {
    /// Build derived indexes. Assumes IDs are unique.
    pub fn build(
        people: Vec<Person>,
        events: Vec<Event>,
        families: Vec<Family>,
        places: Vec<Place>,
        notes: Vec<Note>,
    ) -> Self {
        let person_by_id = people.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
        let event_by_id = events.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
        let family_by_id = families
            .iter()
            .enumerate()
            .map(|(i, f)| (f.id, i))
            .collect();
        let place_by_id = places.iter().enumerate().map(|(i, p)| (p.id, i)).collect();
        let note_by_id = notes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();

        let mut name_index = SearchIndex::default();
        for (idx, person) in people.iter().enumerate() {
            let idx = idx as u32;
            for name in &person.names {
                for token in SearchIndex::tokenize(&name.display) {
                    name_index.insert(token, idx);
                }
                if let Some(given) = name.given.as_deref() {
                    for token in SearchIndex::tokenize(given) {
                        name_index.insert(token, idx);
                    }
                }
                if let Some(surname) = name.surname.as_deref() {
                    for token in SearchIndex::tokenize(surname) {
                        name_index.insert(token, idx);
                    }
                }
                for alias in &name.aliases {
                    for token in SearchIndex::tokenize(alias) {
                        name_index.insert(token, idx);
                    }
                }
            }
        }

        let mut place_index = SearchIndex::default();
        for (idx, place) in places.iter().enumerate() {
            let idx = idx as u32;
            for token in SearchIndex::tokenize(&place.name) {
                place_index.insert(token, idx);
            }
        }

        let mut event_index = SearchIndex::default();
        let mut date_index = DateIndex::default();
        for (idx, event) in events.iter().enumerate() {
            let idx_u32 = idx as u32;

            match &event.kind {
                EventKind::Birth => event_index.insert("birth", idx_u32),
                EventKind::Death => event_index.insert("death", idx_u32),
                EventKind::Marriage => event_index.insert("marriage", idx_u32),
                EventKind::Baptism => event_index.insert("baptism", idx_u32),
                EventKind::Burial => event_index.insert("burial", idx_u32),
                EventKind::Residence => event_index.insert("residence", idx_u32),
                EventKind::Occupation => event_index.insert("occupation", idx_u32),
                EventKind::Other(s) => {
                    for token in SearchIndex::tokenize(s) {
                        event_index.insert(token, idx_u32);
                    }
                }
            }

            if let Some(desc) = event.description.as_deref() {
                for token in SearchIndex::tokenize(desc) {
                    event_index.insert(token, idx_u32);
                }
            }

            if let Some(date) = event.date.as_ref().and_then(|d| d.range.as_ref())
                && let Some(year) = date.earliest_year
            {
                date_index
                    .events_by_year
                    .entry(year)
                    .or_default()
                    .push(idx_u32);
            }
        }

        let mut relations: HashMap<PersonId, PersonRelations> = HashMap::new();
        for p in &people {
            relations.insert(
                p.id,
                PersonRelations {
                    person_id: p.id,
                    ..PersonRelations::default()
                },
            );
        }

        // Edges from families.
        for family in &families {
            for &spouse in &family.spouses {
                if let Some(rel) = relations.get_mut(&spouse) {
                    for &other in &family.spouses {
                        if other != spouse && !rel.spouses.contains(&other) {
                            rel.spouses.push(other);
                        }
                    }
                    for &child in &family.children {
                        if !rel.children.contains(&child) {
                            rel.children.push(child);
                        }
                    }
                }
            }

            for &child in &family.children {
                if let Some(rel) = relations.get_mut(&child) {
                    for &spouse in &family.spouses {
                        if !rel.parents.contains(&spouse) {
                            rel.parents.push(spouse);
                        }
                    }
                }
            }

            // Siblings: all children in same family.
            for &child in &family.children {
                if let Some(rel) = relations.get_mut(&child) {
                    for &sib in &family.children {
                        if sib != child && !rel.siblings.contains(&sib) {
                            rel.siblings.push(sib);
                        }
                    }
                }
            }
        }

        Self {
            people,
            events,
            families,
            places,
            notes,

            person_by_id,
            event_by_id,
            family_by_id,
            place_by_id,
            note_by_id,

            name_index,
            place_index,
            event_index,
            date_index,

            relations,
        }
    }

    /// Convert the runtime index (HashMap-backed) into an archivable form.
    pub fn to_archive(&self) -> GenealogyArchive {
        GenealogyArchive {
            people: self.people.clone(),
            events: self.events.clone(),
            families: self.families.clone(),
            places: self.places.clone(),
            notes: self.notes.clone(),

            person_by_id: self
                .person_by_id
                .iter()
                .map(|(&k, &v)| (k, v as u32))
                .collect(),
            event_by_id: self
                .event_by_id
                .iter()
                .map(|(&k, &v)| (k, v as u32))
                .collect(),
            family_by_id: self
                .family_by_id
                .iter()
                .map(|(&k, &v)| (k, v as u32))
                .collect(),
            place_by_id: self
                .place_by_id
                .iter()
                .map(|(&k, &v)| (k, v as u32))
                .collect(),
            note_by_id: self
                .note_by_id
                .iter()
                .map(|(&k, &v)| (k, v as u32))
                .collect(),

            name_index: SearchIndexArchive {
                postings: self.name_index.postings.clone().into_iter().collect(),
            },
            place_index: SearchIndexArchive {
                postings: self.place_index.postings.clone().into_iter().collect(),
            },
            event_index: SearchIndexArchive {
                postings: self.event_index.postings.clone().into_iter().collect(),
            },
            date_index: DateIndexArchive {
                events_by_year: self.date_index.events_by_year.clone().into_iter().collect(),
            },

            relations: self.relations.clone().into_iter().collect(),
        }
    }
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

/// Very small helper used by importers to build an approximate year index.
///
/// Accepts strings like "YYYY/MM/DD" or "YYYY-MM-DD" and returns the parsed year.
pub fn parse_year_from_genealogy_date(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let mut digits = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 4 {
                break;
            }
        } else if !digits.is_empty() {
            break;
        }
    }

    if digits.len() == 4 {
        digits.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wikidata_year_precision_does_not_invent_month_or_day() {
        let date = HistoricalDate::from_wikidata_time(
            "+1983-01-01T00:00:00Z",
            9,
            CalendarModel::WIKIDATA_GREGORIAN_URI,
        )
        .expect("valid Wikidata year-precision date");

        assert_eq!(date.year, 1983);
        assert_eq!(date.month, None);
        assert_eq!(date.day, None);
        assert_eq!(date.precision, DatePrecision::Year);
        assert_eq!(date.display(), "1983");
        assert_eq!(
            date.year_range(),
            DateRange {
                earliest_year: Some(1983),
                latest_year: Some(1983)
            }
        );
    }

    #[test]
    fn wikidata_day_precision_keeps_month_and_day() {
        let date = HistoricalDate::from_wikidata_time("+1983-10-29T00:00:00Z", 11, "Q1985786")
            .expect("valid Wikidata day-precision date");

        assert_eq!(date.year, 1983);
        assert_eq!(date.month, Some(10));
        assert_eq!(date.day, Some(29));
        assert_eq!(date.precision, DatePrecision::Day);
        assert_eq!(date.calendar, CalendarModel::Julian);
        assert_eq!(date.display(), "1983-10-29");
    }

    #[test]
    fn wikidata_century_precision_indexes_the_century_bucket() {
        let date = HistoricalDate::from_wikidata_time(
            "+1800-01-01T00:00:00Z",
            7,
            CalendarModel::WIKIDATA_GREGORIAN_URI,
        )
        .expect("valid Wikidata century-precision date");

        assert_eq!(date.precision, DatePrecision::Century);
        assert_eq!(date.month, None);
        assert_eq!(date.day, None);
        assert_eq!(date.display(), "1800s");
        assert_eq!(
            date.year_range(),
            DateRange {
                earliest_year: Some(1800),
                latest_year: Some(1899)
            }
        );
    }
}
