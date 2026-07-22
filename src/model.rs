use std::collections::{BTreeMap, HashMap};

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::Provenance;
use crate::genealogy_event::{GenealogyEvent, GenealogyEventKind};

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
    serde::Serialize,
    serde::Deserialize,
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
    serde::Serialize,
    serde::Deserialize,
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
    serde::Serialize,
    serde::Deserialize,
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
    serde::Serialize,
    serde::Deserialize,
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
    serde::Serialize,
    serde::Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct NoteId(pub u64);

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
pub enum Sex {
    Male,
    Female,
    Other,
    Unknown,
}

/// Precision carried by source date/time values.
///
/// Keeping precision explicit prevents lossy conversions such as treating a
/// year-only value as January 1st of that year.
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
    Millennium,
    Century,
    Decade,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

impl DatePrecision {
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

    /// Optional source timestamp string, when a historical date was parsed from a
    /// source format that carries a machine-readable timestamp.
    pub source_time: Option<String>,
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
            Self::MissingSign => write!(f, "time value must begin with '+' or '-'"),
            Self::InvalidYear => write!(f, "invalid year in time value"),
            Self::InvalidComponent(name) => write!(f, "invalid {name} in time value"),
            Self::UnsupportedPrecision(value) => {
                write!(f, "unsupported time precision {value}")
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
            source_time: None,
        }
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

        DateRange::from_years(Some(earliest_year), Some(latest_year))
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
pub struct Name {
    pub display: String,
    pub given: Option<String>,
    pub surname: Option<String>,

    /// Alternate spellings, maiden names, etc.
    pub aliases: Vec<String>,

    /// Source/provenance for this name assertion.
    pub provenance: Provenance,
}

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
    Default,
)]
pub struct DateRange {
    pub earliest_year: Option<i32>,
    pub latest_year: Option<i32>,

    /// Parsed lower bound when the source provided enough structure to retain
    /// precision. This may be open-ended (`None`) for values like `BEF 1900`.
    pub start: Option<HistoricalDate>,

    /// Parsed upper bound when the source provided enough structure to retain
    /// precision. This may be open-ended (`None`) for values like `AFT 1900`.
    pub end: Option<HistoricalDate>,
}

impl DateRange {
    pub fn from_years(earliest_year: Option<i32>, latest_year: Option<i32>) -> Self {
        Self {
            earliest_year,
            latest_year,
            start: earliest_year.map(|year| {
                HistoricalDate::new(year, DatePrecision::Year, CalendarModel::Gregorian)
            }),
            end: latest_year.map(|year| {
                HistoricalDate::new(year, DatePrecision::Year, CalendarModel::Gregorian)
            }),
        }
    }

    pub fn from_bounds(start: Option<HistoricalDate>, end: Option<HistoricalDate>) -> Self {
        let start_range = start.as_ref().map(HistoricalDate::year_range);
        let end_range = end.as_ref().map(HistoricalDate::year_range);
        Self {
            earliest_year: start_range.as_ref().and_then(|range| range.earliest_year),
            latest_year: end_range.as_ref().and_then(|range| range.latest_year),
            start,
            end,
        }
    }
}

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
pub struct DateValue {
    pub original: String,

    /// Precision-aware parsed historical date/time, when available.
    ///
    /// Importers should prefer this over coercing partial dates into concrete
    /// calendar days. For example, a source value with year precision should
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
        let range = parse_year_from_genealogy_date(&original)
            .map(|year| DateRange::from_years(Some(year), Some(year)));

        Self {
            original,
            historical: None,
            range,
            provenance,
        }
    }

    pub fn from_historical(historical: HistoricalDate, provenance: Provenance) -> Self {
        let original = historical
            .source_time
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

    pub fn display(&self) -> String {
        self.historical
            .as_ref()
            .map(HistoricalDate::display)
            .unwrap_or_else(|| self.original.clone())
    }
}

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
pub struct Family {
    pub id: FamilyId,
    pub spouses: Vec<PersonId>,
    pub children: Vec<PersonId>,
    pub events: Vec<EventId>,

    pub provenance: Provenance,
}

#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct Place {
    pub id: PlaceId,
    pub name: String,

    pub lat_lon: Option<(f64, f64)>,

    /// Optional pointer into a separate geocoding index.
    pub geosuggest_id: Option<u64>,

    pub provenance: Provenance,
}

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
pub struct Note {
    pub id: NoteId,
    pub text: String,
    pub copyright: Option<String>,

    pub provenance: Provenance,
}

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
    pub events: Vec<GenealogyEvent>,
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
    pub events: Vec<GenealogyEvent>,
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
    /// Rebuild a runtime index from an archived snapshot.
    #[must_use]
    pub fn from_archive(archive: GenealogyArchive) -> Self {
        Self::build(
            archive.people,
            archive.events,
            archive.families,
            archive.places,
            archive.notes,
        )
    }

    /// Build derived indexes. Assumes IDs are unique.
    pub fn build(
        people: Vec<Person>,
        events: Vec<GenealogyEvent>,
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
                GenealogyEventKind::Birth => event_index.insert("birth", idx_u32),
                GenealogyEventKind::Death => event_index.insert("death", idx_u32),
                GenealogyEventKind::Marriage => event_index.insert("marriage", idx_u32),
                GenealogyEventKind::Baptism => event_index.insert("baptism", idx_u32),
                GenealogyEventKind::Burial => event_index.insert("burial", idx_u32),
                GenealogyEventKind::Residence => event_index.insert("residence", idx_u32),
                GenealogyEventKind::Occupation => event_index.insert("occupation", idx_u32),
                GenealogyEventKind::Other(s) => {
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
