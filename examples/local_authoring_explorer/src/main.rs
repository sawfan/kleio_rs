#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // This example is intended to be built and served with Trunk for wasm32.
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    wasm_app::start()
}

#[cfg(target_arch = "wasm32")]
mod wasm_app {
    use std::cell::RefCell;
    use std::rc::Rc;

    use serde::Deserialize;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        Document, Element, Event, HtmlElement, HtmlInputElement, Request, RequestInit, RequestMode,
        Response,
    };

    const DATASETS: &[Dataset] = &[
        Dataset {
            key: DatasetKey::Semantic,
            label: "Semantic bundle",
            path: "build/kleio.compiled.json",
        },
        Dataset {
            key: DatasetKey::Ecs,
            label: "ECS bundle",
            path: "build/kleio.ecs.json",
        },
        Dataset {
            key: DatasetKey::Timeline,
            label: "Timeline projection",
            path: "build/example-life.timeline.json",
        },
        Dataset {
            key: DatasetKey::Tree,
            label: "Tree projection",
            path: "build/main-family-tree.tree.json",
        },
    ];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum DatasetKey {
        Semantic,
        Ecs,
        Timeline,
        Tree,
    }

    #[derive(Debug, Clone, Copy)]
    struct Dataset {
        key: DatasetKey,
        label: &'static str,
        path: &'static str,
    }

    #[derive(Debug, Deserialize)]
    struct SemanticBundle {
        markdown_records: Vec<MarkdownRecord>,
        toml_documents: Vec<TomlDocument>,
    }

    #[derive(Debug, Deserialize)]
    struct MarkdownRecord {
        path: String,
        id: String,
        kind: String,
        title: Option<String>,
        date: Option<String>,
        summary: Option<String>,
        tags: Vec<String>,
        related: Vec<String>,
        place: Option<String>,
        attributes: serde_json::Value,
        notes_markdown: String,
    }

    #[derive(Debug, Deserialize)]
    struct TomlDocument {
        path: String,
        id: Option<String>,
        kind: Option<String>,
        title: Option<String>,
        data: serde_json::Value,
    }

    #[derive(Debug, Deserialize)]
    struct EcsBundle {
        entities: Vec<EcsEntity>,
        resources: serde_json::Value,
    }

    #[derive(Debug, Deserialize)]
    struct EcsEntity {
        id: String,
        components: serde_json::Value,
    }

    #[derive(Debug, Deserialize)]
    struct TimelineProjection {
        world: String,
        view: Option<ViewSummary>,
        collections: Vec<TimelineCollection>,
        events: Vec<TimelineEvent>,
    }

    #[derive(Debug, Deserialize)]
    struct ViewSummary {
        id: String,
        title: Option<String>,
        path: String,
    }

    #[derive(Debug, Deserialize)]
    struct TimelineCollection {
        id: String,
        title: Option<String>,
        members: Vec<serde_json::Value>,
    }

    #[derive(Debug, Deserialize)]
    struct TimelineEvent {
        id: String,
        kind: String,
        title: Option<String>,
        time: Option<String>,
        path: String,
        notes_markdown: String,
    }

    #[derive(Debug, Deserialize)]
    struct TreeProjection {
        metadata: serde_json::Value,
        people: Vec<serde_json::Value>,
        events: Vec<serde_json::Value>,
        relationships: Vec<serde_json::Value>,
    }

    struct LoadedDataset {
        dataset: Dataset,
        value: serde_json::Value,
    }

    struct AppState {
        datasets: Vec<LoadedDataset>,
        selected: DatasetKey,
        filter: String,
    }

    pub fn start() -> Result<(), JsValue> {
        let state = Rc::new(RefCell::new(AppState {
            datasets: Vec::new(),
            selected: DatasetKey::Semantic,
            filter: String::new(),
        }));

        install_filter_handler(Rc::clone(&state))?;
        wasm_bindgen_futures::spawn_local(async move {
            match load_datasets().await {
                Ok(datasets) => {
                    {
                        let mut state = state.borrow_mut();
                        state.datasets = datasets;
                    }
                    set_status("Loaded generated Kleio JSON outputs.");
                    if let Err(err) = render(&state.borrow(), Rc::clone(&state)) {
                        set_status(&format!("Render failed: {err:?}"));
                    }
                }
                Err(err) => {
                    set_status(&format!("Failed to load generated JSON: {err:?}"));
                }
            }
        });

        Ok(())
    }

    async fn load_datasets() -> Result<Vec<LoadedDataset>, JsValue> {
        let mut datasets = Vec::new();
        for dataset in DATASETS {
            let value = fetch_json(dataset.path).await?;
            datasets.push(LoadedDataset {
                dataset: *dataset,
                value,
            });
        }
        Ok(datasets)
    }

    async fn fetch_json(path: &str) -> Result<serde_json::Value, JsValue> {
        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init(path, &opts)?;
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing window"))?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into()?;
        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "{} returned HTTP {}",
                path,
                resp.status()
            )));
        }

        let text = JsFuture::from(resp.text()?).await?;
        let text = text
            .as_string()
            .ok_or_else(|| JsValue::from_str("response text was not a string"))?;
        serde_json::from_str(&text).map_err(|err| JsValue::from_str(&err.to_string()))
    }

    fn install_filter_handler(state: Rc<RefCell<AppState>>) -> Result<(), JsValue> {
        let input = element("filter")?.dyn_into::<HtmlInputElement>()?;
        let closure = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            let Some(target) = event.target() else {
                return;
            };
            let Ok(input) = target.dyn_into::<HtmlInputElement>() else {
                return;
            };
            {
                let mut state = state.borrow_mut();
                state.filter = input.value();
            }
            if let Err(err) = render(&state.borrow(), Rc::clone(&state)) {
                set_status(&format!("Render failed: {err:?}"));
            }
        });
        input.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    }

    fn render(state: &AppState, state_ref: Rc<RefCell<AppState>>) -> Result<(), JsValue> {
        render_dataset_buttons(state, state_ref)?;
        let Some(dataset) = state
            .datasets
            .iter()
            .find(|dataset| dataset.dataset.key == state.selected)
        else {
            return Ok(());
        };

        match dataset.dataset.key {
            DatasetKey::Semantic => render_semantic(&dataset.value, &state.filter)?,
            DatasetKey::Ecs => render_ecs(&dataset.value, &state.filter)?,
            DatasetKey::Timeline => render_timeline(&dataset.value, &state.filter)?,
            DatasetKey::Tree => render_tree(&dataset.value, &state.filter)?,
        }

        Ok(())
    }

    fn render_dataset_buttons(
        state: &AppState,
        state_ref: Rc<RefCell<AppState>>,
    ) -> Result<(), JsValue> {
        let container = element("dataset-buttons")?;
        container.set_inner_html("");
        for dataset in &state.datasets {
            let button = document().create_element("button")?;
            button.set_text_content(Some(&format!(
                "{}\n{}",
                dataset.dataset.label, dataset.dataset.path
            )));
            button.set_attribute(
                "aria-pressed",
                if dataset.dataset.key == state.selected {
                    "true"
                } else {
                    "false"
                },
            )?;
            let selected = dataset.dataset.key;
            let state_ref = Rc::clone(&state_ref);
            let closure = Closure::<dyn FnMut(Event)>::new(move |_| {
                {
                    let mut state = state_ref.borrow_mut();
                    state.selected = selected;
                }
                if let Err(err) = render(&state_ref.borrow(), Rc::clone(&state_ref)) {
                    set_status(&format!("Render failed: {err:?}"));
                }
            });
            button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
            closure.forget();
            container.append_child(&button)?;
        }
        Ok(())
    }

    fn render_semantic(value: &serde_json::Value, filter: &str) -> Result<(), JsValue> {
        let bundle: SemanticBundle = serde_json::from_value(value.clone())
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        let visible_records = bundle
            .markdown_records
            .iter()
            .filter(|record| matches_filter(record_search_text(record), filter))
            .collect::<Vec<_>>();
        let visible_documents = bundle
            .toml_documents
            .iter()
            .filter(|document| matches_filter(document_search_text(document), filter))
            .collect::<Vec<_>>();

        set_html(
            "summary",
            &format!(
                r#"<div class="summary-grid">
  <div class="metric"><strong>{}</strong><span>Markdown records</span></div>
  <div class="metric"><strong>{}</strong><span>TOML documents</span></div>
  <div class="metric"><strong>{}</strong><span>Visible records</span></div>
</div>"#,
                bundle.markdown_records.len(),
                bundle.toml_documents.len(),
                visible_records.len() + visible_documents.len()
            ),
        )?;

        let mut records_html = String::new();
        records_html.push_str("<h2>Authored records</h2><div class=\"record-list\">");
        for record in &visible_records {
            records_html.push_str(&format!(
                r#"<article class="record-card"><h3>{}</h3><div class="meta"><code>{}</code> · {} · {}</div>{}</article>"#,
                esc(record.title.as_deref().unwrap_or(&record.id)),
                esc(&record.id),
                esc(&record.kind),
                esc(&record.path),
                pills(record.tags.iter().map(String::as_str))
            ));
        }
        for document in &visible_documents {
            records_html.push_str(&format!(
                r#"<article class="record-card"><h3>{}</h3><div class="meta"><code>{}</code> · {} · {}</div></article>"#,
                esc(document.title.as_deref().unwrap_or_else(|| document.id.as_deref().unwrap_or(&document.path))),
                esc(document.id.as_deref().unwrap_or("no id")),
                esc(document.kind.as_deref().unwrap_or("document")),
                esc(&document.path)
            ));
        }
        if visible_records.is_empty() && visible_documents.is_empty() {
            records_html
                .push_str("<div class=\"empty\">No authored records match the filter.</div>");
        }
        records_html.push_str("</div>");
        set_html("records", &records_html)?;
        set_html("details", &raw_json_section(value))
    }

    fn render_ecs(value: &serde_json::Value, filter: &str) -> Result<(), JsValue> {
        let bundle: EcsBundle = serde_json::from_value(value.clone())
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        let visible = bundle
            .entities
            .iter()
            .filter(|entity| matches_filter(format!("{} {}", entity.id, entity.components), filter))
            .collect::<Vec<_>>();
        let relationship_count = bundle
            .resources
            .get("Relationships")
            .and_then(|relationships| relationships.get("items"))
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);

        set_html(
            "summary",
            &format!(
                r#"<div class="summary-grid">
  <div class="metric"><strong>{}</strong><span>ECS entities</span></div>
  <div class="metric"><strong>{relationship_count}</strong><span>Relationships</span></div>
  <div class="metric"><strong>{}</strong><span>Visible entities</span></div>
</div>"#,
                bundle.entities.len(),
                visible.len()
            ),
        )?;

        let mut html = String::from("<h2>ECS entities</h2><div class=\"record-list\">");
        for entity in &visible {
            let components = entity
                .components
                .as_object()
                .map(|object| object.keys().map(String::as_str).collect::<Vec<_>>())
                .unwrap_or_default();
            html.push_str(&format!(
                r#"<article class="record-card"><h3><code>{}</code></h3>{}</article>"#,
                esc(&entity.id),
                pills(components.into_iter())
            ));
        }
        if visible.is_empty() {
            html.push_str("<div class=\"empty\">No ECS entities match the filter.</div>");
        }
        html.push_str("</div>");
        set_html("records", &html)?;
        set_html("details", &raw_json_section(value))
    }

    fn render_timeline(value: &serde_json::Value, filter: &str) -> Result<(), JsValue> {
        let timeline: TimelineProjection = serde_json::from_value(value.clone())
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        let visible = timeline
            .events
            .iter()
            .filter(|event| {
                matches_filter(
                    format!(
                        "{} {} {} {}",
                        event.id,
                        event.kind,
                        event.title.as_deref().unwrap_or(""),
                        event.notes_markdown
                    ),
                    filter,
                )
            })
            .collect::<Vec<_>>();
        let view_label = timeline
            .view
            .as_ref()
            .and_then(|view| view.title.as_deref())
            .unwrap_or("Timeline");
        let view_meta = timeline
            .view
            .as_ref()
            .map(|view| format!(" · <code>{}</code> · {}", esc(&view.id), esc(&view.path)))
            .unwrap_or_default();
        let collection_members = timeline
            .collections
            .iter()
            .map(|collection| collection.members.len())
            .sum::<usize>();

        set_html(
            "summary",
            &format!(
                r#"<div class="summary-grid">
  <div class="metric"><strong>{}</strong><span>Timeline events</span></div>
  <div class="metric"><strong>{}</strong><span>Collections</span></div>
  <div class="metric"><strong>{}</strong><span>Collection members</span></div>
  <div class="metric"><strong>{}</strong><span>Visible events</span></div>
</div><p class="meta">{}{} in <code>{}</code></p>"#,
                timeline.events.len(),
                timeline.collections.len(),
                collection_members,
                visible.len(),
                esc(view_label),
                view_meta,
                esc(&timeline.world)
            ),
        )?;

        let mut html = String::from("<h2>Timeline events</h2><div class=\"record-list\">");
        for event in &visible {
            html.push_str(&format!(
                r#"<article class="record-card"><h3>{}</h3><div class="meta"><code>{}</code> · {} · {} · {}</div></article>"#,
                esc(event.title.as_deref().unwrap_or(&event.id)),
                esc(&event.id),
                esc(&event.kind),
                esc(event.time.as_deref().unwrap_or("unspecified time")),
                esc(&event.path)
            ));
        }
        if visible.is_empty() {
            html.push_str("<div class=\"empty\">No timeline events match the filter.</div>");
        }
        html.push_str("</div>");
        if !timeline.collections.is_empty() {
            html.push_str("<h2>Timeline collections</h2><div class=\"record-list\">");
            for collection in &timeline.collections {
                html.push_str(&format!(
                    r#"<article class="record-card"><h3>{}</h3><div class="meta"><code>{}</code> · {} member(s)</div></article>"#,
                    esc(collection.title.as_deref().unwrap_or(&collection.id)),
                    esc(&collection.id),
                    collection.members.len()
                ));
            }
            html.push_str("</div>");
        }
        set_html("records", &html)?;
        set_html("details", &raw_json_section(value))
    }

    fn render_tree(value: &serde_json::Value, filter: &str) -> Result<(), JsValue> {
        let tree: TreeProjection = serde_json::from_value(value.clone())
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        let visible_people = tree
            .people
            .iter()
            .filter(|person| matches_filter(person.to_string(), filter))
            .collect::<Vec<_>>();

        set_html(
            "summary",
            &format!(
                r#"<div class="summary-grid">
  <div class="metric"><strong>{}</strong><span>People</span></div>
  <div class="metric"><strong>{}</strong><span>Events</span></div>
  <div class="metric"><strong>{}</strong><span>Relationships</span></div>
</div>"#,
                tree.people.len(),
                tree.events.len(),
                tree.relationships.len()
            ),
        )?;

        let mut html = String::from("<h2>Tree people</h2><div class=\"record-list\">");
        for person in &visible_people {
            let id = person
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown person");
            let name = person
                .get("display_name")
                .or_else(|| person.get("name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or(id);
            html.push_str(&format!(
                r#"<article class="record-card"><h3>{}</h3><div class="meta"><code>{}</code></div></article>"#,
                esc(name),
                esc(id)
            ));
        }
        if visible_people.is_empty() {
            html.push_str("<div class=\"empty\">No tree people match the filter.</div>");
        }
        html.push_str("</div>");
        set_html("records", &html)?;
        set_html(
            "details",
            &format!(
                r#"<section class="detail-section"><h3>Tree metadata</h3><pre><code>{}</code></pre></section>{}"#,
                esc(&pretty_json(&tree.metadata)),
                raw_json_section(value)
            ),
        )
    }

    fn record_search_text(record: &MarkdownRecord) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {} {}",
            record.id,
            record.kind,
            record.path,
            record.title.as_deref().unwrap_or(""),
            record.date.as_deref().unwrap_or(""),
            record.summary.as_deref().unwrap_or(""),
            record.related.join(" "),
            record.place.as_deref().unwrap_or(""),
            record.attributes,
            record.notes_markdown
        )
    }

    fn document_search_text(document: &TomlDocument) -> String {
        format!(
            "{} {} {} {} {}",
            document.path,
            document.id.as_deref().unwrap_or(""),
            document.kind.as_deref().unwrap_or(""),
            document.title.as_deref().unwrap_or(""),
            document.data
        )
    }

    fn matches_filter(text: impl AsRef<str>, filter: &str) -> bool {
        let filter = filter.trim().to_lowercase();
        filter.is_empty() || text.as_ref().to_lowercase().contains(&filter)
    }

    fn raw_json_section(value: &serde_json::Value) -> String {
        format!(
            r#"<section class="detail-section"><h3>Raw JSON</h3><pre><code>{}</code></pre></section>"#,
            esc(&pretty_json(value))
        )
    }

    fn pretty_json(value: &serde_json::Value) -> String {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    }

    fn pills<'a>(items: impl Iterator<Item = &'a str>) -> String {
        let items = items.collect::<Vec<_>>();
        if items.is_empty() {
            return String::new();
        }
        let mut html = String::from("<div class=\"pill-row\">");
        for item in items {
            html.push_str(&format!("<span class=\"pill\">{}</span>", esc(item)));
        }
        html.push_str("</div>");
        html
    }

    fn set_status(text: &str) {
        if let Ok(element) = element("status") {
            element.set_text_content(Some(text));
        }
    }

    fn set_html(id: &str, html: &str) -> Result<(), JsValue> {
        element(id)?.dyn_into::<HtmlElement>()?.set_inner_html(html);
        Ok(())
    }

    fn element(id: &str) -> Result<Element, JsValue> {
        document()
            .get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("missing element #{id}")))
    }

    fn document() -> Document {
        web_sys::window()
            .expect("missing window")
            .document()
            .expect("missing document")
    }

    fn esc(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}
