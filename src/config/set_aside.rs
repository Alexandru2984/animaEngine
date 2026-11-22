//! What did not read, kept: a character, group or setting an older
//! version cannot read is left out at load and written back at save, for
//! the version that wrote it (`SetAside`).

use super::*;

/// What of `config.toml` this build could not read — written by a newer
/// animaEngine, or mistyped — kept to be written back as it was (1.5).
///
/// One character, group or setting that did not read used to cost the
/// whole file: the app started with the default scene. Every release
/// adds behaviors, settings and choices an older one does not know, so
/// opening a newer version's config in an older one did just that. Now
/// the app goes on without those parts — a character set aside is not
/// shown, a setting is at its default — and saves them back where they
/// were, for the version that wrote them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetAside {
    /// `[global]` keys whose values did not read: the key, the value as
    /// written, and what the setting was at load — `None` when it is not
    /// written out — to tell whether it has been changed here since.
    pub(super) global: Vec<(String, toml::Value, Option<toml::Value>)>,
    /// List entries that did not read: which list, and the entry.
    pub(super) entries: Vec<(SetAsideList, toml::Value)>,
    /// Keys this build does not know inside tables it does — a newer
    /// version's settings, a character's — by where they were. Not an
    /// error: nothing goes missing for them.
    pub(super) unknown: Vec<(Vec<Seg>, toml::Value)>,
}

/// One step to a key set aside: a table's key, or the entry of a list
/// whose `id` is this.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Seg {
    Key(String),
    Id(String),
}

/// The keys a struct reads, as serde names them — asked of its
/// `Deserialize` by a deserializer that only listens for them. `None`
/// for a type that does not read as a plain struct.
pub(super) fn fields_of<T: serde::de::DeserializeOwned>() -> Option<&'static [&'static str]> {
    use serde::de::{Error, Visitor};
    struct Listen<'a>(&'a mut Option<&'static [&'static str]>);
    impl<'de> serde::Deserializer<'de> for Listen<'_> {
        type Error = serde::de::value::Error;
        fn deserialize_any<V: Visitor<'de>>(
            self,
            _: V,
        ) -> std::result::Result<V::Value, Self::Error> {
            Err(Self::Error::custom("only listening"))
        }
        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _: &'static str,
            fields: &'static [&'static str],
            _: V,
        ) -> std::result::Result<V::Value, Self::Error> {
            *self.0 = Some(fields);
            Err(Self::Error::custom("only listening"))
        }
        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
            bytes byte_buf option unit unit_struct newtype_struct seq tuple
            tuple_struct map enum identifier ignored_any
        }
    }
    let mut fields = None;
    let _ = T::deserialize(Listen(&mut fields));
    fields
}

/// The table at `path` under `root`: a key steps into a table — made
/// empty when this version left it out, as it does a table of defaults —
/// and a key followed by an id into the entry of that list whose `id` it
/// is, which is never made: `None` when it is gone.
pub(super) fn table_at<'a>(root: &'a mut toml::Table, path: &[Seg]) -> Option<&'a mut toml::Table> {
    let mut table = root;
    let mut steps = path.iter().peekable();
    while let Some(seg) = steps.next() {
        let Seg::Key(key) = seg else { return None };
        table = match steps.peek() {
            Some(Seg::Id(id)) => {
                steps.next();
                table
                    .get_mut(key)?
                    .as_array_mut()?
                    .iter_mut()
                    .filter_map(toml::Value::as_table_mut)
                    .find(|entry| entry.get("id").and_then(toml::Value::as_str) == Some(id))?
            }
            _ => table
                .entry(key.clone())
                .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                .as_table_mut()?,
        };
    }
    Some(table)
}

/// Where a table's unknown keys are looked for: the keys `T` reads.
pub(super) type Known = fn() -> Option<&'static [&'static str]>;

/// Note the keys of `table`, at `at`, that `known` does not list.
pub(super) fn note_unknown(
    table: &toml::Table,
    at: &[Seg],
    known: Known,
    out: &mut Vec<(Vec<Seg>, toml::Value)>,
) {
    let Some(fields) = known() else { return };
    for (key, value) in table {
        if !fields.contains(&key.as_str()) {
            let mut path = at.to_vec();
            path.push(Seg::Key(key.clone()));
            out.push((path, value.clone()));
        }
    }
}

/// A character's own unknown keys, and those of its nested tables.
pub(super) fn note_character(
    entry: &toml::Value,
    at: &[Seg],
    out: &mut Vec<(Vec<Seg>, toml::Value)>,
) {
    let (Some(table), Some(id)) = (
        entry.as_table(),
        entry.get("id").and_then(toml::Value::as_str),
    ) else {
        return;
    };
    let mut here = at.to_vec();
    here.push(Seg::Id(id.to_string()));
    note_unknown(table, &here, fields_of::<CharacterConfig>, out);
    for (key, known) in [
        ("talk", fields_of::<Talk> as Known),
        ("hours", fields_of::<crate::hours::Hours>),
    ] {
        if let Some(nested) = table.get(key).and_then(toml::Value::as_table) {
            let mut deeper = here.clone();
            deeper.push(Seg::Key(key.to_string()));
            note_unknown(nested, &deeper, known, out);
        }
    }
}

/// Where a [`SetAside`] entry came from.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SetAsideList {
    /// One of [`SET_ASIDE_LISTS`].
    Top(&'static str),
    /// The characters of the window with this id.
    WindowCharacters(String),
}

/// The lists whose entries are read one by one when the whole file does
/// not read.
pub(super) const SET_ASIDE_LISTS: [&str; 4] =
    ["characters", "groups", "reminders", "scene_schedule"];

/// Whether one entry of `list` reads, and if not, why.
pub(super) fn entry_reads(list: &str, entry: &toml::Value) -> std::result::Result<(), String> {
    let entry = entry.clone();
    match list {
        "groups" => entry.try_into::<crate::group::GroupConfig>().map(drop),
        "reminders" => entry
            .try_into::<crate::reminders::ReminderConfig>()
            .map(drop),
        "scene_schedule" => entry.try_into::<crate::schedule::ScheduleRule>().map(drop),
        _ => entry.try_into::<CharacterConfig>().map(drop),
    }
    .map_err(|e| e.to_string().trim().replace('\n', " "))
}

/// A name for an entry in the log: its `name`, else its `id`.
pub(super) fn entry_label(entry: &toml::Value) -> String {
    ["name", "id"]
        .iter()
        .find_map(|k| entry.get(k).and_then(toml::Value::as_str))
        .map(|s| format!("\"{}\"", crate::drop_validate::escape_untrusted(s)))
        .unwrap_or_else(|| "one".to_string())
}

impl SetAside {
    /// Whether anything was set aside — to write back, at least.
    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.entries.is_empty() && self.unknown.is_empty()
    }

    /// Whether something did not read, and is left out until a version
    /// that reads it: what the banner says. Unknown keys are not: kept,
    /// they cost nothing.
    pub fn unread(&self) -> bool {
        !self.global.is_empty() || !self.entries.is_empty()
    }

    /// How many settings, entries and unknown keys are set aside.
    pub(crate) fn len(&self) -> usize {
        self.global.len() + self.entries.len() + self.unknown.len()
    }

    /// What a scene file (`crate::scenes`) holds that this version does
    /// not read: its characters and groups that do not read, taken out of
    /// `table`, and the keys it does not know — the scene's own, its
    /// characters' and groups' — to be written back when it is saved
    /// over, as a config's are.
    pub(crate) fn of_scene(table: &mut toml::Table) -> Self {
        let mut aside = Self::default();
        note_unknown(
            table,
            &[],
            fields_of::<crate::scenes::SavedScene>,
            &mut aside.unknown,
        );
        // Inside the lists a scene reads; any other key — `windows`,
        // `global` — is kept whole, noted above. Its entries noted by `id`
        // as a config's are, two windows with one `id` had the second's
        // keys written into the first.
        aside.note_unknown_in_lists(table);
        // Only the lists a scene reads have entries set aside one by one.
        // Any other — `reminders`, `windows`, from a version whose scenes
        // hold them — is an unknown key and goes back whole: set aside as
        // well, its unreadable entries made the list anew on save, and the
        // whole one was not put back over it.
        aside.entries = Self::take_unreadable(table)
            .entries
            .into_iter()
            .filter(|(list, _)| matches!(list, SetAsideList::Top("characters" | "groups")))
            .collect();
        aside
    }

    /// How many characters, groups and other list entries did not read.
    pub(crate) fn left_out(&self) -> usize {
        self.entries.len()
    }

    /// Note the keys a newer version wrote inside the tables this one
    /// knows, to be written back where they were.
    pub(super) fn note_unknown_keys(&mut self, table: &toml::Table) {
        let out = &mut self.unknown;
        let key = |k: &str| vec![Seg::Key(k.to_string())];
        if let Some(global) = table.get("global").and_then(toml::Value::as_table) {
            note_unknown(global, &key("global"), fields_of::<GlobalConfig>, out);
            for (nested, known) in [
                (
                    "onboarding",
                    fields_of::<crate::ui::onboarding::OnboardingProgress> as Known,
                ),
                ("bedtime", fields_of::<crate::bedtime::Bedtime>),
                ("focus", fields_of::<crate::focus::FocusConfig>),
            ] {
                if let Some(t) = global.get(nested).and_then(toml::Value::as_table) {
                    let at = vec![Seg::Key("global".into()), Seg::Key(nested.into())];
                    note_unknown(t, &at, known, out);
                }
            }
        }
        if let Some(t) = table.get("collapse_state").and_then(toml::Value::as_table) {
            note_unknown(t, &key("collapse_state"), fields_of::<CollapseState>, out);
        }
        self.note_unknown_in_lists(table);
        let out = &mut self.unknown;
        for window in table
            .get("windows")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let (Some(t), Some(id)) = (
                window.as_table(),
                window.get("id").and_then(toml::Value::as_str),
            ) {
                let at = vec![Seg::Key("windows".into()), Seg::Id(id.into())];
                note_unknown(t, &at, fields_of::<WindowConfig>, out);
                let mut chars = at.clone();
                chars.push(Seg::Key("characters".into()));
                for entry in t
                    .get("characters")
                    .and_then(toml::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    note_character(entry, &chars, out);
                }
            }
        }
    }

    /// The unknown keys of the entries of the lists a scene has too —
    /// its characters and groups.
    pub(super) fn note_unknown_in_lists(&mut self, table: &toml::Table) {
        let out = &mut self.unknown;
        let key = |k: &str| vec![Seg::Key(k.to_string())];
        for entry in table
            .get("characters")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            note_character(entry, &key("characters"), out);
        }
        for entry in table
            .get("groups")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let (Some(t), Some(id)) = (
                entry.as_table(),
                entry.get("id").and_then(toml::Value::as_str),
            ) {
                let at = vec![Seg::Key("groups".into()), Seg::Id(id.into())];
                note_unknown(t, &at, fields_of::<crate::group::GroupConfig>, out);
            }
        }
    }

    /// Take out of `table` what does not read: list entries one by one,
    /// and `[global]` keys one by one. What is left is for the decode to
    /// try again.
    pub(super) fn take_unreadable(table: &mut toml::Table) -> Self {
        let mut aside = Self::default();
        let mut sift = |list: SetAsideList, name: &str, items: &mut Vec<toml::Value>| {
            items.retain(|entry| match entry_reads(name, entry) {
                Ok(()) => true,
                Err(e) => {
                    tracing::warn!(
                        "config: {} in `{name}` does not read ({e}); set aside, and kept in the file",
                        entry_label(entry)
                    );
                    aside.entries.push((list.clone(), entry.clone()));
                    false
                }
            });
        };
        for name in SET_ASIDE_LISTS {
            if let Some(toml::Value::Array(items)) = table.get_mut(name) {
                sift(SetAsideList::Top(name), name, items);
            }
        }
        if let Some(toml::Value::Array(windows)) = table.get_mut("windows") {
            for window in windows.iter_mut().filter_map(toml::Value::as_table_mut) {
                let id = window
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                if let Some(toml::Value::Array(items)) = window.get_mut("characters") {
                    sift(SetAsideList::WindowCharacters(id), "characters", items);
                }
            }
        }
        // Every `[global]` setting has a default, so each reads on its
        // own or does not.
        if let Some(toml::Value::Table(global)) = table.get_mut("global") {
            if toml::Value::Table(global.clone())
                .try_into::<GlobalConfig>()
                .is_err()
            {
                let keys: Vec<String> = global.keys().cloned().collect();
                for key in keys {
                    let alone = toml::Table::from_iter([(key.clone(), global[&key].clone())]);
                    if let Err(e) = toml::Value::Table(alone).try_into::<GlobalConfig>() {
                        let e = e.to_string().trim().replace('\n', " ");
                        tracing::warn!(
                            "config: `{key}` in [global] does not read ({e}); at its default, and kept in the file"
                        );
                        if let Some(value) = global.remove(&key) {
                            aside.global.push((key, value, None));
                        }
                    }
                }
            }
        }
        aside
    }

    /// Note what each setting set aside is now, to be compared on save.
    pub(super) fn note_settings(&mut self, global: &GlobalConfig) {
        let now = toml::Value::try_from(global).ok();
        for (key, _, at_load) in &mut self.global {
            *at_load = now.as_ref().and_then(|t| t.get(key.as_str())).cloned();
        }
    }

    /// Put back into `table`, a config or a scene as about to be saved,
    /// what was set aside: entries at the end of their lists — a window's
    /// only if that window is still there — and settings unless changed
    /// here since.
    pub(crate) fn write_back(&self, table: &mut toml::Table) {
        for (list, entry) in &self.entries {
            let items = match list {
                SetAsideList::Top(name) => Some(
                    table
                        .entry(name.to_string())
                        .or_insert_with(|| toml::Value::Array(Vec::new())),
                ),
                SetAsideList::WindowCharacters(id) => table
                    .get_mut("windows")
                    .and_then(toml::Value::as_array_mut)
                    .and_then(|windows| {
                        windows.iter_mut().find(|w| {
                            w.get("id").and_then(toml::Value::as_str) == Some(id.as_str())
                        })
                    })
                    .and_then(toml::Value::as_table_mut)
                    .map(|window| {
                        window
                            .entry("characters")
                            .or_insert_with(|| toml::Value::Array(Vec::new()))
                    }),
            };
            if let Some(toml::Value::Array(items)) = items {
                items.push(entry.clone());
            }
        }
        if let Some(toml::Value::Table(global)) = table.get_mut("global") {
            for (key, written, at_load) in &self.global {
                if global.get(key) == at_load.as_ref() {
                    global.insert(key.clone(), written.clone());
                }
            }
        }
        // Unknown keys back where they were — if that table is still
        // there (a character deleted here takes its own with it), and
        // the key is not there already.
        for (path, value) in &self.unknown {
            let Some((Seg::Key(last), parents)) = path.split_last() else {
                continue;
            };
            if let Some(at) = table_at(table, parents) {
                at.entry(last.clone()).or_insert_with(|| value.clone());
            }
        }
    }
}
