use std::collections::{BTreeMap, HashMap};

use rkyv::{Archive, Deserialize, Serialize};

use crate::attribution::Provenance;

// -----------------------------------------------------------------------------
// Core model types
// -----------------------------------------------------------------------------

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct PersonId(pub u64);

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct EventId(pub u64);

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct FamilyId(pub u64);

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
)]
#[rkyv(derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash))]
pub struct PlaceId(pub u64);

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Archive, Serialize, Deserialize,
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

    /// Parsed approximation used for indexing.
    pub range: Option<DateRange>,

    pub provenance: Provenance,
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

/// ADB-specific “astrology positions” are modeled as optional extra data.
///
/// This keeps the core genealogy event type usable for non-astrology sources,
/// while still allowing lossless ADB import.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct AstroPositions {
    pub sun_sign: Option<String>,
    pub sun_degmin: Option<String>,
    pub moon_sign: Option<String>,
    pub moon_degmin: Option<String>,
    pub asc_sign: Option<String>,
    pub asc_degmin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub kind: EventKind,

    pub date: Option<DateValue>,

    /// Source-specific time string (may be "local" time).
    pub time: Option<String>,

    /// Source-specific time zone string.
    pub time_zone: Option<String>,

    pub positions: Option<AstroPositions>,

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
