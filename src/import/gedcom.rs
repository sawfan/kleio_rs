//! GEDCOM import/export helpers.
//!
//! This module uses `ged_io` for the actual GEDCOM parser/writer and maps the
//! parsed records into Kleio's normalized people/events/families model.

use std::collections::HashMap;

use ged_io::{GedcomBuilder, GedcomWriter};

use crate::attribution::{Attribute, Provenance, SourceRef, Tag};
use crate::model::{
    DateRange, DateValue, Event, EventId, EventKind, Family, FamilyId, GenealogyIndex, Name,
    Note as KleioNote, NoteId, Person, PersonId, Place, PlaceId, Sex,
};

/// Parse GEDCOM text with `ged_io`.
///
/// Reference validation is enabled by default so broken pointers are surfaced
/// early. For incompatible real-world files, use [`parse_gedcom_text_lenient`]
/// or sanitize the source first.
pub fn parse_gedcom_text(content: &str) -> Result<ged_io::types::GedcomData, ged_io::GedcomError> {
    GedcomBuilder::new()
        .strict_mode(false)
        .validate_references(true)
        .build_from_str(content)
}

/// Parse GEDCOM text with reference validation disabled.
///
/// This is useful for files exported by tools that leave dangling source/media
/// references or include partial trees.
pub fn parse_gedcom_text_lenient(
    content: &str,
) -> Result<ged_io::types::GedcomData, ged_io::GedcomError> {
    GedcomBuilder::new()
        .strict_mode(false)
        .validate_references(false)
        .ignore_unknown_tags(false)
        .build_from_str(content)
}

/// Write `ged_io` GEDCOM data back to text.
pub fn export_gedcom_text(data: &ged_io::types::GedcomData) -> Result<String, ged_io::GedcomError> {
    GedcomWriter::new()
        .write_to_string(data)
        .map_err(Into::into)
}

/// Import summary suitable for UI feedback and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GedcomImportSummary {
    pub source_individuals: usize,
    pub source_families: usize,
    pub source_attributes: usize,
    pub people: usize,
    pub families: usize,
    pub events: usize,
    pub places: usize,
    pub notes: usize,
    pub events_with_dates: usize,
    pub earliest_year: Option<i32>,
    pub latest_year: Option<i32>,
}

impl GedcomImportSummary {
    #[must_use]
    pub fn from_source_and_index(
        source: &ged_io::types::GedcomData,
        index: &GenealogyIndex,
    ) -> Self {
        let mut events_with_dates = 0;
        let mut earliest_year = None::<i32>;
        let mut latest_year = None::<i32>;
        for event in &index.events {
            let Some(range) = event.date.as_ref().and_then(|date| date.range.as_ref()) else {
                continue;
            };
            events_with_dates += 1;
            if let Some(year) = range.earliest_year {
                earliest_year = Some(earliest_year.map_or(year, |current| current.min(year)));
            }
            if let Some(year) = range.latest_year {
                latest_year = Some(latest_year.map_or(year, |current| current.max(year)));
            }
        }

        Self {
            source_individuals: source.individuals.len(),
            source_families: source.families.len(),
            source_attributes: source
                .individuals
                .iter()
                .map(|indi| indi.attributes.len())
                .sum(),
            people: index.people.len(),
            families: index.families.len(),
            events: index.events.len(),
            places: index.places.len(),
            notes: index.notes.len(),
            events_with_dates,
            earliest_year,
            latest_year,
        }
    }
}

/// Result of mapping parsed GEDCOM data into Kleio records.
#[derive(Debug, Clone, PartialEq)]
pub struct GedcomImportResult {
    pub index: GenealogyIndex,
    pub summary: GedcomImportSummary,
}

/// Parse GEDCOM text and import it into a Kleio [`GenealogyIndex`].
pub fn import_gedcom_text(content: &str) -> Result<GedcomImportResult, ged_io::GedcomError> {
    let data = parse_gedcom_text(content)?;
    Ok(import_gedcom_data(&data))
}

/// Parse GEDCOM text leniently and import it into a Kleio [`GenealogyIndex`].
pub fn import_gedcom_text_lenient(
    content: &str,
) -> Result<GedcomImportResult, ged_io::GedcomError> {
    let data = parse_gedcom_text_lenient(content)?;
    Ok(import_gedcom_data(&data))
}

/// Map parsed `ged_io` data into Kleio's normalized records.
#[must_use]
pub fn import_gedcom_data(data: &ged_io::types::GedcomData) -> GedcomImportResult {
    let mut importer = GedcomImporter::new(data);
    let index = importer.import();
    let summary = GedcomImportSummary::from_source_and_index(data, &index);
    GedcomImportResult { index, summary }
}

struct GedcomImporter<'a> {
    data: &'a ged_io::types::GedcomData,
    person_ids: HashMap<String, PersonId>,
    family_ids: HashMap<String, FamilyId>,
    place_ids: HashMap<String, PlaceId>,
    next_event_id: u64,
    next_place_id: u64,
    next_note_id: u64,
    people: Vec<Person>,
    events: Vec<Event>,
    families: Vec<Family>,
    places: Vec<Place>,
    notes: Vec<KleioNote>,
}

impl<'a> GedcomImporter<'a> {
    fn new(data: &'a ged_io::types::GedcomData) -> Self {
        Self {
            data,
            person_ids: HashMap::new(),
            family_ids: HashMap::new(),
            place_ids: HashMap::new(),
            next_event_id: 1,
            next_place_id: 1,
            next_note_id: 1,
            people: Vec::new(),
            events: Vec::new(),
            families: Vec::new(),
            places: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn import(&mut self) -> GenealogyIndex {
        self.allocate_ids();
        self.import_people();
        self.import_families();

        GenealogyIndex::build(
            std::mem::take(&mut self.people),
            std::mem::take(&mut self.events),
            std::mem::take(&mut self.families),
            std::mem::take(&mut self.places),
            std::mem::take(&mut self.notes),
        )
    }

    fn allocate_ids(&mut self) {
        for (idx, individual) in self.data.individuals.iter().enumerate() {
            let fallback = format!("__INDI_{idx}");
            let xref = individual.xref.as_deref().unwrap_or(&fallback);
            self.person_ids
                .insert(xref.to_string(), PersonId(idx as u64 + 1));
        }

        for (idx, family) in self.data.families.iter().enumerate() {
            let fallback = format!("__FAM_{idx}");
            let xref = family.xref.as_deref().unwrap_or(&fallback);
            self.family_ids
                .insert(xref.to_string(), FamilyId(idx as u64 + 1));
        }
    }

    fn import_people(&mut self) {
        for (idx, individual) in self.data.individuals.iter().enumerate() {
            let fallback = format!("__INDI_{idx}");
            let xref = individual.xref.as_deref().unwrap_or(&fallback);
            let id = self.person_ids[xref];
            let provenance = provenance_for("gedcom:individual", xref);
            let mut person = Person {
                id,
                names: individual_name(individual, &provenance),
                sex: individual.sex.as_ref().map(|sex| map_sex(&sex.value)),
                events: Vec::new(),
                families_as_child: Vec::new(),
                families_as_spouse: Vec::new(),
                notes: Vec::new(),
                source_record: Some(SourceRef(format!("gedcom:{xref}"))),
                provenance,
            };

            for family_link in &individual.families {
                if let Some(&family_id) = self.family_ids.get(&family_link.xref) {
                    match family_link.family_link_type {
                        ged_io::types::individual::family_link::FamilyLinkType::Child => {
                            push_unique(&mut person.families_as_child, family_id);
                        }
                        ged_io::types::individual::family_link::FamilyLinkType::Spouse => {
                            push_unique(&mut person.families_as_spouse, family_id);
                        }
                    }
                }
            }

            if let Some(note_id) =
                self.import_note(individual.note.as_ref(), "gedcom:individual_note", xref)
            {
                person.notes.push(note_id);
            }

            for detail in &individual.events {
                let event_id = self.import_event_detail(detail, &[id]);
                person.events.push(event_id);
            }

            for attribute in &individual.attributes {
                let event_id = self.import_attribute_detail(attribute, id);
                person.events.push(event_id);
            }

            self.people.push(person);
        }
    }

    fn import_families(&mut self) {
        for (idx, ged_family) in self.data.families.iter().enumerate() {
            let fallback = format!("__FAM_{idx}");
            let xref = ged_family.xref.as_deref().unwrap_or(&fallback);
            let id = self.family_ids[xref];

            let mut spouses = Vec::new();
            if let Some(person_id) = ged_family
                .individual1
                .as_ref()
                .and_then(|xref| self.person_ids.get(xref))
                .copied()
            {
                spouses.push(person_id);
                self.add_family_to_person(person_id, id, FamilyRole::Spouse);
            }
            if let Some(person_id) = ged_family
                .individual2
                .as_ref()
                .and_then(|xref| self.person_ids.get(xref))
                .copied()
            {
                push_unique(&mut spouses, person_id);
                self.add_family_to_person(person_id, id, FamilyRole::Spouse);
            }

            let mut children = Vec::new();
            for child_xref in &ged_family.children {
                if let Some(&person_id) = self.person_ids.get(child_xref) {
                    push_unique(&mut children, person_id);
                    self.add_family_to_person(person_id, id, FamilyRole::Child);
                }
            }

            let mut event_ids = Vec::new();
            for detail in &ged_family.events {
                let event_id = self.import_event_detail(detail, &spouses);
                event_ids.push(event_id);
            }

            self.families.push(Family {
                id,
                spouses,
                children,
                events: event_ids,
                provenance: provenance_for("gedcom:family", xref),
            });
        }
    }

    fn add_family_to_person(&mut self, person_id: PersonId, family_id: FamilyId, role: FamilyRole) {
        let Some(person) = self.people.iter_mut().find(|p| p.id == person_id) else {
            return;
        };

        match role {
            FamilyRole::Child => push_unique(&mut person.families_as_child, family_id),
            FamilyRole::Spouse => push_unique(&mut person.families_as_spouse, family_id),
        }
    }

    fn import_event_detail(
        &mut self,
        detail: &ged_io::types::event::detail::Detail,
        participants: &[PersonId],
    ) -> EventId {
        let event_id = EventId(self.next_event_id);
        self.next_event_id += 1;

        let place = detail
            .place
            .as_ref()
            .and_then(|place| self.place_for(place));
        let date = detail
            .date
            .as_ref()
            .and_then(|date| date.value_without_calendar().or_else(|| date.value.clone()))
            .map(|value| {
                date_value_from_gedcom_date(value, provenance_for("gedcom:event_date", "DATE"))
            });

        let note_id = self.import_note(
            detail.note.as_ref(),
            "gedcom:event_note",
            &detail.event.to_string(),
        );
        let mut event = Event {
            id: event_id,
            kind: map_event_kind(&detail.event, detail.event_type.as_deref()),
            date,
            time: detail.date.as_ref().and_then(|date| date.time.clone()),
            time_zone: None,
            place,
            description: detail
                .event_type
                .clone()
                .or_else(|| detail.value.clone())
                .or_else(|| detail.cause.clone()),
            participants: participants.to_vec(),
            provenance: provenance_for("gedcom:event", &detail.event.to_string()),
        };
        if let Some(note_id) = note_id {
            event.provenance.attributes.push(Attribute {
                key: "gedcom.note_id".to_string(),
                value: note_id.0.to_string(),
            });
        }
        self.events.push(event);

        event_id
    }

    fn import_attribute_detail(
        &mut self,
        attribute: &ged_io::types::individual::attribute::detail::AttributeDetail,
        participant: PersonId,
    ) -> EventId {
        let event_id = EventId(self.next_event_id);
        self.next_event_id += 1;

        let place = attribute
            .place
            .as_ref()
            .and_then(|place| self.place_for(place));
        let date = attribute
            .date
            .as_ref()
            .and_then(|date| date.value_without_calendar().or_else(|| date.value.clone()))
            .map(|value| {
                date_value_from_gedcom_date(value, provenance_for("gedcom:attribute_date", "DATE"))
            });
        let note_id = self.import_note(
            attribute.note.as_ref(),
            "gedcom:attribute_note",
            &attribute.attribute.to_string(),
        );

        let mut provenance = provenance_for("gedcom:attribute", &attribute.attribute.to_string());
        if let Some(attribute_type) = attribute
            .attribute_type
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            provenance.attributes.push(Attribute {
                key: "gedcom.attribute_type".to_string(),
                value: attribute_type.to_string(),
            });
        }
        if let Some(note_id) = note_id {
            provenance.attributes.push(Attribute {
                key: "gedcom.note_id".to_string(),
                value: note_id.0.to_string(),
            });
        }

        self.events.push(Event {
            id: event_id,
            kind: map_attribute_kind(&attribute.attribute),
            date,
            time: attribute.date.as_ref().and_then(|date| date.time.clone()),
            time_zone: None,
            place,
            description: attribute
                .attribute_type
                .clone()
                .or_else(|| attribute.value.clone())
                .or_else(|| attribute.cause.clone()),
            participants: vec![participant],
            provenance,
        });

        event_id
    }

    fn import_note(
        &mut self,
        note: Option<&ged_io::types::note::Note>,
        kind: &str,
        source_value: &str,
    ) -> Option<NoteId> {
        let text = note?.value.as_deref()?.trim();
        if text.is_empty() {
            return None;
        }

        let id = NoteId(self.next_note_id);
        self.next_note_id += 1;
        self.notes.push(KleioNote {
            id,
            text: text.to_string(),
            copyright: None,
            provenance: provenance_for(kind, source_value),
        });
        Some(id)
    }

    fn place_for(&mut self, place: &ged_io::types::place::Place) -> Option<PlaceId> {
        let name = place.value.as_deref()?.trim();
        if name.is_empty() {
            return None;
        }

        let lat_lon = place.map.as_ref().and_then(|coords| {
            let lat = coords.latitude_decimal()?;
            let lon = coords.longitude_decimal()?;
            Some((lat, lon))
        });
        let key = match lat_lon {
            Some((lat, lon)) => format!("{name}|{lat:.7}|{lon:.7}"),
            None => name.to_string(),
        };

        if let Some(&id) = self.place_ids.get(&key) {
            return Some(id);
        }

        let id = PlaceId(self.next_place_id);
        self.next_place_id += 1;
        self.place_ids.insert(key, id);
        self.places.push(Place {
            id,
            name: name.to_string(),
            lat_lon,
            geosuggest_id: None,
            provenance: provenance_for("gedcom:place", name),
        });

        Some(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FamilyRole {
    Child,
    Spouse,
}

fn individual_name(
    individual: &ged_io::types::individual::Individual,
    provenance: &Provenance,
) -> Vec<Name> {
    let Some(name) = individual.name.as_ref() else {
        return vec![Name {
            display: individual
                .xref
                .as_deref()
                .unwrap_or("Unnamed GEDCOM individual")
                .to_string(),
            given: None,
            surname: None,
            aliases: Vec::new(),
            provenance: provenance.clone(),
        }];
    };

    let display = individual
        .full_name()
        .or_else(|| name.value.as_ref().map(|value| clean_gedcom_name(value)))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Unnamed GEDCOM individual".to_string());

    let mut aliases = Vec::new();
    if let Some(nickname) = name
        .nickname
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        aliases.push(nickname.trim().to_string());
    }
    for variation in name.phonetic.iter().chain(name.romanized.iter()) {
        let value = variation.value.trim();
        if !value.is_empty() {
            aliases.push(clean_gedcom_name(value));
        }
    }

    vec![Name {
        display,
        given: name
            .given
            .clone()
            .or_else(|| individual.given_name().map(str::to_string)),
        surname: name
            .surname
            .clone()
            .or_else(|| individual.surname().map(str::to_string)),
        aliases,
        provenance: provenance.clone(),
    }]
}

fn clean_gedcom_name(value: &str) -> String {
    value
        .replace('/', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn map_sex(value: &ged_io::types::individual::gender::GenderType) -> Sex {
    match value {
        ged_io::types::individual::gender::GenderType::Male => Sex::Male,
        ged_io::types::individual::gender::GenderType::Female => Sex::Female,
        ged_io::types::individual::gender::GenderType::Nonbinary => Sex::Other,
        ged_io::types::individual::gender::GenderType::Unknown => Sex::Unknown,
    }
}

fn map_event_kind(event: &ged_io::types::event::Event, event_type: Option<&str>) -> EventKind {
    match event {
        ged_io::types::event::Event::Birth => EventKind::Birth,
        ged_io::types::event::Event::Death => EventKind::Death,
        ged_io::types::event::Event::Marriage => EventKind::Marriage,
        ged_io::types::event::Event::Baptism | ged_io::types::event::Event::AdultChristening => {
            EventKind::Baptism
        }
        ged_io::types::event::Event::Burial | ged_io::types::event::Event::Cremation => {
            EventKind::Burial
        }
        ged_io::types::event::Event::Residence => EventKind::Residence,
        ged_io::types::event::Event::Event => EventKind::Other(
            event_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("event")
                .to_string(),
        ),
        other => EventKind::Other(other.to_string()),
    }
}

fn date_value_from_gedcom_date(original: String, provenance: Provenance) -> DateValue {
    let range = parse_gedcom_date_range(&original);
    DateValue {
        original,
        historical: None,
        range,
        provenance,
    }
}

fn parse_gedcom_date_range(value: &str) -> Option<DateRange> {
    let normalized = normalize_gedcom_date(value);
    let years = extract_years(&normalized);
    if years.is_empty() {
        return None;
    }

    let earliest = years.iter().copied().min();
    let latest = years.iter().copied().max();
    Some(DateRange {
        earliest_year: earliest,
        latest_year: latest,
    })
}

fn normalize_gedcom_date(value: &str) -> String {
    value
        .replace("@#DGREGORIAN@", " ")
        .replace("@#DJULIAN@", " ")
        .replace("@#DHEBREW@", " ")
        .replace("@#DFRENCH R@", " ")
        .replace("(", " ")
        .replace(")", " ")
}

fn extract_years(value: &str) -> Vec<i32> {
    let mut years = Vec::new();
    let bytes = value.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        let sign = match bytes[idx] {
            b'-' => {
                idx += 1;
                -1
            }
            b'+' => {
                idx += 1;
                1
            }
            _ => 1,
        };

        if idx + 4 <= bytes.len() && bytes[idx..idx + 4].iter().all(u8::is_ascii_digit) {
            let before_ok = idx == 0 || !bytes[idx - 1].is_ascii_alphanumeric();
            let after_ok = idx + 4 == bytes.len() || !bytes[idx + 4].is_ascii_alphanumeric();
            if before_ok && after_ok {
                if let Ok(year) = value[idx..idx + 4].parse::<i32>() {
                    years.push(year * sign);
                    idx += 4;
                    continue;
                }
            }
        }

        idx += 1;
    }

    years
}

fn map_attribute_kind(
    attribute: &ged_io::types::individual::attribute::IndividualAttribute,
) -> EventKind {
    match attribute {
        ged_io::types::individual::attribute::IndividualAttribute::Occupation => {
            EventKind::Occupation
        }
        ged_io::types::individual::attribute::IndividualAttribute::ResidesAt => {
            EventKind::Residence
        }
        other => EventKind::Other(other.to_string()),
    }
}

fn provenance_for(kind: &str, value: &str) -> Provenance {
    Provenance {
        sources: vec![SourceRef(format!("gedcom:{value}"))],
        citations: Vec::new(),
        tags: vec![Tag(kind.to_string())],
        attributes: vec![Attribute {
            key: "gedcom.value".to_string(),
            value: value.to_string(),
        }],
    }
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, value: T) {
    if !items.contains(&value) {
        items.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EventKind;

    const SAMPLE: &str = "0 HEAD\n1 GEDC\n2 VERS 5.5.1\n0 @I1@ INDI\n1 NAME Ada /Lovelace/\n2 GIVN Ada\n2 SURN Lovelace\n1 SEX F\n1 BIRT\n2 DATE 10 DEC 1815\n2 PLAC London, England\n1 NOTE Analytical engine pioneer\n2 CONT Countess of Lovelace\n1 OCCU Mathematician\n2 DATE FROM 1842 TO 1843\n2 NOTE Worked on notes for Menabrea translation\n1 FAMS @F1@\n0 @I2@ INDI\n1 NAME William /King-Noel/\n1 SEX M\n1 FAMS @F1@\n0 @I3@ INDI\n1 NAME Byron /King-Noel/\n1 FAMC @F1@\n0 @F1@ FAM\n1 HUSB @I2@\n1 WIFE @I1@\n1 CHIL @I3@\n1 MARR\n2 DATE 8 JUL 1835\n2 PLAC St. James, England\n0 TRLR\n";

    #[test]
    fn parses_and_imports_basic_gedcom() {
        let result = import_gedcom_text(SAMPLE).expect("sample GEDCOM imports");

        assert_eq!(result.summary.people, 3);
        assert_eq!(result.summary.families, 1);
        assert_eq!(result.summary.events, 3);
        assert_eq!(result.summary.places, 2);
        assert_eq!(result.summary.source_attributes, 1);
        assert_eq!(result.summary.notes, 2);
        assert_eq!(result.summary.events_with_dates, 3);
        assert_eq!(result.summary.earliest_year, Some(1815));
        assert_eq!(result.summary.latest_year, Some(1843));

        let ada = result
            .index
            .people
            .iter()
            .find(|person| {
                person
                    .names
                    .iter()
                    .any(|name| name.display == "Ada Lovelace")
            })
            .expect("Ada imported");
        assert_eq!(ada.sex, Some(Sex::Female));
        assert_eq!(ada.events.len(), 2);
        assert_eq!(ada.notes.len(), 1);
        assert_eq!(ada.families_as_spouse, vec![FamilyId(1)]);

        let birth = result
            .index
            .events
            .iter()
            .find(|event| event.kind == EventKind::Birth)
            .expect("birth imported");
        assert_eq!(
            birth.date.as_ref().map(DateValue::display),
            Some("10 DEC 1815".to_string())
        );
        let occupation = result
            .index
            .events
            .iter()
            .find(|event| event.kind == EventKind::Occupation)
            .expect("occupation imported");
        assert_eq!(
            occupation.date.as_ref().map(DateValue::display),
            Some("FROM 1842 TO 1843".to_string())
        );

        assert!(
            result
                .index
                .notes
                .iter()
                .any(|note| note.text.contains("Analytical engine pioneer"))
        );
    }

    #[test]
    fn gedcom_date_ranges_parse_common_qualifiers() {
        assert_eq!(
            parse_gedcom_date_range("BET 1901 AND 1905"),
            Some(DateRange {
                earliest_year: Some(1901),
                latest_year: Some(1905)
            })
        );
        assert_eq!(
            parse_gedcom_date_range("FROM 1842 TO 1843"),
            Some(DateRange {
                earliest_year: Some(1842),
                latest_year: Some(1843)
            })
        );
        assert_eq!(
            parse_gedcom_date_range("ABT 10 DEC 1815"),
            Some(DateRange {
                earliest_year: Some(1815),
                latest_year: Some(1815)
            })
        );
    }

    #[test]
    fn gedcom_round_trip_uses_ged_io_writer() {
        let data = parse_gedcom_text(SAMPLE).expect("sample parses");
        let text = export_gedcom_text(&data).expect("sample writes");

        assert!(text.contains("0 @I1@ INDI"));
        assert!(text.contains("1 NAME Ada /Lovelace/"));
    }
}
